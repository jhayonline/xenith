use super::syntax::*;
use std::collections::{HashMap, HashSet};

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
            span: None,
        });
    } else {
        let main = mains[0];
        if !main.params.is_empty() {
            errors.push(Diagnostic {
                message: "main takes no parameters".into(),
                span: None,
            });
        }
        if main.result != Type::I32 {
            errors.push(Diagnostic {
                message: "main must return -> i32".into(),
                span: None,
            });
        }
    }
    for function in &program.functions {
        for parameter in &function.params {
            validate_type(&parameter.ty, &layouts, &mut errors);
        }
        validate_type(&function.result, &layouts, &mut errors);
        check_function(function, &signatures, &layouts, &mut errors);
    }
    if errors.is_empty() {
        Ok(TypedProgram(program.clone(), signatures, layouts))
    } else {
        Err(errors)
    }
}

fn validate_type(ty: &Type, layouts: &HashMap<String, StructLayout>, errors: &mut Vec<Diagnostic>) {
    if let Type::Named(name) = ty {
        if !layouts.contains_key(name) {
            errors.push(Diagnostic {
                message: format!("unknown type `{name}`"),
                span: None,
            });
        }
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
                    span: None,
                });
            }
            fields.push((field.name.clone(), field.ty.clone()));
        }
        if fields.is_empty() {
            errors.push(Diagnostic {
                message: "structs need at least one field".into(),
                span: None,
            });
        }
        if layouts
            .insert(structure.name.clone(), StructLayout { fields })
            .is_some()
        {
            errors.push(Diagnostic {
                message: format!("duplicate struct `{}`", structure.name),
                span: None,
            });
        }
    }
    let mut states = HashMap::new();
    for name in layouts.keys().cloned().collect::<Vec<_>>() {
        validate_layout(&name, &layouts, &mut states, errors);
    }
    layouts
}

fn validate_layout(
    name: &str,
    layouts: &HashMap<String, StructLayout>,
    states: &mut HashMap<String, u8>,
    errors: &mut Vec<Diagnostic>,
) {
    match states.get(name) {
        Some(1) => {
            errors.push(Diagnostic {
                message: "recursive struct layout".into(),
                span: None,
            });
            return;
        }
        Some(2) => return,
        _ => {}
    }
    states.insert(name.into(), 1);
    for (_, ty) in &layouts[name].fields {
        if let Type::Named(field_type) = ty {
            if layouts.contains_key(field_type) {
                validate_layout(field_type, layouts, states, errors);
            } else {
                errors.push(Diagnostic {
                    message: format!("unknown struct `{field_type}`"),
                    span: None,
                });
            }
        }
    }
    states.insert(name.into(), 2);
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
                span: None,
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
    let mut declared = HashSet::new();
    for parameter in &function.params {
        if names
            .insert(parameter.name.clone(), (parameter.ty.clone(), false))
            .is_some()
        {
            errors.push(Diagnostic {
                message: format!("duplicate parameter `{}`", parameter.name),
                span: None,
            });
        }
        declared.insert(parameter.name.clone());
    }
    let mut returned = false;
    for statement in &function.body {
        match statement {
            Stmt::While { condition, body } => {
                if expr(condition, &names, signatures, layouts, errors) != Some(Type::Bool) {
                    errors.push(Diagnostic {
                        message: "while condition must be bool".into(),
                        span: None,
                    });
                }
                let body_returns = check_block(body, &names, function, signatures, layouts, errors);
                returned |= matches!(condition, Expr::Bool(true)) && body_returns;
            }
            Stmt::If {
                condition,
                then_body,
                else_body,
            } => {
                if expr(condition, &names, signatures, layouts, errors) != Some(Type::Bool) {
                    errors.push(Diagnostic {
                        message: "if condition must be bool".into(),
                        span: None,
                    });
                }
                let then_returns =
                    check_block(then_body, &names, function, signatures, layouts, errors);
                let else_returns =
                    check_block(else_body, &names, function, signatures, layouts, errors);
                returned |= then_returns && else_returns;
            }
            Stmt::Let {
                name,
                mutable,
                ty,
                value,
            } => {
                let duplicate = !declared.insert(name.clone());
                if duplicate {
                    errors.push(Diagnostic {
                        message: format!("duplicate binding `{name}`"),
                        span: None,
                    });
                }
                if let Some(ty) = ty {
                    validate_type(ty, layouts, errors);
                }
                let value_type = expr(value, &names, signatures, layouts, errors);
                if let (Some(annotation), Some(inferred)) = (ty, value_type.clone()) {
                    if *annotation != inferred {
                        errors.push(Diagnostic {
                            message: "type mismatch".into(),
                            span: None,
                        });
                    }
                }
                if !duplicate {
                    names.insert(
                        name.clone(),
                        (ty.clone().or(value_type).unwrap_or(Type::I32), *mutable),
                    );
                }
            }
            Stmt::Assign { name, value } => {
                if let Some((binding_type, mutable)) = names.get(name).cloned() {
                    if !mutable {
                        errors.push(Diagnostic {
                            message: format!("immutable binding `{name}`"),
                            span: None,
                        });
                    }
                    if let Some(value_type) = expr(value, &names, signatures, layouts, errors) {
                        if value_type != binding_type {
                            errors.push(Diagnostic {
                                message: format!("type mismatch for `{name}`"),
                                span: None,
                            });
                        }
                    }
                } else {
                    errors.push(Diagnostic {
                        message: format!("unknown name `{name}`"),
                        span: None,
                    });
                }
            }
            Stmt::Return(value) => {
                returned = true;
                if expr(value, &names, signatures, layouts, errors) != Some(function.result.clone())
                {
                    errors.push(Diagnostic {
                        message: format!("return type mismatch in `{}`", function.name),
                        span: None,
                    });
                }
            }
        }
    }
    if !returned {
        errors.push(Diagnostic {
            message: format!("{} must return", function.name),
            span: None,
        });
    }
}

