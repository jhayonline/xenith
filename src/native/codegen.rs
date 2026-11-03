use super::{
    check::{StructLayout, TypedProgram},
    error::NativeError,
    syntax::{Expr, Function, InterpolationPart, Span, Stmt, Type},
};
use cranelift_codegen::{
    ir::{
        condcodes::{FloatCC, IntCC},
        types, AbiParam, ArgumentPurpose, Block, InstBuilder, MemFlags, Signature, StackSlotData,
        StackSlotKind, TrapCode, Type as ClifType, Value,
    },
    settings,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{default_libcall_names, DataDescription, FuncId, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};
use std::{
    collections::{HashMap, HashSet},
    fs,
    hash::{Hash, Hasher},
    path::Path,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};

static STRING_ID: AtomicUsize = AtomicUsize::new(0);

pub fn emit_object(program: &TypedProgram, output: &Path) -> Result<(), NativeError> {
    let isa = cranelift_native::builder()
        .map_err(|e| NativeError::not_implemented(e.to_string()))?
        .finish(settings::Flags::new(settings::builder()))
        .map_err(|e| NativeError::not_implemented(e.to_string()))?;
    let mut module = ObjectModule::new(
        ObjectBuilder::new(isa, "xenith", default_libcall_names())
            .map_err(|e| NativeError::not_implemented(e.to_string()))?,
    );
    let mut synthetic = Vec::new();
    let mut closure_captures = HashMap::new();
    for f in &program.0.functions {
        let scope = f
            .params
            .iter()
            .map(|parameter| (parameter.name.clone(), parameter.ty.clone()))
            .collect();
        collect_closures(&f.body, &mut synthetic, &mut closure_captures, &scope);
    }
    let mut all_functions = program.0.functions.clone();
    all_functions.extend(synthetic);
    let mut ids = HashMap::new();
    for f in &all_functions {
        let id = module
            .declare_function(
                &f.name,
                if f.name == "main" {
                    Linkage::Export
                } else {
                    Linkage::Local
                },
                &signature(&module, f, &program.2),
            )
            .map_err(|e| NativeError::not_implemented(e.to_string()))?;
        ids.insert(f.name.clone(), (id, f.result.clone()));
    }
    for f in &all_functions {
        emit(&mut module, f, &ids, &program.2, &closure_captures, &program.0.source)?;
    }
    fs::write(
        output,
        module
            .finish()
            .emit()
            .map_err(|e| NativeError::not_implemented(e.to_string()))?,
    )
    .map_err(NativeError::io)
}

fn closure_name(expression: &Expr) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    format!("{:?}", expression).hash(&mut hasher);
    format!("__closure_{:x}", hasher.finish())
}

fn restore_capture_lanes(
    b: &mut FunctionBuilder,
    lanes: &[Value],
    ty: &Type,
    layouts: &HashMap<String, StructLayout>,
) -> Vec<Value> {
    flat(ty, layouts)
        .into_iter()
        .zip(lanes.iter().copied())
        .map(|(lane, value)| match lane {
            lane if lane == types::I8 => b.ins().ireduce(types::I8, value),
            lane if lane == types::I32 => b.ins().ireduce(types::I32, value),
            lane if lane == types::F64 => b.ins().bitcast(types::F64, MemFlags::new(), value),
            _ => value,
        })
        .collect()
}

fn collect_closures(
    statements: &[Stmt],
    out: &mut Vec<Function>,
    captures: &mut HashMap<String, Vec<(String, Type)>>,
    scope: &HashMap<String, Type>,
) {
    let mut scope = scope.clone();
    for statement in statements {
        match statement {
            Stmt::Let { value, .. } => collect_closure_expr(value, out, captures, &scope),
            Stmt::Expr(value) | Stmt::Return(value) => {
                collect_closure_expr(value, out, captures, &scope)
            }
            Stmt::Assign { value, .. } => collect_closure_expr(value, out, captures, &scope),
            Stmt::IndexAssign { base, index, value } => {
                collect_closure_expr(base, out, captures, &scope);
                collect_closure_expr(index, out, captures, &scope);
                collect_closure_expr(value, out, captures, &scope);
            }
            Stmt::If {
                condition,
                then_body,
                else_body,
            } => {
                collect_closure_expr(condition, out, captures, &scope);
                collect_closures(then_body, out, captures, &scope);
                collect_closures(else_body, out, captures, &scope);
            }
            Stmt::While { condition, body } => {
                collect_closure_expr(condition, out, captures, &scope);
                collect_closures(body, out, captures, &scope);
            }
            Stmt::For {
                init,
                condition,
                step,
                body,
            } => {
                collect_closures(std::slice::from_ref(init), out, captures, &scope);
                collect_closure_expr(condition, out, captures, &scope);
                collect_closures(std::slice::from_ref(step), out, captures, &scope);
                collect_closures(body, out, captures, &scope);
            }
            Stmt::ForIn { iterable, body, .. } => {
                collect_closure_expr(iterable, out, captures, &scope);
                collect_closures(body, out, captures, &scope);
            }
            Stmt::Panic { value, .. } => collect_closure_expr(value, out, captures, &scope),
            Stmt::TryCatch { body, catch_body, .. } => {
                collect_closures(body, out, captures, &scope);
                collect_closures(catch_body, out, captures, &scope);
            }
            Stmt::Stop | Stmt::Skip => {}
        }
        if let Stmt::Let {
            name, ty: Some(ty), ..
        } = statement
        {
            scope.insert(name.clone(), ty.clone());
        }
    }
}

fn collect_closure_expr(
    expression: &Expr,
    out: &mut Vec<Function>,
    captures: &mut HashMap<String, Vec<(String, Type)>>,
    scope: &HashMap<String, Type>,
) {
    match expression {
        Expr::Closure {
            params,
            result,
            body,
        } => {
            let name = closure_name(expression);
            let mut names = HashSet::new();
            collect_names(body, &mut names);
            for parameter in params {
                names.remove(&parameter.name);
            }
            let mut names: Vec<_> = names
                .into_iter()
                .filter_map(|name| scope.get(&name).cloned().map(|ty| (name, ty)))
                .collect();
            names.sort_by(|left, right| left.0.cmp(&right.0));
            captures.insert(name.clone(), names);
            if !out.iter().any(|function: &Function| function.name == name) {
                out.push(Function {
                    name,
                    span: Span::new(0, 0),
                    exported: false,
                    params: params.clone(),
                    result: result.clone(),
                    body: vec![Stmt::Return((**body).clone())],
                });
            }
            collect_closure_expr(body, out, captures, scope);
        }
        Expr::Call { arguments, .. } | Expr::Tuple(arguments) | Expr::List(arguments) => {
            for argument in arguments {
                collect_closure_expr(argument, out, captures, scope);
            }
        }
        Expr::Map(pairs) => {
            for (key, value) in pairs {
                collect_closure_expr(key, out, captures, scope);
                collect_closure_expr(value, out, captures, scope);
            }
        }
        Expr::EnumVariant { arguments, .. } => {
            for argument in arguments {
                collect_closure_expr(argument, out, captures, scope);
            }
        }
        Expr::Match { subject, arms } => {
            collect_closure_expr(subject, out, captures, scope);
            for arm in arms {
                collect_closure_expr(&arm.body, out, captures, scope);
            }
        }
        Expr::StructLiteral { fields, .. } => {
            for (_, value) in fields {
                collect_closure_expr(value, out, captures, scope);
            }
        }
        Expr::Field { base, .. } => collect_closure_expr(base, out, captures, scope),
        Expr::Index { base, index, .. } => {
            collect_closure_expr(base, out, captures, scope);
            collect_closure_expr(index, out, captures, scope);
        }
        Expr::Neg(value) | Expr::Not(value) => collect_closure_expr(value, out, captures, scope),
        Expr::Logical(left, _, right)
        | Expr::Compare(left, _, right)
        | Expr::Binary(left, _, right, _) => {
            collect_closure_expr(left, out, captures, scope);
            collect_closure_expr(right, out, captures, scope);
        }
        Expr::Interpolated(_)
        | Expr::Int(_, _)
        | Expr::Float(_)
        | Expr::Bool(_)
        | Expr::Null
        | Expr::String(_)
        | Expr::Bytes(_)
        | Expr::Name(_) => {}
    }
}

fn collect_names(expression: &Expr, names: &mut HashSet<String>) {
    match expression {
        Expr::Name(name) => {
            names.insert(name.clone());
        }
        Expr::Closure { body, .. } => collect_names(body, names),
        Expr::Call { arguments, .. } | Expr::Tuple(arguments) | Expr::List(arguments) => {
            for argument in arguments {
                collect_names(argument, names);
            }
        }
        Expr::Map(pairs) => {
            for (key, value) in pairs {
                collect_names(key, names);
                collect_names(value, names);
            }
        }
        Expr::EnumVariant { arguments, .. } => {
            for argument in arguments {
                collect_names(argument, names);
            }
        }
        Expr::Match { subject, arms } => {
            collect_names(subject, names);
            for arm in arms {
                collect_names(&arm.body, names);
            }
        }
        Expr::StructLiteral { fields, .. } => {
            for (_, value) in fields {
                collect_names(value, names);
            }
        }
        Expr::Field { base, .. } => collect_names(base, names),
        Expr::Index { base, index, .. } => {
            collect_names(base, names);
            collect_names(index, names);
        }
        Expr::Neg(value) | Expr::Not(value) => collect_names(value, names),
        Expr::Logical(left, _, right)
        | Expr::Compare(left, _, right)
        | Expr::Binary(left, _, right, _) => {
            collect_names(left, names);
            collect_names(right, names);
        }
        _ => {}
    }
}
fn flat(ty: &Type, layouts: &HashMap<String, StructLayout>) -> Vec<cranelift_codegen::ir::Type> {
    match ty {
        Type::I32 => vec![types::I32],
        Type::I64 => vec![types::I64],
        Type::Float => vec![types::F64],
        Type::Bool => vec![types::I8],
        Type::Null => vec![types::I8],
        Type::String => vec![types::I64, types::I64],
        Type::Bytes | Type::List(_) | Type::Map(_, _) => vec![types::I64],
        Type::Tuple(elements) => elements
            .iter()
            .flat_map(|element| flat(element, layouts))
            .collect(),
        Type::Function(_, _) => vec![types::I64, types::I64],
        Type::Named(n) => layouts[n]
            .fields
            .iter()
            .flat_map(|(_, t)| flat(t, layouts))
            .collect(),
    }
}

