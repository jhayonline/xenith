use super::{
    check::{StructLayout, TypedProgram},
    error::NativeError,
    syntax::{Expr, Function, Stmt, Type},
};
use cranelift_codegen::{
    ir::{
        condcodes::IntCC, types, AbiParam, ArgumentPurpose, Block, InstBuilder, MemFlags,
        Signature, StackSlotData, StackSlotKind, TrapCode, Type as ClifType, Value,
    },
    settings,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{default_libcall_names, FuncId, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
    process::Command,
};

pub fn emit_object(program: &TypedProgram, output: &Path) -> Result<(), NativeError> {
    let isa = cranelift_native::builder()
        .map_err(|e| NativeError::not_implemented(e.to_string()))?
        .finish(settings::Flags::new(settings::builder()))
        .map_err(|e| NativeError::not_implemented(e.to_string()))?;
    let mut module = ObjectModule::new(
        ObjectBuilder::new(isa, "xenith", default_libcall_names())
            .map_err(|e| NativeError::not_implemented(e.to_string()))?,
    );
    let mut ids = HashMap::new();
    for f in &program.0.functions {
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
    for f in &program.0.functions {
        emit(&mut module, f, &ids, &program.2)?;
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
fn flat(ty: &Type, layouts: &HashMap<String, StructLayout>) -> Vec<cranelift_codegen::ir::Type> {
    match ty {
        Type::I32 => vec![types::I32],
        Type::I64 => vec![types::I64],
        Type::Bool => vec![types::I8],
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
        let incoming = b.block_params(block).to_vec();
        let return_area = indirect_return(&f.result, l).then(|| incoming[0]);
        let mut at = usize::from(return_area.is_some());
        let mut vals = HashMap::new();
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
                    ) == LowerFlow::Returns
                    {
                        break;
                    }
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
                Stmt::Let { name, value, .. } | Stmt::Assign { name, value } => {
                    let v = lower(&mut b, value, &vals, ids, m, l);
                    vals.insert(name.clone(), v);
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
        .map_err(|e| NativeError::not_implemented(e.to_string()))?;
    m.clear_context(&mut c);
    Ok(())
}
type Values = HashMap<String, (Vec<Value>, Type)>;

#[derive(Clone)]
struct LoopTarget {
    header: Block,
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
            _ => {}
        }
    }
    names
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
                b.ins().jump(target.header, &args);
                return LowerFlow::LoopJump;
            }
            Stmt::Let { name, value, .. } => {
                let value = lower(b, value, values, ids, m, layouts);
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
                );
                if flow != LowerFlow::FallsThrough {
                    return flow;
                }
            }
        }
    }
    LowerFlow::FallsThrough
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
) -> LowerFlow {
    let always_true = matches!(condition, Expr::Bool(true));
    let names = assigned_names(body, values);
    let header = b.create_block();
    let loop_body = b.create_block();
    let exit = b.create_block();
    for name in &names {
        for ty in flat(&values[name].1, layouts) {
            b.append_block_param(header, ty);
            b.append_block_param(exit, ty);
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
    let loop_target = loops.pop().expect("loop target was pushed");
    if body_flow == LowerFlow::FallsThrough {
        let next: Vec<Value> = names
            .iter()
            .flat_map(|name| loop_values[name].0.iter().copied())
            .collect();
        b.ins().jump(header, &next);
    }
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
        Expr::Bool(value) => (
            vec![b.ins().iconst(types::I8, i64::from(*value))],
            Type::Bool,
        ),
        Expr::Neg(value) => {
            let (values, ty) = lower(b, value, v, ids, m, l);
            let zero = b.ins().iconst(flat(&ty, l)[0], 0);
            let values = b.ins().ssub_overflow(zero, values[0]);
            (vec![checked_integer_operation(b, values)], ty)
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
        Expr::Name(n) => v[n].clone(),
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
        Expr::Call { name, arguments } => {
            let mut a = vec![];
            for x in arguments {
                a.extend(lower(b, x, v, ids, m, l).0)
            }
            let result_type = &ids[name].1;
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
            let callee = m.declare_func_in_func(ids[name].0, b.func);
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
            let x = left[0];
            let y = right[0];
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
        Expr::Binary(a, o, c) => {
            let (left, ty) = lower(b, a, v, ids, m, l);
            let x = left[0];
            let y = lower(b, c, v, ids, m, l).0[0];
            (
                vec![match o {
                    '+' => {
                        let values = b.ins().sadd_overflow(x, y);
                        checked_integer_operation(b, values)
                    }
                    '-' => {
                        let values = b.ins().ssub_overflow(x, y);
                        checked_integer_operation(b, values)
                    }
                    '*' => {
                        let values = b.ins().smul_overflow(x, y);
                        checked_integer_operation(b, values)
                    }
                    _ => b.ins().sdiv(x, y),
                }],
                ty,
            )
        }
    }
}

fn checked_integer_operation(b: &mut FunctionBuilder, (value, overflow): (Value, Value)) -> Value {
    b.ins().trapnz(overflow, TrapCode::INTEGER_OVERFLOW);
    value
}

pub fn link_host(object: &Path, executable: &Path) -> Result<(), NativeError> {
    let out = Command::new("cc")
        .arg(object)
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