fn check_block(
    statements: &[Stmt],
    inherited: &HashMap<String, (Type, bool)>,
    function: &Function,
    signatures: &HashMap<String, FunctionSignature>,
    layouts: &HashMap<String, StructLayout>,
    errors: &mut Vec<Diagnostic>,
) -> bool {
    let mut names = inherited.clone();
    let mut declared = HashSet::new();
    let mut returned = false;
    for statement in statements {
        match statement {
            Stmt::Let {
                name,
                mutable,
                ty,
                value,
            } => {
                let duplicate = !declared.insert(name.clone());
                if duplicate {
                    errors.push(Diagnostic {
                        message: format!("duplicate binding `{name}`"),
                        span: None,
                    });
                }
                if let Some(ty) = ty {
                    validate_type(ty, layouts, errors);
                }
                let value_type = expr(value, &names, signatures, layouts, errors);
                if let (Some(annotation), Some(inferred)) = (ty, value_type.clone()) {
                    if *annotation != inferred {
                        errors.push(Diagnostic {
                            message: format!("type mismatch for `{name}`"),
                            span: None,
                        });
                    }
                }
                if !duplicate {
                    names.insert(
                        name.clone(),
                        (ty.clone().or(value_type).unwrap_or(Type::I32), *mutable),
                    );
                }
            }
            Stmt::Assign { name, value } => match names.get(name).cloned() {
                Some((ty, mutable)) => {
                    if !mutable {
                        errors.push(Diagnostic {
                            message: format!("immutable binding `{name}`"),
                            span: None,
                        });
                    }
                    if expr(value, &names, signatures, layouts, errors) != Some(ty) {
                        errors.push(Diagnostic {
                            message: "type mismatch".into(),
                            span: None,
                        });
                    }
                }
                None => errors.push(Diagnostic {
                    message: format!("unknown name `{name}`"),
                    span: None,
                }),
            },
            Stmt::Return(value) => {
                returned = true;
                if expr(value, &names, signatures, layouts, errors) != Some(function.result.clone())
                {
                    errors.push(Diagnostic {
                        message: format!("return type mismatch in `{}`", function.name),
                        span: None,
                    });
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
                        span: None,
                    });
                }
                let then_returns =
                    check_block(then_body, &names, function, signatures, layouts, errors);
                let else_returns =
                    check_block(else_body, &names, function, signatures, layouts, errors);
                returned |= then_returns && else_returns;
            }
            Stmt::While { condition, body } => {
                if expr(condition, &names, signatures, layouts, errors) != Some(Type::Bool) {
                    errors.push(Diagnostic {
                        message: "while condition must be bool".into(),
                        span: None,
                    });
                }
                let body_returns = check_block(body, &names, function, signatures, layouts, errors);
                returned |= matches!(condition, Expr::Bool(true)) && body_returns;
            }
        }
    }
    returned
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
        Expr::Bool(_) => Some(Type::Bool),
        Expr::Not(value) => {
            if expr(value, names, signatures, layouts, errors) != Some(Type::Bool) {
                errors.push(Diagnostic {
                    message: "`!` requires bool".into(),
                    span: None,
                });
            }
            Some(Type::Bool)
        }
        Expr::Logical(left, operator, right) => {
            let left_type = expr(left, names, signatures, layouts, errors);
            if left_type != Some(Type::Bool) {
                errors.push(Diagnostic {
                    message: format!("`{operator}` requires bool"),
                    span: None,
                });
            }
            let short_circuits = matches!(
                (operator.as_str(), left.as_ref()),
                ("&&", Expr::Bool(false)) | ("||", Expr::Bool(true))
            );
            let mut right_errors = vec![];
            let right_type = expr(right, names, signatures, layouts, &mut right_errors);
            if right_type != Some(Type::Bool) {
                right_errors.push(Diagnostic {
                    message: format!("`{operator}` requires bool"),
                    span: None,
                });
            }
            if short_circuits {
                right_errors.retain(|error| error.message != "division by zero");
            }
            errors.append(&mut right_errors);
            Some(Type::Bool)
        }
        Expr::Name(name) => names
            .get(name)
            .map(|binding| binding.0.clone())
            .or_else(|| {
                errors.push(Diagnostic {
                    message: format!("unknown name `{name}`"),
                    span: None,
                });
                None
            }),
        Expr::Call { name, arguments } => {
            let Some(signature) = signatures.get(name) else {
                errors.push(Diagnostic {
                    message: format!("unknown function `{name}`"),
                    span: None,
                });
                return None;
            };
            if arguments.len() != signature.parameters.len() {
                errors.push(Diagnostic {
                    message: format!(
                        "function `{name}` expects {} arguments",
                        signature.parameters.len()
                    ),
                    span: None,
                });
            }
            for (argument, parameter_type) in arguments.iter().zip(&signature.parameters) {
                if expr(argument, names, signatures, layouts, errors)
                    != Some(parameter_type.clone())
                {
                    errors.push(Diagnostic {
                        message: format!("argument type does not match `{name}`"),
                        span: None,
                    });
                }
            }
            Some(signature.result.clone())
        }
        Expr::StructLiteral { name, fields } => {
            let Some(layout) = layouts.get(name) else {
                errors.push(Diagnostic {
                    message: "unknown struct".into(),
                    span: None,
                });
                return None;
            };
            let mut given = HashMap::new();
            for (field, _) in fields {
                if given.insert(field, ()).is_some() {
                    errors.push(Diagnostic {
                        message: format!("duplicate field `{field}`"),
                        span: None,
                    });
                }
            }
            for (field, ty) in &layout.fields {
                match fields.iter().find(|(given, _)| given == field) {
                    Some((_, value))
                        if expr(value, names, signatures, layouts, errors) == Some(ty.clone()) => {}
                    Some(_) => errors.push(Diagnostic {
                        message: "field type mismatch".into(),
                        span: None,
                    }),
                    None => errors.push(Diagnostic {
                        message: format!("missing field `{field}`"),
                        span: None,
                    }),
                }
            }
            for (field, _) in fields {
                if !layout.fields.iter().any(|(known, _)| known == field) {
                    errors.push(Diagnostic {
                        message: format!("unknown field `{field}`"),
                        span: None,
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
                        span: None,
                    });
                    None
                }),
            _ => {
                errors.push(Diagnostic {
                    message: "field access requires a struct".into(),
                    span: None,
                });
                None
            }
        },
        Expr::Compare(left, operator, right) => {
            let left = expr(left, names, signatures, layouts, errors);
            let right = expr(right, names, signatures, layouts, errors);
            if left != right {
                errors.push(Diagnostic {
                    message: "comparison operands must have same type".into(),
                    span: None,
                });
            } else if !matches!(operator.as_str(), "==" | "!=")
                && !matches!(left, Some(Type::I32 | Type::I64))
            {
                errors.push(Diagnostic {
                    message: "ordering comparisons require integer operands".into(),
                    span: None,
                });
            }
            Some(Type::Bool)
        }
        Expr::Binary(left, operator, right) => {
            let left_type = expr(left, names, signatures, layouts, errors);
            let right_type = expr(right, names, signatures, layouts, errors);
            if left_type != right_type {
                errors.push(Diagnostic {
                    message: "numeric operands must have same type".into(),
                    span: None,
                });
            } else if !matches!(left_type, Some(Type::I32 | Type::I64)) {
                errors.push(Diagnostic {
                    message: "arithmetic operands require integers".into(),
                    span: None,
                });
            } else if *operator == '/' && matches!(right.as_ref(), Expr::Int(0, _)) {
                errors.push(Diagnostic {
                    message: "division by zero".into(),
                    span: None,
                });
            }
            left_type
        }
    }
}
