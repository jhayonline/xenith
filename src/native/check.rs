use super::syntax::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub parameters: Vec<Type>,
    pub result: Type,
}

#[derive(Debug, Clone)]
pub struct TypedProgram(pub Program, pub HashMap<String, FunctionSignature>);

pub fn check(program: &Program) -> Result<TypedProgram, Vec<Diagnostic>> {
    let mut errors = vec![];
    let signatures = signatures(program, &mut errors);
    let mains: Vec<_> = program
        .functions
        .iter()
        .filter(|f| f.name == "main")
        .collect();
    if mains.len() != 1 {
        errors.push(Diagnostic {
            message: "expected exactly one main".into(),
        });
    } else {
        let main = mains[0];
        if !main.params.is_empty() {
            errors.push(Diagnostic {
                message: "main takes no parameters".into(),
            });
        }
        if main.result != Type::I32 {
            errors.push(Diagnostic {
                message: "main must return -> i32".into(),
            });
        }
    }
    for function in &program.functions {
        check_function(function, &signatures, &mut errors);
    }
    if errors.is_empty() {
        Ok(TypedProgram(program.clone(), signatures))
    } else {
        Err(errors)
    }
}

fn signatures(
    program: &Program,
    errors: &mut Vec<Diagnostic>,
) -> HashMap<String, FunctionSignature> {
    let mut signatures = HashMap::new();
    for function in &program.functions {
        let signature = FunctionSignature {
            parameters: function
                .params
                .iter()
                .map(|parameter| parameter.ty.clone())
                .collect(),
            result: function.result.clone(),
        };
        if signatures
            .insert(function.name.clone(), signature)
            .is_some()
        {
            errors.push(Diagnostic {
                message: format!("duplicate function `{}`", function.name),
            });
        }
    }
    signatures
}

fn check_function(
    function: &Function,
    signatures: &HashMap<String, FunctionSignature>,
    errors: &mut Vec<Diagnostic>,
) {
    let mut names: HashMap<String, (Type, bool)> = function
        .params
        .iter()
        .map(|parameter| (parameter.name.clone(), (parameter.ty.clone(), false)))
        .collect();
    let mut returned = false;
    for statement in &function.body {
        match statement {
            Stmt::Let {
                name,
                mutable,
                ty,
                value,
            } => {
                let value_type = expr(value, &names, signatures, errors);
                if let (Some(annotation), Some(inferred)) = (ty, value_type.clone()) {
                    if *annotation != inferred {
                        errors.push(Diagnostic {
                            message: "type mismatch".into(),
                        });
                    }
                }
                names.insert(
                    name.clone(),
                    (ty.clone().or(value_type).unwrap_or(Type::I32), *mutable),
                );
            }
            Stmt::Assign { name, value } => {
                if let Some((binding_type, mutable)) = names.get(name).cloned() {
                    if !mutable {
                        errors.push(Diagnostic {
                            message: "immutable binding".into(),
                        });
                    }
                    if let Some(value_type) = expr(value, &names, signatures, errors) {
                        if value_type != binding_type {
                            errors.push(Diagnostic {
                                message: "type mismatch".into(),
                            });
                        }
                    }
                } else {
                    errors.push(Diagnostic {
                        message: "unknown name".into(),
                    });
                }
            }
            Stmt::Return(value) => {
                returned = true;
                if expr(value, &names, signatures, errors) != Some(function.result.clone()) {
                    errors.push(Diagnostic {
                        message: format!("return must be {:?}", function.result).to_lowercase(),
                    });
                }
            }
        }
    }
    if !returned {
        errors.push(Diagnostic {
            message: format!("{} must return", function.name),
        });
    }
}

fn expr(
    expression: &Expr,
    names: &HashMap<String, (Type, bool)>,
    signatures: &HashMap<String, FunctionSignature>,
    errors: &mut Vec<Diagnostic>,
) -> Option<Type> {
    match expression {
        Expr::Int(_, ty) => Some(ty.clone()),
        Expr::Name(name) => names
            .get(name)
            .map(|binding| binding.0.clone())
            .or_else(|| {
                errors.push(Diagnostic {
                    message: "unknown name".into(),
                });
                None
            }),
        Expr::Call { name, arguments } => {
            let Some(signature) = signatures.get(name) else {
                errors.push(Diagnostic {
                    message: format!("unknown function `{name}`"),
                });
                return None;
            };
            if arguments.len() != signature.parameters.len() {
                errors.push(Diagnostic {
                    message: format!(
                        "function `{name}` expects {} arguments",
                        signature.parameters.len()
                    ),
                });
            }
            for (argument, parameter_type) in arguments.iter().zip(&signature.parameters) {
                if expr(argument, names, signatures, errors) != Some(parameter_type.clone()) {
                    errors.push(Diagnostic {
                        message: format!("argument type does not match `{name}`"),
                    });
                }
            }
            Some(signature.result.clone())
        }
        Expr::Binary(left, _, right) => {
            let left_type = expr(left, names, signatures, errors);
            let right_type = expr(right, names, signatures, errors);
            if left_type != right_type {
                errors.push(Diagnostic {
                    message: "numeric operands must have same type".into(),
                });
            }
            left_type
        }
    }
}