fn indirect_return(ty: &Type, layouts: &HashMap<String, StructLayout>) -> bool {
    flat(ty, layouts).len() > 2
}

fn return_layout(
    ty: &Type,
    layouts: &HashMap<String, StructLayout>,
) -> (u32, Vec<(ClifType, i32)>) {
    let mut size = 0;
    let mut lanes = Vec::new();
    for lane in flat(ty, layouts) {
        let alignment = lane.bytes().min(8);
        size = (size + alignment - 1) / alignment * alignment;
        lanes.push((lane, size as i32));
        size += lane.bytes();
    }
    (size, lanes)
}

fn signature(m: &ObjectModule, f: &Function, l: &HashMap<String, StructLayout>) -> Signature {
    let mut s = m.make_signature();
    if indirect_return(&f.result, l) {
        s.params.push(AbiParam::special(
            m.target_config().pointer_type(),
            ArgumentPurpose::StructReturn,
        ));
    }
    if f.name != "main" {
        s.params.push(AbiParam::new(types::I64));
    }
    for p in &f.params {
        s.params
            .extend(flat(&p.ty, l).into_iter().map(AbiParam::new));
    }
    if !indirect_return(&f.result, l) {
        s.returns
            .extend(flat(&f.result, l).into_iter().map(AbiParam::new));
    }
    s
}
fn emit(
    m: &mut ObjectModule,
    f: &Function,
    ids: &HashMap<String, (FuncId, Type)>,
    l: &HashMap<String, StructLayout>,
    captures: &HashMap<String, Vec<(String, Type)>>,
    source: &str,
) -> Result<(), NativeError> {
    let mut c = m.make_context();
    c.func.signature = signature(m, f, l);
    let mut bc = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut c.func, &mut bc);
        let block = b.create_block();
        b.append_block_params_for_function_params(block);
        b.switch_to_block(block);
        b.seal_block(block);
        let source_data = m
            .declare_data(
                &format!("xenith_source_{}", STRING_ID.fetch_add(1, Ordering::Relaxed)),
                Linkage::Local,
                true,
                false,
            )
            .map_err(|e| NativeError::not_implemented(e.to_string()))?;
        let mut source_description = DataDescription::new();
        source_description.define(source.as_bytes().to_vec().into_boxed_slice());
        m.define_data(source_data, &source_description)
            .map_err(|e| NativeError::not_implemented(e.to_string()))?;
        let source_global = m.declare_data_in_func(source_data, b.func);
        let source_pointer = b.ins().symbol_value(m.target_config().pointer_type(), source_global);
        let source_length = b.ins().iconst(types::I64, source.len() as i64);
        let _ = runtime_call(
            &mut b,
            m,
            "xenith_error_set_source",
            vec![AbiParam::new(types::I64), AbiParam::new(types::I64)],
            vec![],
            &[source_pointer, source_length],
        );
        let function_data = m
            .declare_data(
                &format!("xenith_function_{}", STRING_ID.fetch_add(1, Ordering::Relaxed)),
                Linkage::Local,
                true,
                false,
            )
            .map_err(|e| NativeError::not_implemented(e.to_string()))?;
        let mut function_description = DataDescription::new();
        function_description.define(f.name.as_bytes().to_vec().into_boxed_slice());
        m.define_data(function_data, &function_description)
            .map_err(|e| NativeError::not_implemented(e.to_string()))?;
        let function_global = m.declare_data_in_func(function_data, b.func);
        let function_pointer = b
            .ins()
            .symbol_value(m.target_config().pointer_type(), function_global);
        let function_length = b.ins().iconst(types::I64, f.name.len() as i64);
        let _ = runtime_call(
            &mut b,
            m,
            "xenith_error_set_function",
            vec![AbiParam::new(types::I64), AbiParam::new(types::I64)],
            vec![],
            &[function_pointer, function_length],
        );
        let function_offset = b.ins().iconst(types::I64, f.span.start as i64);
        let _ = runtime_call(
            &mut b,
            m,
            "xenith_error_set_offset",
            vec![AbiParam::new(types::I64)],
            vec![],
            &[function_offset],
        );
        let incoming = b.block_params(block).to_vec();
        let return_area = indirect_return(&f.result, l).then(|| incoming[0]);
        let mut at = usize::from(return_area.is_some());
        let mut vals = HashMap::new();
        let env = if f.name != "main" {
            let value = incoming[at];
            at += 1;
            if f.name.starts_with("__closure_") {
                Some(value)
            } else {
                None
            }
        } else {
            None
        };
        if let Some(env) = env {
            let mut slot_index = 0;
            for (name, ty) in captures.get(&f.name).into_iter().flatten() {
                let width = flat(ty, l).len();
                let mut lanes = Vec::new();
                for _ in 0..width {
                    let slot = b.ins().iconst(types::I64, slot_index as i64);
                    let raw = runtime_call(
                        &mut b,
                        m,
                        "xenith_closure_get",
                        vec![AbiParam::new(types::I64), AbiParam::new(types::I64)],
                        vec![AbiParam::new(types::I64)],
                        &[env, slot],
                    )[0];
                    lanes.push(raw);
                    slot_index += 1;
                }
                let values = restore_capture_lanes(&mut b, &lanes, ty, l);
                vals.insert(name.clone(), (values, ty.clone()));
            }
        }
        for p in &f.params {
            let n = flat(&p.ty, l).len();
            vals.insert(
                p.name.clone(),
                (incoming[at..at + n].to_vec(), p.ty.clone()),
            );
            at += n;
        }
        let mut loops = Vec::new();
        for s in &f.body {
            match s {
                Stmt::Stop | Stmt::Skip => {
                    unreachable!("loop control is checked before native lowering")
                }
                Stmt::While { condition, body } => {
                    if lower_while(
                        &mut b,
                        condition,
                        body,
                        &mut vals,
                        ids,
                        m,
                        l,
                        return_area,
                        &mut loops,
                        &HashSet::new(),
                        None,
                    ) == LowerFlow::Returns
                    {
                        break;
                    }
                }
                Stmt::For {
                    init,
                    condition,
                    step,
                    body,
                } => {
                    let _ = lower_for(
                        &mut b,
                        init,
                        condition,
                        &mut vals,
                        ids,
                        m,
                        l,
                        return_area,
                        &mut loops,
                        &HashSet::new(),
                        step,
                        body,
                    );
                }
                Stmt::ForIn {
                    names,
                    iterable,
                    body,
                } => {
                    let _ = lower_for_in(
                        &mut b,
                        names,
                        iterable,
                        body,
                        &mut vals,
                        ids,
                        m,
                        l,
                        return_area,
                        &mut loops,
                        &HashSet::new(),
                    );
                }
                Stmt::If {
                    condition,
                    then_body,
                    else_body,
                } => {
                    if lower_if(
                        &mut b,
                        condition,
                        then_body,
                        else_body,
                        &mut vals,
                        ids,
                        m,
                        l,
                        return_area,
                        &mut loops,
                        &HashSet::new(),
                    ) == LowerFlow::Returns
                    {
                        break;
                    }
                }
                Stmt::Let {
                    name, ty, value, ..
                } => {
                    let v = lower_binding(&mut b, value, ty.as_ref(), &vals, ids, m, l);
                    vals.insert(name.clone(), v);
                }
                Stmt::Assign { name, value } => {
                    let v = lower(&mut b, value, &vals, ids, m, l);
                    vals.insert(name.clone(), v);
                }
                Stmt::IndexAssign { base, index, value } => {
                    lower_index_assignment(&mut b, base, index, value, &vals, ids, m, l)
                }
                Stmt::Expr(value) => {
                    let _ = lower(&mut b, value, &vals, ids, m, l);
                }
                Stmt::Panic { value, span } => {
                    emit_panic(&mut b, value, &vals, ids, m, l, *span);
                }
                Stmt::TryCatch { body, binding, catch_body } => {
                    let _ = lower_try_catch(
                        &mut b,
                        body,
                        binding,
                        catch_body,
                        &mut vals,
                        ids,
                        m,
                        l,
                        return_area,
                        &mut loops,
                    );
                }
                Stmt::Return(v) => {
                    emit_return(&mut b, v, &vals, ids, m, l, return_area);
                    break;
                }
            }
        }
        b.finalize();
    }
    m.define_function(ids[&f.name].0, &mut c)
        .map_err(|e| NativeError::not_implemented(format!("{e}\n{}", c.func.display())))?;
    m.clear_context(&mut c);
    Ok(())
}
type Values = HashMap<String, (Vec<Value>, Type)>;

