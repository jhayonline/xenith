use super::{
    check::{StructLayout, TypedProgram},
    error::NativeError,
    syntax::{Expr, Function, Stmt, Type},
};
use cranelift_codegen::{
    ir::{condcodes::IntCC, types, AbiParam, InstBuilder, Signature, Value},
    settings,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{default_libcall_names, FuncId, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};
use std::{collections::HashMap, fs, path::Path, process::Command};

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
fn signature(m: &ObjectModule, f: &Function, l: &HashMap<String, StructLayout>) -> Signature {
    let mut s = m.make_signature();
    for p in &f.params {
        s.params
            .extend(flat(&p.ty, l).into_iter().map(AbiParam::new));
    }
    s.returns
        .extend(flat(&f.result, l).into_iter().map(AbiParam::new));
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
        let mut at = 0;
        let mut vals = HashMap::new();
        for p in &f.params {
            let n = flat(&p.ty, l).len();
            vals.insert(
                p.name.clone(),
                (incoming[at..at + n].to_vec(), p.ty.clone()),
            );
            at += n;
        }
        for s in &f.body {
            match s {
                Stmt::If {
                    condition,
                    then_body,
                    else_body,
                } => {
                    let condition = lower(&mut b, condition, &vals, ids, m, l).0[0];
                    let yes = b.create_block();
                    let no = b.create_block();
                    b.ins().brif(condition, yes, &[], no, &[]);
                    for (block, branch) in [(yes, then_body), (no, else_body)] {
                        b.switch_to_block(block);
                        b.seal_block(block);
                        for statement in branch {
                            if let Stmt::Return(value) = statement {
                                let (value, _) = lower(&mut b, value, &vals, ids, m, l);
                                b.ins().return_(&value);
                            }
                        }
                    }
                }
                Stmt::Let { name, value, .. } | Stmt::Assign { name, value } => {
                    let v = lower(&mut b, value, &vals, ids, m, l);
                    vals.insert(name.clone(), v);
                }
                Stmt::Return(v) => {
                    let (v, _) = lower(&mut b, v, &vals, ids, m, l);
                    b.ins().return_(&v);
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
            let i = l[&s].fields.iter().position(|(n, _)| n == name).unwrap();
            (vec![a[i]], l[&s].fields[i].1.clone())
        }
        Expr::Call { name, arguments } => {
            let mut a = vec![];
            for x in arguments {
                a.extend(lower(b, x, v, ids, m, l).0)
            }
            let callee = m.declare_func_in_func(ids[name].0, b.func);
            let call = b.ins().call(callee, &a);
            (b.inst_results(call).to_vec(), ids[name].1.clone())
        }
        Expr::Compare(a, operator, c) => {
            let x = lower(b, a, v, ids, m, l).0[0];
            let y = lower(b, c, v, ids, m, l).0[0];
            let cc = match operator.as_str() {
                "<" => IntCC::SignedLessThan,
                ">" => IntCC::SignedGreaterThan,
                "==" => IntCC::Equal,
                "!=" => IntCC::NotEqual,
                _ => IntCC::SignedLessThan,
            };
            (vec![b.ins().icmp(cc, x, y)], Type::Bool)
        }
        Expr::Binary(a, o, c) => {
            let x = lower(b, a, v, ids, m, l).0[0];
            let y = lower(b, c, v, ids, m, l).0[0];
            (
                vec![match o {
                    '+' => b.ins().iadd(x, y),
                    '-' => b.ins().isub(x, y),
                    '*' => b.ins().imul(x, y),
                    _ => b.ins().sdiv(x, y),
                }],
                Type::I32,
            )
        }
    }
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
