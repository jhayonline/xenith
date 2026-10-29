use std::{collections::HashMap, fs, path::Path, process::Command};
use cranelift_codegen::{ir::{types, AbiParam, InstBuilder}, settings};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{default_libcall_names, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};
use super::{check::TypedProgram, error::NativeError, syntax::{Expr, Stmt}};

pub fn emit_object(program: &TypedProgram, output: &Path) -> Result<(), NativeError> {
    let flags = settings::Flags::new(settings::builder());
    let isa = cranelift_native::builder().map_err(|e| NativeError::not_implemented(e.to_string()))?.finish(flags).map_err(|e| NativeError::not_implemented(e.to_string()))?;
    let builder = ObjectBuilder::new(isa, "xenith", default_libcall_names()).map_err(|e| NativeError::not_implemented(e.to_string()))?;
    let mut module = ObjectModule::new(builder);
    let mut context = module.make_context(); context.func.signature.returns.push(AbiParam::new(types::I32));
    let id = module.declare_function("main", Linkage::Export, &context.func.signature).map_err(|e| NativeError::not_implemented(e.to_string()))?;
    let mut fbctx = FunctionBuilderContext::new(); let block = { let mut b=FunctionBuilder::new(&mut context.func,&mut fbctx); let block=b.create_block(); b.switch_to_block(block); b.seal_block(block); let mut values=HashMap::new(); for stmt in &program.0.functions.iter().find(|f|f.name=="main").unwrap().body { match stmt { Stmt::Let{name,value,..}=>{let v=lower(&mut b,value,&values);values.insert(name,v);}, Stmt::Assign{name,value}=>{let v=lower(&mut b,value,&values);values.insert(name,v);}, Stmt::Return(value)=>{let v=lower(&mut b,value,&values);b.ins().return_(&[v]);} } } b.finalize(); block };
    let _=block; module.define_function(id,&mut context).map_err(|e| NativeError::not_implemented(e.to_string()))?; module.clear_context(&mut context);
    fs::write(output,module.finish().emit().map_err(|e| NativeError::not_implemented(e.to_string()))?).map_err(NativeError::io)
}
fn lower(b:&mut FunctionBuilder, x:&Expr, names:&HashMap<&String,cranelift_codegen::ir::Value>)->cranelift_codegen::ir::Value { match x { Expr::Int(n,_)=>b.ins().iconst(types::I32,*n), Expr::Name(n)=>*names.get(n).unwrap(), Expr::Binary(a,op,c)=>{let x=lower(b,a,names);let y=lower(b,c,names);match op { '+' => b.ins().iadd(x,y), '-' => b.ins().isub(x,y), '*' => b.ins().imul(x,y), _ => b.ins().sdiv(x,y) }}}}
pub fn link_host(object:&Path, executable:&Path)->Result<(),NativeError>{let output=Command::new("cc").arg(object).arg("-o").arg(executable).output().map_err(NativeError::io)?;if output.status.success(){Ok(())}else{Err(NativeError::not_implemented(String::from_utf8_lossy(&output.stderr).to_string()))}}