#[derive(Clone)]
struct LoopTarget {
    header: Block,
    continue_to: Block,
    exit: Block,
    names: Vec<String>,
    carried: HashMap<String, Vec<Value>>,
    protected: HashSet<String>,
    has_stop: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LowerFlow {
    FallsThrough,
    Returns,
    LoopJump,
}

fn loop_args(target: &LoopTarget) -> Vec<Value> {
    target
        .names
        .iter()
        .flat_map(|name| target.carried[name].iter().copied())
        .collect()
}

fn record_assignment(loops: &mut [LoopTarget], name: &str, value: &[Value]) {
    for target in loops {
        if !target.protected.contains(name) {
            if let Some(carried) = target.carried.get_mut(name) {
                *carried = value.to_vec();
            }
        }
    }
}

fn sync_loop_values(loops: &mut [LoopTarget], values: &Values, shadowed: &HashSet<String>) {
    for target in loops {
        for name in &target.names {
            if !shadowed.contains(name) && !target.protected.contains(name) {
                if let Some((value, _)) = values.get(name) {
                    target.carried.insert(name.clone(), value.clone());
                }
            }
        }
    }
}

fn emit_return(
    b: &mut FunctionBuilder,
    expression: &Expr,
    values: &Values,
    ids: &HashMap<String, (FuncId, Type)>,
    m: &mut ObjectModule,
    layouts: &HashMap<String, StructLayout>,
    return_area: Option<Value>,
) {
    let (returned, ty) = lower(b, expression, values, ids, m, layouts);
    let _ = runtime_call(b, m, "xenith_error_abort_if_pending", vec![], vec![], &[]);
    if let Some(return_area) = return_area {
        for (value, (_, offset)) in returned
            .into_iter()
            .zip(return_layout(&ty, layouts).1.into_iter())
        {
            b.ins().store(MemFlags::new(), value, return_area, offset);
        }
        b.ins().return_(&[]);
    } else {
        b.ins().return_(&returned);
    }
}

fn assigned_names(statements: &[Stmt], values: &Values) -> Vec<String> {
    assigned_names_in_scope(statements, values, &HashSet::new())
}

fn assigned_names_in_scope(
    statements: &[Stmt],
    values: &Values,
    shadowed: &HashSet<String>,
) -> Vec<String> {
    let mut names = Vec::new();
    let mut shadowed = shadowed.clone();
    for statement in statements {
        match statement {
            Stmt::Let { name, .. } => {
                shadowed.insert(name.clone());
            }
            Stmt::Assign { name, .. }
                if values.contains_key(name)
                    && !shadowed.contains(name)
                    && !names.contains(name) =>
            {
                names.push(name.clone())
            }
            Stmt::If {
                then_body,
                else_body,
                ..
            } => {
                for name in assigned_names_in_scope(then_body, values, &shadowed)
                    .into_iter()
                    .chain(assigned_names_in_scope(else_body, values, &shadowed))
                {
                    if !names.contains(&name) {
                        names.push(name);
                    }
                }
            }
            Stmt::While { body, .. } => {
                for name in assigned_names_in_scope(body, values, &shadowed) {
                    if !names.contains(&name) {
                        names.push(name);
                    }
                }
            }
            Stmt::For {
                init, step, body, ..
            } => {
                let mut for_shadowed = shadowed.clone();
                if let Stmt::Let { name, .. } = init.as_ref() {
                    for_shadowed.insert(name.clone());
                }
                for name in assigned_names_in_scope(body, values, &for_shadowed)
                    .into_iter()
                    .chain(assigned_names_in_scope(
                        std::slice::from_ref(step.as_ref()),
                        values,
                        &for_shadowed,
                    ))
                {
                    if !names.contains(&name) {
                        names.push(name);
                    }
                }
            }
            _ => {}
        }
    }
    names
}

fn emit_panic(
    b: &mut FunctionBuilder,
    value: &Expr,
    values: &Values,
    ids: &HashMap<String, (FuncId, Type)>,
    m: &mut ObjectModule,
    layouts: &HashMap<String, StructLayout>,
    span: Span,
) {
    let location = b.ins().iconst(types::I64, span.start as i64);
    let _ = runtime_call(b, m, "xenith_error_set_offset", vec![AbiParam::new(types::I64)], vec![], &[location]);
    let (message, _) = lower(b, value, values, ids, m, layouts);
    let code = lower(b, &Expr::String("XEN300".into()), values, ids, m, layouts).0;
    let name = lower(b, &Expr::String("Panic".into()), values, ids, m, layouts).0;
    let _ = runtime_call(
        b,
        m,
        "xenith_error_raise",
        vec![
            AbiParam::new(types::I64),
            AbiParam::new(types::I64),
            AbiParam::new(types::I64),
            AbiParam::new(types::I64),
            AbiParam::new(types::I64),
            AbiParam::new(types::I64),
        ],
        vec![],
        &[message[0], message[1], code[0], code[1], name[0], name[1]],
    );
}

fn lower_try_catch(
    b: &mut FunctionBuilder,
    body: &[Stmt],
    binding: &str,
    catch_body: &[Stmt],
    values: &mut Values,
    ids: &HashMap<String, (FuncId, Type)>,
    m: &mut ObjectModule,
    layouts: &HashMap<String, StructLayout>,
    return_area: Option<Value>,
    loops: &mut Vec<LoopTarget>,
) -> LowerFlow {
    if let Some(Stmt::Return(expression)) = body.last() {
        let prefix = &body[..body.len() - 1];
        let prefix_flow = lower_branch(
            b,
            prefix,
            values,
            ids,
            m,
            layouts,
            return_area,
            loops,
            &mut HashSet::new(),
        );
        if prefix_flow != LowerFlow::FallsThrough {
            return prefix_flow;
        }
        let (returned, return_type) = lower(b, expression, values, ids, m, layouts);
        let catch_block = b.create_block();
        let success_block = b.create_block();
        let pending = runtime_call(
            b,
            m,
            "xenith_error_pending",
            vec![],
            vec![AbiParam::new(types::I8)],
            &[],
        )[0];
        b.ins().brif(pending, catch_block, &[], success_block, &[]);
        b.switch_to_block(success_block);
        b.seal_block(success_block);
        if let Some(area) = return_area {
            for (value, (_, offset)) in returned
                .into_iter()
                .zip(return_layout(&return_type, layouts).1.into_iter())
            {
                b.ins().store(MemFlags::new(), value, area, offset);
            }
            b.ins().return_(&[]);
        } else {
            b.ins().return_(&returned);
        }
        b.switch_to_block(catch_block);
        b.seal_block(catch_block);
        let mut catch_values = values.clone();
        let mut error_lanes = Vec::new();
        for field in [
            ("xenith_error_message", "xenith_error_message_len"),
            ("xenith_error_code", "xenith_error_code_len"),
            ("xenith_error_name", "xenith_error_name_len"),
            ("xenith_error_hint", "xenith_error_hint_len"),
            ("xenith_error_function", "xenith_error_function_len"),
        ] {
            error_lanes.push(runtime_call(b, m, field.0, vec![], vec![AbiParam::new(types::I64)], &[])[0]);
            error_lanes.push(runtime_call(b, m, field.1, vec![], vec![AbiParam::new(types::I64)], &[])[0]);
        }
        error_lanes.push(runtime_call(b, m, "xenith_error_offset_value", vec![], vec![AbiParam::new(types::I64)], &[])[0]);
        error_lanes.push(runtime_call(b, m, "xenith_error_line_value", vec![], vec![AbiParam::new(types::I64)], &[])[0]);
        error_lanes.push(runtime_call(b, m, "xenith_error_column_value", vec![], vec![AbiParam::new(types::I64)], &[])[0]);
        catch_values.insert(binding.to_string(), (error_lanes, Type::Named("Error".into())));
        let _ = runtime_call(b, m, "xenith_error_clear", vec![], vec![], &[]);
        return lower_branch(
            b,
            catch_body,
            &mut catch_values,
            ids,
            m,
            layouts,
            return_area,
            loops,
            &mut HashSet::new(),
        );
    }
    let body_flow = lower_branch(
        b,
        body,
        values,
        ids,
        m,
        layouts,
        return_area,
        loops,
        &mut HashSet::new(),
    );
    if body_flow != LowerFlow::FallsThrough {
        return body_flow;
    }

    let catch_block = b.create_block();
    let merge_block = b.create_block();
    let pending = runtime_call(
        b,
        m,
        "xenith_error_pending",
        vec![],
        vec![AbiParam::new(types::I8)],
        &[],
    )[0];
    b.ins().brif(pending, catch_block, &[], merge_block, &[]);

    b.switch_to_block(catch_block);
    b.seal_block(catch_block);
    let mut catch_values = values.clone();
    let mut error_lanes = Vec::new();
    for field in [
        ("xenith_error_message", "xenith_error_message_len"),
        ("xenith_error_code", "xenith_error_code_len"),
        ("xenith_error_name", "xenith_error_name_len"),
        ("xenith_error_hint", "xenith_error_hint_len"),
        ("xenith_error_function", "xenith_error_function_len"),
    ] {
        error_lanes.push(
            runtime_call(
                b,
                m,
                field.0,
                vec![],
                vec![AbiParam::new(types::I64)],
                &[],
            )[0],
        );
        error_lanes.push(
            runtime_call(
                b,
                m,
                field.1,
                vec![],
                vec![AbiParam::new(types::I64)],
                &[],
            )[0],
        );
    }
    error_lanes.push(runtime_call(b, m, "xenith_error_offset_value", vec![], vec![AbiParam::new(types::I64)], &[])[0]);
    error_lanes.push(runtime_call(b, m, "xenith_error_line_value", vec![], vec![AbiParam::new(types::I64)], &[])[0]);
    error_lanes.push(runtime_call(b, m, "xenith_error_column_value", vec![], vec![AbiParam::new(types::I64)], &[])[0]);
    catch_values.insert(binding.to_string(), (error_lanes, Type::Named("Error".into())));
    let _ = runtime_call(b, m, "xenith_error_clear", vec![], vec![], &[]);
    let catch_flow = lower_branch(
        b,
        catch_body,
        &mut catch_values,
        ids,
        m,
        layouts,
        return_area,
        loops,
        &mut HashSet::new(),
    );
    if catch_flow == LowerFlow::FallsThrough {
        let _ = runtime_call(b, m, "xenith_error_clear", vec![], vec![], &[]);
        b.ins().jump(merge_block, &[]);
    }

    b.switch_to_block(merge_block);
    b.seal_block(merge_block);
    LowerFlow::FallsThrough
}

fn lower_branch(
    b: &mut FunctionBuilder,
    statements: &[Stmt],
    values: &mut Values,
    ids: &HashMap<String, (FuncId, Type)>,
    m: &mut ObjectModule,
    layouts: &HashMap<String, StructLayout>,
    return_area: Option<Value>,
    loops: &mut Vec<LoopTarget>,
    shadowed: &mut HashSet<String>,
) -> LowerFlow {
    for statement in statements {
        match statement {
            Stmt::Stop => {
                let target = loops.last_mut().expect("checked loop control has a target");
                target.has_stop = true;
                let args = loop_args(target);
                b.ins().jump(target.exit, &args);
                return LowerFlow::LoopJump;
            }
            Stmt::Skip => {
                let target = loops.last().expect("checked loop control has a target");
                let args = loop_args(target);
                b.ins().jump(target.continue_to, &args);
                return LowerFlow::LoopJump;
            }
            Stmt::Let {
                name, ty, value, ..
            } => {
                let value = lower_binding(b, value, ty.as_ref(), values, ids, m, layouts);
                values.insert(name.clone(), value);
                shadowed.insert(name.clone());
                for target in &mut *loops {
                    target.protected.insert(name.clone());
                }
            }
            Stmt::Assign { name, value } => {
                let value = lower(b, value, values, ids, m, layouts);
                if !shadowed.contains(name) {
                    record_assignment(loops, name, &value.0);
                }
                values.insert(name.clone(), value);
            }
            Stmt::IndexAssign { base, index, value } => {
                lower_index_assignment(b, base, index, value, values, ids, m, layouts)
            }
            Stmt::Expr(value) => {
                let _ = lower(b, value, values, ids, m, layouts);
            }
            Stmt::Panic { value, span } => {
                emit_panic(b, value, values, ids, m, layouts, *span);
            }
            Stmt::TryCatch { body, binding, catch_body } => {
                let flow = lower_try_catch(
                    b,
                    body,
                    binding,
                    catch_body,
                    values,
                    ids,
                    m,
                    layouts,
                    return_area,
                    loops,
                );
                if flow != LowerFlow::FallsThrough {
                    return flow;
                }
            }
            Stmt::If {
                condition,
                then_body,
                else_body,
            } => {
                let flow = lower_if(
                    b,
                    condition,
                    then_body,
                    else_body,
                    values,
                    ids,
                    m,
                    layouts,
                    return_area,
                    loops,
                    shadowed,
                );
                if flow != LowerFlow::FallsThrough {
                    return flow;
                }
            }
            Stmt::ForIn {
                names,
                iterable,
                body,
            } => {
                let flow = lower_for_in(
                    b,
                    names,
                    iterable,
                    body,
                    values,
                    ids,
                    m,
                    layouts,
                    return_area,
                    loops,
                    shadowed,
                );
                if flow != LowerFlow::FallsThrough {
                    return flow;
                }
            }
            Stmt::For {
                init,
                condition,
                step,
                body,
            } => {
                let flow = lower_for(
                    b,
                    init,
                    condition,
                    values,
                    ids,
                    m,
                    layouts,
                    return_area,
                    loops,
                    shadowed,
                    step,
                    body,
                );
                if flow != LowerFlow::FallsThrough {
                    return flow;
                }
            }
            Stmt::Return(value) => {
                emit_return(b, value, values, ids, m, layouts, return_area);
                return LowerFlow::Returns;
            }
            Stmt::While { condition, body } => {
                let flow = lower_while(
                    b,
                    condition,
                    body,
                    values,
                    ids,
                    m,
                    layouts,
                    return_area,
                    loops,
                    shadowed,
                    None,
                );
                if flow != LowerFlow::FallsThrough {
                    return flow;
                }
            }
        }
    }
    LowerFlow::FallsThrough
}

fn lower_for_in(
    b: &mut FunctionBuilder,
    names: &[String],
    iterable: &Expr,
    body: &[Stmt],
    values: &mut Values,
    ids: &HashMap<String, (FuncId, Type)>,
    m: &mut ObjectModule,
    layouts: &HashMap<String, StructLayout>,
    return_area: Option<Value>,
    loops: &mut Vec<LoopTarget>,
    shadowed: &HashSet<String>,
) -> LowerFlow {
    if names.len() != 1 {
        return LowerFlow::FallsThrough;
    }
    let (iter_values, iterable_type) = lower(b, iterable, values, ids, m, layouts);
    let (binding_type, binding_expr) = match iterable_type.clone() {
        Type::List(element) => (
            *element,
            Expr::Index {
                base: Box::new(Expr::Name("__xenith_iter".into())),
                index: Box::new(Expr::Name("__xenith_index".into())),
                span: Span::new(0, 0),
            },
        ),
        Type::Map(_, _) => (
            Type::String,
            Expr::Call {
                name: "__xenith_map_key".into(),
                arguments: vec![
                    Expr::Name("__xenith_iter".into()),
                    Expr::Name("__xenith_index".into()),
                ],
            },
        ),
        _ => return LowerFlow::FallsThrough,
    };
    let iter_name = "__xenith_iter".to_string();
    let index_name = "__xenith_index".to_string();
    let binding = names[0].clone();
    let original_names: Vec<String> = values.keys().cloned().collect();
    values.insert(iter_name.clone(), (iter_values, iterable_type));
    values.insert(
        index_name.clone(),
        (vec![b.ins().iconst(types::I32, 0)], Type::I32),
    );
    let condition = Expr::Compare(
        Box::new(Expr::Name(index_name.clone())),
        "<".into(),
        Box::new(Expr::Call {
            name: "len".into(),
            arguments: vec![Expr::Name(iter_name.clone())],
        }),
    );
    let step = Stmt::Assign {
        name: index_name.clone(),
        value: Expr::Binary(
            Box::new(Expr::Name(index_name.clone())),
            '+',
            Box::new(Expr::Int(1, Type::I32)),
            Span::new(0, 0),
        ),
    };
    let mut lowered_body = vec![Stmt::Let {
        name: binding,
        mutable: false,
        ty: Some(binding_type),
        value: binding_expr,
    }];
    lowered_body.extend_from_slice(body);
    let flow = lower_while(
        b,
        &condition,
        &lowered_body,
        values,
        ids,
        m,
        layouts,
        return_area,
        loops,
        shadowed,
        Some(&step),
    );
    values.remove(&iter_name);
    values.remove(&index_name);
    for name in original_names {
        if let Some(value) = values.get(&name).cloned() {
            values.insert(name, value);
        }
    }
    flow
}

fn lower_for(
    b: &mut FunctionBuilder,
    init: &Stmt,
    condition: &Expr,
    values: &mut Values,
    ids: &HashMap<String, (FuncId, Type)>,
    m: &mut ObjectModule,
    layouts: &HashMap<String, StructLayout>,
    return_area: Option<Value>,
    loops: &mut Vec<LoopTarget>,
    inherited_shadows: &HashSet<String>,
    step: &Stmt,
    body: &[Stmt],
) -> LowerFlow {
    // The initializer's binding lives for the condition, step, and body only.
    // Keep a separate environment so it cannot leak out of the `for`, while
    // copying updates to pre-existing, non-shadowed bindings back afterwards.
    let outer_names: Vec<String> = values.keys().cloned().collect();
    let mut for_values = values.clone();
    let init_name = match init {
        Stmt::Let { name, value, .. } => {
            let value = lower(b, value, &for_values, ids, m, layouts);
            for_values.insert(name.clone(), value);
            name.clone()
        }
        _ => unreachable!("for initializers are checked before native lowering"),
    };
    let mut for_shadows = inherited_shadows.clone();
    if values.contains_key(&init_name) {
        for_shadows.insert(init_name.clone());
    }
    let flow = lower_while(
        b,
        condition,
        body,
        &mut for_values,
        ids,
        m,
        layouts,
        return_area,
        loops,
        &for_shadows,
        Some(step),
    );
    for name in outer_names {
        if name != init_name {
            values.insert(name.clone(), for_values[&name].clone());
        }
    }
    flow
}

fn lower_while(
    b: &mut FunctionBuilder,
    condition: &Expr,
    body: &[Stmt],
    values: &mut Values,
    ids: &HashMap<String, (FuncId, Type)>,
    m: &mut ObjectModule,
    layouts: &HashMap<String, StructLayout>,
    return_area: Option<Value>,
    loops: &mut Vec<LoopTarget>,
    inherited_shadows: &HashSet<String>,
    step: Option<&Stmt>,
) -> LowerFlow {
    let always_true = matches!(condition, Expr::Bool(true));
    // A classic `for` loop's update expression is part of its back edge.  Its
    // assignments must therefore be carried through the loop even when the
    // body only reads the variable (and especially when `skip` bypasses the
    // rest of the body).
    let mut carried_statements = body.to_vec();
    if let Some(step) = step {
        carried_statements.push(step.clone());
    }
    let names = assigned_names(&carried_statements, values);
    let header = b.create_block();
    let loop_body = b.create_block();
    let step_block = b.create_block();
    let exit = b.create_block();
    for name in &names {
        for ty in flat(&values[name].1, layouts) {
            b.append_block_param(header, ty);
            b.append_block_param(exit, ty);
            b.append_block_param(step_block, ty);
        }
    }
    let initial: Vec<Value> = names
        .iter()
        .flat_map(|name| values[name].0.iter().copied())
        .collect();
    b.ins().jump(header, &initial);
    b.switch_to_block(header);
    let mut loop_values = values.clone();
    let header_values = b.block_params(header).to_vec();
    let mut offset = 0;
    for name in &names {
        let value_count = values[name].0.len();
        loop_values.insert(
            name.clone(),
            (
                header_values[offset..offset + value_count].to_vec(),
                values[name].1.clone(),
            ),
        );
        offset += value_count;
    }
    let condition = lower(b, condition, &loop_values, ids, m, layouts).0[0];
    b.ins()
        .brif(condition, loop_body, &[], exit, &header_values);
    b.switch_to_block(loop_body);
    b.seal_block(loop_body);
    loops.push(LoopTarget {
        header,
        continue_to: if step.is_some() { step_block } else { header },
        exit,
        names: names.clone(),
        carried: names
            .iter()
            .map(|name| (name.clone(), loop_values[name].0.clone()))
            .collect(),
        protected: inherited_shadows
            .iter()
            .filter(|name| !names.contains(name))
            .cloned()
            .collect(),
        has_stop: false,
    });
    let mut body_shadows = inherited_shadows.clone();
    for name in &names {
        body_shadows.remove(name);
    }
    let body_flow = lower_branch(
        b,
        body,
        &mut loop_values,
        ids,
        m,
        layouts,
        return_area,
        loops,
        &mut body_shadows,
    );
    if body_flow == LowerFlow::FallsThrough {
        let next: Vec<Value> = names
            .iter()
            .flat_map(|name| loop_values[name].0.iter().copied())
            .collect();
        b.ins()
            .jump(if step.is_some() { step_block } else { header }, &next);
    }
    if let Some(step) = step {
        b.switch_to_block(step_block);
        b.seal_block(step_block);
        let step_values = b.block_params(step_block).to_vec();
        let mut offset = 0;
        for name in &names {
            let value_count = loop_values[name].0.len();
            loop_values.insert(
                name.clone(),
                (
                    step_values[offset..offset + value_count].to_vec(),
                    loop_values[name].1.clone(),
                ),
            );
            offset += value_count;
        }
        let step_flow = lower_branch(
            b,
            std::slice::from_ref(step),
            &mut loop_values,
            ids,
            m,
            layouts,
            return_area,
            loops,
            &mut body_shadows,
        );
        if step_flow == LowerFlow::FallsThrough {
            let next: Vec<Value> = names
                .iter()
                .flat_map(|name| loop_values[name].0.iter().copied())
                .collect();
            b.ins().jump(header, &next);
        }
    }
    let loop_target = loops.pop().expect("loop target was pushed");
    b.seal_block(header);
    b.switch_to_block(exit);
    b.seal_block(exit);
    if always_true && body_flow == LowerFlow::Returns && !loop_target.has_stop {
        b.ins().trap(TrapCode::unwrap_user(1));
    }
    let exit_values = b.block_params(exit).to_vec();
    let mut offset = 0;
    for name in &names {
        let value_count = values[name].0.len();
        values.insert(
            name.clone(),
            (
                exit_values[offset..offset + value_count].to_vec(),
                values[name].1.clone(),
            ),
        );
        offset += value_count;
    }
    sync_loop_values(loops, values, inherited_shadows);
    if always_true && body_flow == LowerFlow::Returns && !loop_target.has_stop {
        LowerFlow::Returns
    } else {
        LowerFlow::FallsThrough
    }
}

fn lower_if(
    b: &mut FunctionBuilder,
    condition: &Expr,
    then_body: &[Stmt],
    else_body: &[Stmt],
    values: &mut Values,
    ids: &HashMap<String, (FuncId, Type)>,
    m: &mut ObjectModule,
    layouts: &HashMap<String, StructLayout>,
    return_area: Option<Value>,
    loops: &mut Vec<LoopTarget>,
    inherited_shadows: &HashSet<String>,
) -> LowerFlow {
    let condition = lower(b, condition, values, ids, m, layouts).0[0];
    let yes = b.create_block();
    let no = b.create_block();
    let merge = b.create_block();
    let names = assigned_names(then_body, values)
        .into_iter()
        .chain(assigned_names(else_body, values))
        .fold(Vec::new(), |mut names, name| {
            if !names.contains(&name) {
                names.push(name);
            }
            names
        });
    for name in &names {
        for ty in flat(&values[name].1, layouts) {
            b.append_block_param(merge, ty);
        }
    }
    b.ins().brif(condition, yes, &[], no, &[]);
    let mut branch_flows = Vec::new();
    for (block, branch) in [(yes, then_body), (no, else_body)] {
        b.switch_to_block(block);
        b.seal_block(block);
        let mut branch_values = values.clone();
        let mut branch_loops = loops.clone();
        let mut branch_shadows = inherited_shadows.clone();
        let flow = lower_branch(
            b,
            branch,
            &mut branch_values,
            ids,
            m,
            layouts,
            return_area,
            &mut branch_loops,
            &mut branch_shadows,
        );
        for (target, branch_target) in loops.iter_mut().zip(branch_loops) {
            target.has_stop |= branch_target.has_stop;
        }
        branch_flows.push(flow);
        if flow == LowerFlow::FallsThrough {
            let args: Vec<Value> = names
                .iter()
                .flat_map(|name| branch_values[name].0.iter().copied())
                .collect();
            b.ins().jump(merge, &args);
        }
    }
    b.seal_block(merge);
    b.switch_to_block(merge);
    if branch_flows
        .iter()
        .all(|flow| *flow != LowerFlow::FallsThrough)
    {
        b.ins().trap(TrapCode::unwrap_user(1));
        if branch_flows.iter().all(|flow| *flow == LowerFlow::Returns) {
            return LowerFlow::Returns;
        }
        return LowerFlow::LoopJump;
    }
    let merge_values = b.block_params(merge).to_vec();
    let mut offset = 0;
    for name in &names {
        let value_count = values[name].0.len();
        values.insert(
            name.clone(),
            (
                merge_values[offset..offset + value_count].to_vec(),
                values[name].1.clone(),
            ),
        );
        offset += value_count;
    }
    sync_loop_values(loops, values, inherited_shadows);
    LowerFlow::FallsThrough
}
fn lower(
    b: &mut FunctionBuilder,
    x: &Expr,
    v: &HashMap<String, (Vec<Value>, Type)>,
    ids: &HashMap<String, (FuncId, Type)>,
    m: &mut ObjectModule,
    l: &HashMap<String, StructLayout>,
) -> (Vec<Value>, Type) {
    match x {
        Expr::Int(n, t) => (vec![b.ins().iconst(flat(t, l)[0], *n)], t.clone()),
        Expr::Float(value) => (vec![b.ins().f64const(*value)], Type::Float),
        Expr::Bool(value) => (
            vec![b.ins().iconst(types::I8, i64::from(*value))],
            Type::Bool,
        ),
        Expr::Null => (vec![b.ins().iconst(types::I8, 0)], Type::Null),
        Expr::EnumVariant {
            enum_name,
            variant,
            arguments,
        } => {
            let layout = &l[enum_name];
            let tag = layout
                .variants
                .as_ref()
                .and_then(|variants| variants.iter().position(|(name, _)| name == variant))
                .expect("checked enum variant") as i64;
            let mut values = vec![b.ins().iconst(types::I32, tag)];
            for argument in arguments {
                values.extend(lower(b, argument, v, ids, m, l).0);
            }
            let expected = flat(&Type::Named(enum_name.clone()), l);
            for lane in expected.into_iter().skip(values.len()) {
                values.push(if lane == types::F64 {
                    b.ins().f64const(0.0)
                } else {
                    b.ins().iconst(lane, 0)
                });
            }
            (values, Type::Named(enum_name.clone()))
        }
        Expr::Match { subject, arms } => {
            let (subject_values, Type::Named(enum_name)) = lower(b, subject, v, ids, m, l) else {
                unreachable!()
            };
            let merge = b.create_block();
            let mut arm_blocks = Vec::new();
            let result_type = infer_lowered_type(&arms[0].body, v, ids, l);
            for ty in flat(&result_type, l) {
                b.append_block_param(merge, ty);
            }
            for _ in arms {
                arm_blocks.push(b.create_block());
            }
            let test_blocks: Vec<_> = (0..arms.len()).map(|_| b.create_block()).collect();
            b.ins().jump(test_blocks[0], &[]);
            for (index, arm) in arms.iter().enumerate() {
                let current_test = test_blocks[index];
                b.switch_to_block(current_test);
                let target = arm_blocks[index];
                if let Some(variant) = &arm.variant {
                    let tag = l[&enum_name]
                        .variants
                        .as_ref()
                        .unwrap()
                        .iter()
                        .position(|(name, _)| name == variant)
                        .unwrap() as i64;
                    if index + 1 == arms.len() {
                        b.ins().jump(target, &[]);
                    } else {
                        let matches = b.ins().icmp_imm(IntCC::Equal, subject_values[0], tag);
                        let next = test_blocks[index + 1];
                        b.ins().brif(matches, target, &[], next, &[]);
                    }
                } else {
                    b.ins().jump(target, &[]);
                }
                b.seal_block(current_test);
            }
            for (index, arm) in arms.iter().enumerate() {
                b.switch_to_block(arm_blocks[index]);
                let mut arm_values = v.clone();
                if let Some(binding) = &arm.binding {
                    arm_values.insert(
                        binding.clone(),
                        (
                            subject_values[1..].to_vec(),
                            infer_lowered_type(&arm.body, v, ids, l),
                        ),
                    );
                }
                let body = lower(b, &arm.body, &arm_values, ids, m, l).0;
                b.ins().jump(merge, &body);
                b.seal_block(arm_blocks[index]);
            }
            b.seal_block(merge);
            b.switch_to_block(merge);
            (b.block_params(merge).to_vec(), result_type)
        }
        Expr::Tuple(elements) => {
            let mut values = Vec::new();
            let mut types = Vec::new();
            for element in elements {
                let (element_values, element_type) = lower(b, element, v, ids, m, l);
                values.extend(element_values);
                types.push(element_type);
            }
            (values, Type::Tuple(types))
        }
        Expr::Interpolated(parts) => {
            let mut current = lower(b, &Expr::String(String::new()), v, ids, m, l).0;
            for part in parts {
                let next = match part {
                    InterpolationPart::Text(text) => {
                        lower(b, &Expr::String(text.clone()), v, ids, m, l).0
                    }
                    InterpolationPart::Name(name) => {
                        lower(b, &Expr::Name(name.clone()), v, ids, m, l).0
                    }
                };
                let ptr_slot = b.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    8,
                    3,
                ));
                let len_slot = b.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    8,
                    3,
                ));
                let ptr_out = b.ins().stack_addr(types::I64, ptr_slot, 0);
                let len_out = b.ins().stack_addr(types::I64, len_slot, 0);
                runtime_call(
                    b,
                    m,
                    "xenith_string_concat",
                    vec![AbiParam::new(types::I64); 6],
                    vec![],
                    &[current[0], current[1], next[0], next[1], ptr_out, len_out],
                );
                current = vec![
                    b.ins().load(types::I64, MemFlags::new(), ptr_out, 0),
                    b.ins().load(types::I64, MemFlags::new(), len_out, 0),
                ];
            }
            (current, Type::String)
        }
        Expr::String(value) => {
            let id = m
                .declare_data(
                    &format!(
                        "xenith_string_{}",
                        STRING_ID.fetch_add(1, Ordering::Relaxed)
                    ),
                    Linkage::Local,
                    true,
                    false,
                )
                .map_err(|e| NativeError::not_implemented(e.to_string()))
                .expect("string declaration");
            let mut data = DataDescription::new();
            data.define(value.as_bytes().to_vec().into_boxed_slice());
            m.define_data(id, &data)
                .map_err(|e| NativeError::not_implemented(e.to_string()))
                .expect("string definition");
            let global = m.declare_data_in_func(id, b.func);
            let pointer = b
                .ins()
                .symbol_value(m.target_config().pointer_type(), global);
            let length = b.ins().iconst(types::I64, value.len() as i64);
            (vec![pointer, length], Type::String)
        }
        Expr::Bytes(value) => {
            let (data, _) = lower(b, &Expr::String(value.clone()), v, ids, m, l);
            let raw = runtime_call(
                b,
                m,
                "xenith_bytes_new",
                vec![
                    AbiParam::new(m.target_config().pointer_type()),
                    AbiParam::new(types::I64),
                ],
                vec![AbiParam::new(types::I64)],
                &data,
            );
            (raw, Type::Bytes)
        }
        Expr::List(elements) => {
            let element_type = match elements.first() {
                Some(element) => infer_lowered_type(element, v, ids, l),
                None => unreachable!("checked"),
            };
            let (width, lanes) = return_layout(&element_type, l);
            let length = b.ins().iconst(types::I64, elements.len() as i64);
            let width_value = b.ins().iconst(types::I64, width as i64);
            let raw = runtime_call(
                b,
                m,
                "xenith_list_new",
                vec![AbiParam::new(types::I64), AbiParam::new(types::I64)],
                vec![AbiParam::new(types::I64)],
                &[length, width_value],
            )[0];
            for (offset, element) in elements.iter().enumerate() {
                let offset_value = b.ins().iconst(types::I64, offset as i64);
                let pointer = runtime_call(
                    b,
                    m,
                    "xenith_list_at",
                    vec![AbiParam::new(types::I64), AbiParam::new(types::I64)],
                    vec![AbiParam::new(types::I64)],
                    &[raw, offset_value],
                )[0];
                for (value, (_, field_offset)) in lower(b, element, v, ids, m, l)
                    .0
                    .into_iter()
                    .zip(lanes.iter())
                {
                    b.ins()
                        .store(MemFlags::new(), value, pointer, *field_offset);
                }
            }
            (vec![raw], Type::List(Box::new(element_type)))
        }
        Expr::Map(pairs) => {
            let (key_type, value_type) = match pairs.first() {
                Some((key, value)) => (
                    infer_lowered_type(key, v, ids, l),
                    infer_lowered_type(value, v, ids, l),
                ),
                None => unreachable!("checked"),
            };
            let (width, lanes) = return_layout(&value_type, l);
            let length = b.ins().iconst(types::I64, pairs.len() as i64);
            let width_value = b.ins().iconst(types::I64, width as i64);
            let raw = runtime_call(
                b,
                m,
                "xenith_map_new",
                vec![AbiParam::new(types::I64), AbiParam::new(types::I64)],
                vec![AbiParam::new(types::I64)],
                &[length, width_value],
            )[0];
            let mut entries = pairs.clone();
            entries.sort_by_key(|(key, _)| match key {
                Expr::String(value) => value.clone(),
                _ => String::new(),
            });
            for (slot, (key, value)) in entries.iter().enumerate() {
                let key = lower(b, key, v, ids, m, l).0;
                let slot = b.ins().iconst(types::I64, slot as i64);
                let buffer = b.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    width,
                    3,
                ));
                let pointer = b
                    .ins()
                    .stack_addr(m.target_config().pointer_type(), buffer, 0);
                for (item, (_, field_offset)) in lower(b, value, v, ids, m, l)
                    .0
                    .into_iter()
                    .zip(lanes.iter())
                {
                    b.ins().store(MemFlags::new(), item, pointer, *field_offset);
                }
                let _ = runtime_call(
                    b,
                    m,
                    "xenith_map_put",
                    vec![
                        AbiParam::new(types::I64),
                        AbiParam::new(types::I64),
                        AbiParam::new(types::I64),
                        AbiParam::new(types::I64),
                        AbiParam::new(types::I64),
                    ],
                    vec![],
                    &[raw, slot, key[0], key[1], pointer],
                );
            }
            (
                vec![raw],
                Type::Map(Box::new(key_type), Box::new(value_type)),
            )
        }
        Expr::Neg(value) => {
            let (values, ty) = lower(b, value, v, ids, m, l);
            if ty == Type::Float {
                return (vec![b.ins().fneg(values[0])], ty);
            }
            let zero = b.ins().iconst(flat(&ty, l)[0], 0);
            let values = b.ins().ssub_overflow(zero, values[0]);
            (vec![checked_integer_operation(b, m, values, &ty)], ty)
        }
        Expr::Not(value) => {
            let value = lower(b, value, v, ids, m, l).0[0];
            let zero = b.ins().iconst(types::I8, 0);
            (vec![b.ins().icmp(IntCC::Equal, value, zero)], Type::Bool)
        }
        Expr::Logical(left, operator, right) => {
            let left = lower(b, left, v, ids, m, l).0[0];
            let evaluate_right = b.create_block();
            let short_circuit = b.create_block();
            let merge = b.create_block();
            b.append_block_param(merge, types::I8);
            if operator == "&&" {
                b.ins().brif(left, evaluate_right, &[], short_circuit, &[]);
            } else {
                b.ins().brif(left, short_circuit, &[], evaluate_right, &[]);
            }
            b.switch_to_block(evaluate_right);
            b.seal_block(evaluate_right);
            let right = lower(b, right, v, ids, m, l).0[0];
            b.ins().jump(merge, &[right]);
            b.switch_to_block(short_circuit);
            b.seal_block(short_circuit);
            let value = b.ins().iconst(types::I8, i64::from(operator == "||"));
            b.ins().jump(merge, &[value]);
            b.seal_block(merge);
            b.switch_to_block(merge);
            (vec![b.block_params(merge)[0]], Type::Bool)
        }
        Expr::Name(n) => {
            if let Some(value) = v.get(n) {
                return value.clone();
            }
            let callee = m.declare_func_in_func(ids[n].0, b.func);
            let env = b.ins().iconst(types::I64, 0);
            (
                vec![b.ins().func_addr(types::I64, callee), env],
                Type::Function(vec![], Box::new(ids[n].1.clone())),
            )
        }
        Expr::Closure {
            params,
            result,
            body,
        } => {
            let name = closure_name(x);
            let callee = m.declare_func_in_func(ids[&name].0, b.func);
            let mut capture_names = HashSet::new();
            collect_names(body, &mut capture_names);
            for parameter in params {
                capture_names.remove(&parameter.name);
            }
            let mut capture_names: Vec<_> = capture_names
                .into_iter()
                .filter(|name| v.contains_key(name))
                .collect();
            capture_names.sort();
            let capture_count: usize = capture_names
                .iter()
                .map(|name| flat(&v[name].1, l).len())
                .sum();
            let capture_count = b.ins().iconst(types::I64, capture_count as i64);
            let env = runtime_call(
                b,
                m,
                "xenith_closure_new",
                vec![AbiParam::new(types::I64)],
                vec![AbiParam::new(types::I64)],
                &[capture_count],
            )[0];
            let mut slot_index = 0;
            for capture in capture_names.iter() {
                for (lane_type, value) in flat(&v[capture].1, l).into_iter().zip(&v[capture].0) {
                    let stored = match lane_type {
                        lane if lane == types::I32 => b.ins().sextend(types::I64, *value),
                        lane if lane == types::I8 => b.ins().uextend(types::I64, *value),
                        lane if lane == types::F64 => {
                            b.ins().bitcast(types::I64, MemFlags::new(), *value)
                        }
                        _ => *value,
                    };
                    let slot = b.ins().iconst(types::I64, slot_index);
                    let _ = runtime_call(
                        b,
                        m,
                        "xenith_closure_set",
                        vec![
                            AbiParam::new(types::I64),
                            AbiParam::new(types::I64),
                            AbiParam::new(types::I64),
                        ],
                        vec![],
                        &[env, slot, stored],
                    );
                    slot_index += 1;
                }
            }
            (
                vec![b.ins().func_addr(types::I64, callee), env],
                Type::Function(
                    params
                        .iter()
                        .map(|parameter| parameter.ty.clone())
                        .collect(),
                    Box::new(result.clone()),
                ),
            )
        }
        Expr::StructLiteral { name, fields } => {
            let mut out = vec![];
            for (field, _) in &l[name].fields {
                out.extend(
                    lower(
                        b,
                        &fields.iter().find(|(n, _)| n == field).unwrap().1,
                        v,
                        ids,
                        m,
                        l,
                    )
                    .0,
                );
            }
            (out, Type::Named(name.clone()))
        }
        Expr::Field { base, name } => {
            let (a, t) = lower(b, base, v, ids, m, l);
            let Type::Named(s) = t else { unreachable!() };
            let mut offset = 0;
            for (field, ty) in &l[&s].fields {
                let width = flat(ty, l).len();
                if field == name {
                    return (a[offset..offset + width].to_vec(), ty.clone());
                }
                offset += width;
            }
            unreachable!()
        }
        Expr::Index { base, index, span } => {
            let location = b.ins().iconst(types::I64, span.start as i64);
            let _ = runtime_call(b, m, "xenith_error_set_offset", vec![AbiParam::new(types::I64)], vec![], &[location]);
            let (raw, collection_type) = lower(b, base, v, ids, m, l);
            let index_expr = index.as_ref();
            let index = lower(b, index_expr, v, ids, m, l).0;
            let (element_type, pointer) = match collection_type {
                Type::List(element) => {
                    let offset = b.ins().sextend(types::I64, index[0]);
                    let pointer = runtime_call(
                        b,
                        m,
                        "xenith_list_at",
                        vec![AbiParam::new(types::I64), AbiParam::new(types::I64)],
                        vec![AbiParam::new(types::I64)],
                        &[raw[0], offset],
                    )[0];
                    (*element, pointer)
                }
                Type::Bytes => {
                    let offset = b.ins().sextend(types::I64, index[0]);
                    let pointer = runtime_call(
                        b,
                        m,
                        "xenith_list_at",
                        vec![AbiParam::new(types::I64), AbiParam::new(types::I64)],
                        vec![AbiParam::new(types::I64)],
                        &[raw[0], offset],
                    )[0];
                    (Type::Bytes, pointer)
                }
                Type::Map(_, value) => {
                    let pointer = runtime_call(
                        b,
                        m,
                        "xenith_map_at",
                        vec![
                            AbiParam::new(types::I64),
                            AbiParam::new(types::I64),
                            AbiParam::new(types::I64),
                        ],
                        vec![AbiParam::new(types::I64)],
                        &[raw[0], index[0], index[1]],
                    )[0];
                    (*value, pointer)
                }
                Type::Tuple(elements) => {
                    let Expr::Int(position, _) = index_expr else {
                        unreachable!()
                    };
                    let position = usize::try_from(*position).unwrap_or(0);
                    let mut offset = 0;
                    let mut selected = None;
                    for (current, element) in elements.iter().enumerate() {
                        let width = flat(element, l).len();
                        if current == position {
                            selected = Some((element.clone(), offset));
                        }
                        offset += width;
                    }
                    let (element, offset) = selected.expect("checked tuple index");
                    return (
                        raw[offset..offset + flat(&element, l).len()].to_vec(),
                        element,
                    );
                }
                _ => unreachable!(),
            };
            if element_type == Type::Bytes {
                let byte = b.ins().load(types::I8, MemFlags::new(), pointer, 0);
                return (vec![b.ins().uextend(types::I32, byte)], Type::I32);
            }
            let (_, lanes) = return_layout(&element_type, l);
            let values = lanes
                .into_iter()
                .map(|(ty, offset)| b.ins().load(ty, MemFlags::new(), pointer, offset))
                .collect();
            (values, element_type)
        }
        Expr::Call { name, arguments } => {
            if name == "__xenith_map_key" {
                let (map, _) = lower(b, &arguments[0], v, ids, m, l);
                let index_value = lower(b, &arguments[1], v, ids, m, l).0[0];
                let index = b.ins().sextend(types::I64, index_value);
                let pointer = runtime_call(
                    b,
                    m,
                    "xenith_map_key_at",
                    vec![AbiParam::new(types::I64), AbiParam::new(types::I64)],
                    vec![AbiParam::new(types::I64)],
                    &[map[0], index],
                )[0];
                let length = runtime_call(
                    b,
                    m,
                    "xenith_map_key_len_at",
                    vec![AbiParam::new(types::I64), AbiParam::new(types::I64)],
                    vec![AbiParam::new(types::I64)],
                    &[map[0], index],
                )[0];
                return (vec![pointer, length], Type::String);
            }
            if let Some((_, Type::Function(parameters, result))) = v.get(name) {
                let mut args = Vec::new();
                args.push(v[name].0[1]);
                for argument in arguments {
                    args.extend(lower(b, argument, v, ids, m, l).0);
                }
                let mut signature = m.make_signature();
                signature.params.push(AbiParam::new(types::I64));
                for parameter in parameters {
                    signature
                        .params
                        .extend(flat(parameter, l).into_iter().map(AbiParam::new));
                }
                signature
                    .returns
                    .extend(flat(result, l).into_iter().map(AbiParam::new));
                let sig_ref = b.func.dfg.signatures.push(signature);
                let call = b.ins().call_indirect(sig_ref, v[name].0[0], &args);
                return (b.inst_results(call).to_vec(), (**result).clone());
            }
            if name == "echo" {
                let argument = lower(b, &arguments[0], v, ids, m, l).0;
                let mut signature = m.make_signature();
                signature
                    .params
                    .push(AbiParam::new(m.target_config().pointer_type()));
                signature.params.push(AbiParam::new(types::I64));
                let echo = m
                    .declare_function("xenith_echo", Linkage::Import, &signature)
                    .map_err(|e| NativeError::not_implemented(e.to_string()))
                    .expect("echo declaration");
                let callee = m.declare_func_in_func(echo, b.func);
                b.ins().call(callee, &argument);
                return (vec![b.ins().iconst(types::I32, 0)], Type::I32);
            }
            if name == "len" {
                let (argument, argument_type) = lower(b, &arguments[0], v, ids, m, l);
                return match argument_type {
                    Type::String => (vec![b.ins().ireduce(types::I32, argument[1])], Type::I32),
                    Type::Bytes | Type::List(_) => (
                        runtime_call(
                            b,
                            m,
                            "xenith_list_len",
                            vec![AbiParam::new(types::I64)],
                            vec![AbiParam::new(types::I32)],
                            &[argument[0]],
                        ),
                        Type::I32,
                    ),
                    Type::Map(_, _) => (
                        runtime_call(
                            b,
                            m,
                            "xenith_map_len",
                            vec![AbiParam::new(types::I64)],
                            vec![AbiParam::new(types::I32)],
                            &[argument[0]],
                        ),
                        Type::I32,
                    ),
                    _ => unreachable!(),
                };
            }
            if name == "append" {
                let (list, Type::List(element)) = lower(b, &arguments[0], v, ids, m, l) else {
                    unreachable!()
                };
                let value = lower(b, &arguments[1], v, ids, m, l).0;
                let (width, lanes) = return_layout(&element, l);
                let slot = b.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    width,
                    3,
                ));
                let pointer = b
                    .ins()
                    .stack_addr(m.target_config().pointer_type(), slot, 0);
                for (item, (_, offset)) in value.into_iter().zip(lanes) {
                    b.ins().store(MemFlags::new(), item, pointer, offset);
                }
                let width_value = b.ins().iconst(types::I64, width as i64);
                let result = runtime_call(
                    b,
                    m,
                    "xenith_list_append",
                    vec![
                        AbiParam::new(types::I64),
                        AbiParam::new(types::I64),
                        AbiParam::new(types::I64),
                    ],
                    vec![AbiParam::new(types::I64)],
                    &[list[0], width_value, pointer],
                );
                return (result, Type::List(element));
            }
            if name == "remove" {
                let (map, _) = lower(b, &arguments[0], v, ids, m, l);
                let key = lower(b, &arguments[1], v, ids, m, l).0;
                let result = runtime_call(
                    b,
                    m,
                    "xenith_map_remove",
                    vec![
                        AbiParam::new(types::I64),
                        AbiParam::new(types::I64),
                        AbiParam::new(types::I64),
                    ],
                    vec![AbiParam::new(types::I32)],
                    &[map[0], key[0], key[1]],
                );
                return (result, Type::I32);
            }
            if name == "contains" {
                let (map, _) = lower(b, &arguments[0], v, ids, m, l);
                let key = lower(b, &arguments[1], v, ids, m, l).0;
                let result = runtime_call(
                    b,
                    m,
                    "xenith_map_contains",
                    vec![
                        AbiParam::new(types::I64),
                        AbiParam::new(types::I64),
                        AbiParam::new(types::I64),
                    ],
                    vec![AbiParam::new(types::I8)],
                    &[map[0], key[0], key[1]],
                );
                return (result, Type::Bool);
            }
            let (callee_name, receiver) = if ids.contains_key(name) {
                (name.as_str(), None)
            } else if let Some((receiver, method)) = name.split_once("__") {
                (method, Some(receiver))
            } else {
                (name.as_str(), None)
            };
            let mut a = vec![];
            if let Some(receiver) = receiver {
                a.extend(lower(b, &Expr::Name(receiver.into()), v, ids, m, l).0);
            }
            for x in arguments {
                a.extend(lower(b, x, v, ids, m, l).0)
            }
            if callee_name != "main" {
                a.insert(0, b.ins().iconst(types::I64, 0));
            }
            let result_type = &ids[callee_name].1;
            let indirect = indirect_return(result_type, l);
            let return_buffer = if indirect {
                let (size, _) = return_layout(result_type, l);
                let slot = b.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    size,
                    3,
                ));
                let pointer = b
                    .ins()
                    .stack_addr(m.target_config().pointer_type(), slot, 0);
                a.insert(0, pointer);
                Some(pointer)
            } else {
                None
            };
            let callee = m.declare_func_in_func(ids[callee_name].0, b.func);
            let call = b.ins().call(callee, &a);
            let values = if let Some(return_buffer) = return_buffer {
                return_layout(result_type, l)
                    .1
                    .into_iter()
                    .map(|(ty, offset)| b.ins().load(ty, MemFlags::new(), return_buffer, offset))
                    .collect()
            } else {
                b.inst_results(call).to_vec()
            };
            (values, result_type.clone())
        }
        Expr::Compare(a, operator, c) => {
            let (left, ty) = lower(b, a, v, ids, m, l);
            let right = lower(b, c, v, ids, m, l).0;
            if matches!(ty, Type::Named(_)) {
                let mut fields = left.into_iter().zip(right);
                let (first_left, first_right) = fields.next().unwrap();
                let mut equal = b.ins().icmp(IntCC::Equal, first_left, first_right);
                for (left, right) in fields {
                    let field_equal = b.ins().icmp(IntCC::Equal, left, right);
                    equal = b.ins().band(equal, field_equal);
                }
                let value = if operator == "!=" {
                    let zero = b.ins().iconst(types::I8, 0);
                    b.ins().icmp(IntCC::Equal, equal, zero)
                } else {
                    equal
                };
                return (vec![value], Type::Bool);
            }
            if ty == Type::String {
                let equal = runtime_call(
                    b,
                    m,
                    "xenith_string_equal",
                    vec![AbiParam::new(types::I64); 4],
                    vec![AbiParam::new(types::I8)],
                    &[left[0], left[1], right[0], right[1]],
                )[0];
                let value = if operator == "!=" {
                    let zero = b.ins().iconst(types::I8, 0);
                    b.ins().icmp(IntCC::Equal, equal, zero)
                } else {
                    equal
                };
                return (vec![value], Type::Bool);
            }
            let x = left[0];
            let y = right[0];
            if ty == Type::Float {
                let cc = match operator.as_str() {
                    "<" => FloatCC::LessThan,
                    ">" => FloatCC::GreaterThan,
                    "<=" => FloatCC::LessThanOrEqual,
                    ">=" => FloatCC::GreaterThanOrEqual,
                    "==" => FloatCC::Equal,
                    "!=" => FloatCC::NotEqual,
                    _ => FloatCC::Equal,
                };
                return (vec![b.ins().fcmp(cc, x, y)], Type::Bool);
            }
            let cc = match operator.as_str() {
                "<" => IntCC::SignedLessThan,
                ">" => IntCC::SignedGreaterThan,
                "<=" => IntCC::SignedLessThanOrEqual,
                ">=" => IntCC::SignedGreaterThanOrEqual,
                "==" => IntCC::Equal,
                "!=" => IntCC::NotEqual,
                _ => IntCC::SignedLessThan,
            };
            (vec![b.ins().icmp(cc, x, y)], Type::Bool)
        }
        Expr::Binary(a, o, c, span) => {
            let (left, ty) = lower(b, a, v, ids, m, l);
            let x = left[0];
            let y = lower(b, c, v, ids, m, l).0[0];
            let location = b.ins().iconst(types::I64, span.start as i64);
            let _ = runtime_call(b, m, "xenith_error_set_offset", vec![AbiParam::new(types::I64)], vec![], &[location]);
            (
                vec![match o {
                    '+' if ty == Type::Float => b.ins().fadd(x, y),
                    '-' if ty == Type::Float => b.ins().fsub(x, y),
                    '*' if ty == Type::Float => b.ins().fmul(x, y),
                    '/' if ty == Type::Float => b.ins().fdiv(x, y),
                    '+' => {
                        let values = b.ins().sadd_overflow(x, y);
                        checked_integer_operation(b, m, values, &ty)
                    }
                    '-' => {
                        let values = b.ins().ssub_overflow(x, y);
                        checked_integer_operation(b, m, values, &ty)
                    }
                    '*' => {
                        let values = b.ins().smul_overflow(x, y);
                        checked_integer_operation(b, m, values, &ty)
                    }
                    _ => {
                        let left = if ty == Type::I32 {
                            b.ins().sextend(types::I64, x)
                        } else {
                            x
                        };
                        let right = if ty == Type::I32 {
                            b.ins().sextend(types::I64, y)
                        } else {
                            y
                        };
                        let quotient = runtime_call(
                            b,
                            m,
                            "xenith_int_divide",
                            vec![AbiParam::new(types::I64), AbiParam::new(types::I64)],
                            vec![AbiParam::new(types::I64)],
                            &[left, right],
                        )[0];
                        if ty == Type::I32 {
                            b.ins().ireduce(types::I32, quotient)
                        } else {
                            quotient
                        }
                    }
                }],
                ty,
            )
        }
    }
}

