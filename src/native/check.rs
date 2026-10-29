use super::syntax::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub parameters: Vec<Type>,
    pub result: Type,
}
#[derive(Debug, Clone)]
pub struct StructLayout {
    pub fields: Vec<(String, Type)>,
}

#[derive(Debug, Clone)]
pub struct TypedProgram(
    pub Program,
    pub HashMap<String, FunctionSignature>,
    pub HashMap<String, StructLayout>,
);

pub fn check(program: &Program) -> Result<TypedProgram, Vec<Diagnostic>> {
    let mut errors = vec![];
    let signatures = signatures(program, &mut errors);
    let layouts = layouts(program, &mut errors);
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
        check_function(function, &signatures, &layouts, &mut errors);
    }
    if errors.is_empty() {
        Ok(TypedProgram(program.clone(), signatures, layouts))
    } else {
        Err(errors)
    }
}
fn layouts(program: &Program, errors: &mut Vec<Diagnostic>) -> HashMap<String, StructLayout> {
    let mut layouts = HashMap::new();
    for structure in &program.structs {
        let mut fields = vec![];
        let mut names = HashMap::new();
        for field in &structure.fields {
            if names.insert(field.name.clone(), ()).is_some() {
                errors.push(Diagnostic {
                    message: format!("duplicate field `{}`", field.name),
                });
            }
            if field.ty != Type::I32 {
                errors.push(Diagnostic {
                    message: "struct fields must be i32".into(),
                });
            }
            fields.push((field.name.clone(), field.ty.clone()));
        }
        if !(1..=2).contains(&fields.len()) {
            errors.push(Diagnostic {
                message: "structs need one or two i32 fields".into(),
            });
        }
        if layouts
            .insert(structure.name.clone(), StructLayout { fields })
            .is_some()
        {
            errors.push(Diagnostic {
                message: format!("duplicate struct `{}`", structure.name),
            });
        }
    }
    layouts
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
    layouts: &HashMap<String, StructLayout>,
    errors: &mut Vec<Diagnostic>,
) {
    let mut names = HashMap::new();
    for parameter in &function.params {
        if names
            .insert(parameter.name.clone(), (parameter.ty.clone(), false))
            .is_some()
        {
            errors.push(Diagnostic {
                message: format!("duplicate parameter `{}`", parameter.name),
            });
        }
    }
    let mut returned = false;
    for statement in &function.body {
        match statement {
            Stmt::While { condition, body } => {
                if expr(condition, &names, signatures, layouts, errors) != Some(Type::Bool) {
                    errors.push(Diagnostic {
                        message: "while condition must be bool".into(),
                    });
                }
                for statement in body {
                    if let Stmt::Assign { name, value } = statement {
                        if let Some((ty, mutable)) = names.get(name).cloned() {
                            if !mutable {
                                errors.push(Diagnostic {
                                    message: "immutable binding".into(),
                                });
                            }
                            if expr(value, &names, signatures, layouts, errors) != Some(ty) {
                                errors.push(Diagnostic {
                                    message: "type mismatch".into(),
                                });
                            }
                        }
                    }
                }
            }
            Stmt::If {
                condition,
                then_body,
                else_body,
            } => {
                if expr(condition, &names, signatures, layouts, errors) != Some(Type::Bool) {
                    errors.push(Diagnostic {
                        message: "if condition must be bool".into(),
                    });
                }
                for branch in [then_body, else_body] {
                    for statement in branch {
                        if let Stmt::Return(value) = statement {
                            returned = true;
                            if expr(value, &names, signatures, layouts, errors)
                                != Some(function.result.clone())
                            {
                                errors.push(Diagnostic {
                                    message: "branch return type mismatch".into(),
                                });
                            }
                        }
                    }
                }
            }
            Stmt::Let {
                name,
                mutable,
                ty,
                value,
            } => {
                let value_type = expr(value, &names, signatures, layouts, errors);
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
                    if let Some(value_type) = expr(value, &names, signatures, layouts, errors) {
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
                if expr(value, &names, signatures, layouts, errors) != Some(function.result.clone())
                {
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
    layouts: &HashMap<String, StructLayout>,
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
                if expr(argument, names, signatures, layouts, errors)
                    != Some(parameter_type.clone())
                {
                    errors.push(Diagnostic {
                        message: format!("argument type does not match `{name}`"),
                    });
                }
            }
            Some(signature.result.clone())
        }
        Expr::StructLiteral { name, fields } => {
            let Some(layout) = layouts.get(name) else {
                errors.push(Diagnostic {
                    message: "unknown struct".into(),
                });
                return None;
            };
            for (field, ty) in &layout.fields {
                match fields.iter().find(|(given, _)| given == field) {
                    Some((_, value))
                        if expr(value, names, signatures, layouts, errors) == Some(ty.clone()) => {}
                    Some(_) => errors.push(Diagnostic {
                        message: "field type mismatch".into(),
                    }),
                    None => errors.push(Diagnostic {
                        message: format!("missing field `{field}`"),
                    }),
                }
            }
            for (field, _) in fields {
                if !layout.fields.iter().any(|(known, _)| known == field) {
                    errors.push(Diagnostic {
                        message: format!("unknown field `{field}`"),
                    });
                }
            }
            Some(Type::Named(name.clone()))
        }
        Expr::Field { base, name } => match expr(base, names, signatures, layouts, errors) {
            Some(Type::Named(structure)) => layouts
                .get(&structure)
                .and_then(|layout| {
                    layout
                        .fields
                        .iter()
                        .find(|(field, _)| field == name)
                        .map(|(_, ty)| ty.clone())
                })
                .or_else(|| {
                    errors.push(Diagnostic {
                        message: format!("unknown field `{name}`"),
                    });
                    None
                }),
            _ => {
                errors.push(Diagnostic {
                    message: "field access requires a struct".into(),
                });
                None
            }
        },
        Expr::Compare(left, _, right) => {
            let left = expr(left, names, signatures, layouts, errors);
            let right = expr(right, names, signatures, layouts, errors);
            if left != right {
                errors.push(Diagnostic {
                    message: "comparison operands must have same type".into(),
                });
            }
            Some(Type::Bool)
        }
        Expr::Binary(left, _, right) => {
            let left_type = expr(left, names, signatures, layouts, errors);
            let right_type = expr(right, names, signatures, layouts, errors);
            if left_type != right_type {
                errors.push(Diagnostic {
                    message: "numeric operands must have same type".into(),
                });
            }
            left_type
        }
    }
}
