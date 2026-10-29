use super::{
    check::TypedProgram,
    error::NativeError,
    syntax::{Expr, Function, Stmt, Type},
};
use cranelift_codegen::{
    ir::{types, AbiParam, InstBuilder, Signature, Value},
    settings,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{default_libcall_names, FuncId, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};
use std::{collections::HashMap, fs, path::Path, process::Command};

pub fn emit_object(program: &TypedProgram, output: &Path) -> Result<(), NativeError> {
    let flags = settings::Flags::new(settings::builder());
    let isa = cranelift_native::builder()
        .map_err(|error| NativeError::not_implemented(error.to_string()))?
        .finish(flags)
        .map_err(|error| NativeError::not_implemented(error.to_string()))?;
    let builder = ObjectBuilder::new(isa, "xenith", default_libcall_names())
        .map_err(|error| NativeError::not_implemented(error.to_string()))?;
    let mut module = ObjectModule::new(builder);
    let mut functions = HashMap::new();

    for function in &program.0.functions {
        let linkage = if function.name == "main" {
            Linkage::Export
        } else {
            Linkage::Local
        };
        let id = module
            .declare_function(&function.name, linkage, &signature(&module, function))
            .map_err(|error| NativeError::not_implemented(error.to_string()))?;
        functions.insert(function.name.clone(), id);
    }

    for function in &program.0.functions {
        emit_function(&mut module, function, &functions)?;
    }

    fs::write(
        output,
        module
            .finish()
            .emit()
            .map_err(|error| NativeError::not_implemented(error.to_string()))?,
    )
    .map_err(NativeError::io)
}

fn signature(module: &ObjectModule, function: &Function) -> Signature {
    let mut signature = module.make_signature();
    signature.params.extend(
        function
            .params
            .iter()
            .map(|parameter| AbiParam::new(native_type(&parameter.ty))),
    );
    signature
        .returns
        .push(AbiParam::new(native_type(&function.result)));
    signature
}

fn emit_function(
    module: &mut ObjectModule,
    function: &Function,
    functions: &HashMap<String, FuncId>,
) -> Result<(), NativeError> {
    let id = functions[&function.name];
    let mut context = module.make_context();
    context.func.signature = signature(module, function);
    let mut function_builder_context = FunctionBuilderContext::new();
    {
        let mut builder = FunctionBuilder::new(&mut context.func, &mut function_builder_context);
        let entry = builder.create_block();
        builder.append_block_params_for_function_params(entry);
        builder.switch_to_block(entry);
        builder.seal_block(entry);

        let mut values = HashMap::new();
        let incoming = builder.block_params(entry).to_vec();
        for (parameter, value) in function.params.iter().zip(incoming) {
            values.insert(parameter.name.clone(), value);
        }
        for statement in &function.body {
            match statement {
                Stmt::Let { name, value, .. } | Stmt::Assign { name, value } => {
                    let value = lower(&mut builder, value, &values, functions, module);
                    values.insert(name.clone(), value);
                }
                Stmt::Return(value) => {
                    let value = lower(&mut builder, value, &values, functions, module);
                    builder.ins().return_(&[value]);
                }
            }
        }
        builder.finalize();
    }
    module
        .define_function(id, &mut context)
        .map_err(|error| NativeError::not_implemented(error.to_string()))?;
    module.clear_context(&mut context);
    Ok(())
}

fn lower(
    builder: &mut FunctionBuilder,
    expression: &Expr,
    values: &HashMap<String, Value>,
    functions: &HashMap<String, FuncId>,
    module: &mut ObjectModule,
) -> Value {
    match expression {
        Expr::Int(number, ty) => builder.ins().iconst(native_type(ty), *number),
        Expr::Name(name) => values[name],
        Expr::Call { name, arguments } => {
            let arguments = arguments
                .iter()
                .map(|argument| lower(builder, argument, values, functions, module))
                .collect::<Vec<_>>();
            let callee = module.declare_func_in_func(functions[name], builder.func);
            let call = builder.ins().call(callee, &arguments);
            builder.inst_results(call)[0]
        }
        Expr::Binary(left, operator, right) => {
            let left = lower(builder, left, values, functions, module);
            let right = lower(builder, right, values, functions, module);
            match operator {
                '+' => builder.ins().iadd(left, right),
                '-' => builder.ins().isub(left, right),
                '*' => builder.ins().imul(left, right),
                _ => builder.ins().sdiv(left, right),
            }
        }
    }
}

fn native_type(ty: &Type) -> cranelift_codegen::ir::Type {
    match ty {
        Type::I32 => types::I32,
        Type::I64 => types::I64,
    }
}

pub fn link_host(object: &Path, executable: &Path) -> Result<(), NativeError> {
    let output = Command::new("cc")
        .arg(object)
        .arg("-o")
        .arg(executable)
        .output()
        .map_err(NativeError::io)?;
    if output.status.success() {
        Ok(())
    } else {
        Err(NativeError::not_implemented(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }
}