fn infer_lowered_type(
    expression: &Expr,
    values: &Values,
    ids: &HashMap<String, (FuncId, Type)>,
    layouts: &HashMap<String, StructLayout>,
) -> Type {
    match expression {
        Expr::Int(_, ty) => ty.clone(),
        Expr::Null => Type::Null,
        Expr::Float(_) => Type::Float,
        Expr::Tuple(items) => Type::Tuple(
            items
                .iter()
                .map(|item| infer_lowered_type(item, values, ids, layouts))
                .collect(),
        ),
        Expr::Bool(_) => Type::Bool,
        Expr::String(_) => Type::String,
        Expr::Interpolated(_) => Type::String,
        Expr::Bytes(_) => Type::Bytes,
        Expr::Closure { params, result, .. } => Type::Function(
            params
                .iter()
                .map(|parameter| parameter.ty.clone())
                .collect(),
            Box::new(result.clone()),
        ),
        Expr::EnumVariant { enum_name, .. } => Type::Named(enum_name.clone()),
        Expr::Match { arms, .. } => infer_lowered_type(&arms[0].body, values, ids, layouts),
        Expr::Name(name) => values[name].1.clone(),
        Expr::List(items) => Type::List(Box::new(infer_lowered_type(
            &items[0], values, ids, layouts,
        ))),
        Expr::Map(items) => Type::Map(
            Box::new(infer_lowered_type(&items[0].0, values, ids, layouts)),
            Box::new(infer_lowered_type(&items[0].1, values, ids, layouts)),
        ),
        Expr::Index { base, .. } => match infer_lowered_type(base, values, ids, layouts) {
            Type::List(element) => *element,
            Type::Map(_, value) => *value,
            Type::Bytes => Type::I32,
            _ => unreachable!(),
        },
        Expr::Call { name, .. } => ids[name].1.clone(),
        Expr::StructLiteral { name, .. } => Type::Named(name.clone()),
        Expr::Field { base, name } => {
            let Type::Named(structure) = infer_lowered_type(base, values, ids, layouts) else {
                unreachable!()
            };
            layouts[&structure]
                .fields
                .iter()
                .find(|(field, _)| field == name)
                .unwrap()
                .1
                .clone()
        }
        Expr::Neg(value) | Expr::Not(value) => infer_lowered_type(value, values, ids, layouts),
        Expr::Logical(_, _, _) | Expr::Compare(_, _, _) => Type::Bool,
        Expr::Binary(left, _, _, _) => infer_lowered_type(left, values, ids, layouts),
    }
}

fn runtime_call(
    b: &mut FunctionBuilder,
    m: &mut ObjectModule,
    name: &str,
    params: Vec<AbiParam>,
    returns: Vec<AbiParam>,
    arguments: &[Value],
) -> Vec<Value> {
    let mut signature = m.make_signature();
    signature.params = params;
    signature.returns = returns;
    let function = m
        .declare_function(name, Linkage::Import, &signature)
        .expect("runtime declaration");
    let callee = m.declare_func_in_func(function, b.func);
    let call = b.ins().call(callee, arguments);
    b.inst_results(call).to_vec()
}

fn lower_index_assignment(
    b: &mut FunctionBuilder,
    base: &Expr,
    index: &Expr,
    value: &Expr,
    values: &Values,
    ids: &HashMap<String, (FuncId, Type)>,
    m: &mut ObjectModule,
    layouts: &HashMap<String, StructLayout>,
) {
    let (raw, collection_type) = lower(b, base, values, ids, m, layouts);
    let index = lower(b, index, values, ids, m, layouts).0;
    let (element_type, pointer) = match collection_type {
        Type::List(element) => {
            let offset = b.ins().sextend(types::I64, index[0]);
            (
                *element,
                runtime_call(
                    b,
                    m,
                    "xenith_list_at",
                    vec![AbiParam::new(types::I64), AbiParam::new(types::I64)],
                    vec![AbiParam::new(types::I64)],
                    &[raw[0], offset],
                )[0],
            )
        }
        Type::Bytes => {
            let offset = b.ins().sextend(types::I64, index[0]);
            (
                Type::Bytes,
                runtime_call(
                    b,
                    m,
                    "xenith_list_at",
                    vec![AbiParam::new(types::I64), AbiParam::new(types::I64)],
                    vec![AbiParam::new(types::I64)],
                    &[raw[0], offset],
                )[0],
            )
        }
        Type::Map(_, value_type) => (
            *value_type,
            runtime_call(
                b,
                m,
                "xenith_map_at",
                vec![
                    AbiParam::new(types::I64),
                    AbiParam::new(types::I64),
                    AbiParam::new(types::I64),
                ],
                vec![AbiParam::new(types::I64)],
                &[raw[0], index[0], index[1]],
            )[0],
        ),
        _ => unreachable!(),
    };
    if element_type == Type::Bytes {
        let byte = lower(b, value, values, ids, m, layouts).0[0];
        let narrowed = b.ins().ireduce(types::I8, byte);
        b.ins().store(MemFlags::new(), narrowed, pointer, 0);
        return;
    }
    for (item, (_, offset)) in lower(b, value, values, ids, m, layouts)
        .0
        .into_iter()
        .zip(return_layout(&element_type, layouts).1)
    {
        b.ins().store(MemFlags::new(), item, pointer, offset);
    }
}

fn lower_binding(
    b: &mut FunctionBuilder,
    value: &Expr,
    annotation: Option<&Type>,
    values: &Values,
    ids: &HashMap<String, (FuncId, Type)>,
    m: &mut ObjectModule,
    layouts: &HashMap<String, StructLayout>,
) -> (Vec<Value>, Type) {
    match (annotation, value) {
        (Some(Type::Function(_, _)), Expr::Name(name)) if ids.contains_key(name) => {
            let callee = m.declare_func_in_func(ids[name].0, b.func);
            let env = b.ins().iconst(types::I64, 0);
            (
                vec![b.ins().func_addr(types::I64, callee), env],
                annotation.cloned().unwrap(),
            )
        }
        (Some(Type::List(element)), Expr::List(elements)) if elements.is_empty() => {
            let zero = b.ins().iconst(types::I64, 0);
            let width = b
                .ins()
                .iconst(types::I64, return_layout(element, layouts).0 as i64);
            (
                runtime_call(
                    b,
                    m,
                    "xenith_list_new",
                    vec![AbiParam::new(types::I64), AbiParam::new(types::I64)],
                    vec![AbiParam::new(types::I64)],
                    &[zero, width],
                ),
                Type::List(element.clone()),
            )
        }
        (Some(Type::Map(key, value_type)), Expr::Map(pairs)) if pairs.is_empty() => {
            let zero = b.ins().iconst(types::I64, 0);
            let width = b
                .ins()
                .iconst(types::I64, return_layout(value_type, layouts).0 as i64);
            (
                runtime_call(
                    b,
                    m,
                    "xenith_map_new",
                    vec![AbiParam::new(types::I64), AbiParam::new(types::I64)],
                    vec![AbiParam::new(types::I64)],
                    &[zero, width],
                ),
                Type::Map(key.clone(), value_type.clone()),
            )
        }
        _ => lower(b, value, values, ids, m, layouts),
    }
}

fn checked_integer_operation(
    b: &mut FunctionBuilder,
    m: &mut ObjectModule,
    (value, overflow): (Value, Value),
    ty: &Type,
) -> Value {
    let widened = if *ty == Type::I32 {
        b.ins().sextend(types::I64, value)
    } else {
        value
    };
    let checked = runtime_call(
        b,
        m,
        "xenith_overflow_value",
        vec![AbiParam::new(types::I64), AbiParam::new(types::I8)],
        vec![AbiParam::new(types::I64)],
        &[widened, overflow],
    )[0];
    if *ty == Type::I32 {
        b.ins().ireduce(types::I32, checked)
    } else {
        checked
    }
}

pub fn link_host(object: &Path, executable: &Path) -> Result<(), NativeError> {
    let runtime =
        super::runtime::write_source(object.parent().unwrap()).map_err(NativeError::io)?;
    let out = Command::new("cc")
        .arg(object)
        .arg(runtime)
        .arg("-o")
        .arg(executable)
        .output()
        .map_err(NativeError::io)?;
    if out.status.success() {
        Ok(())
    } else {
        Err(NativeError::not_implemented(
            String::from_utf8_lossy(&out.stderr).to_string(),
        ))
    }
}
