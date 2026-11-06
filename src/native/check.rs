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
    pub variants: Option<Vec<(String, Vec<Type>)>>,
}

#[derive(Debug, Clone)]
pub struct TypedProgram(
    pub Program,
    pub HashMap<String, FunctionSignature>,
    pub HashMap<String, StructLayout>,
);

#[derive(Default)]
struct Flow {
    returns: bool,
    breaks_loop: bool,
    can_fall_through: bool,
}

pub fn check(program: &Program) -> Result<TypedProgram, Vec<Diagnostic>> {
    let mut errors = vec![];
    let program = match monomorphize(program) {
        Ok(program) => program,
        Err(errors) => return Err(errors),
    };
    let mut signatures = signatures(&program, &mut errors);
    let layouts = layouts(&program, &mut errors);
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
    for name in layouts.keys() {
        signatures.remove(&format!("{name}__all"));
        signatures.remove(&format!("{name}__filter"));
    }
    if errors.is_empty() {
        Ok(TypedProgram(program.clone(), signatures, layouts))
    } else {
        errors.sort_by(|left, right| {
            left.span
                .map(|span| span.start)
                .unwrap_or(usize::MAX)
                .cmp(&right.span.map(|span| span.start).unwrap_or(usize::MAX))
                .then_with(|| left.message.cmp(&right.message))
        });
        Err(errors)
    }
}

/// Expand generic functions at their call sites before the ordinary checker and
/// native backend see them. Native code generation therefore only ever receives
/// concrete signatures and does not need a runtime type system.
fn monomorphize(program: &Program) -> Result<Program, Vec<Diagnostic>> {
    let generic: HashMap<_, _> = program
        .functions
        .iter()
        .filter(|f| !f.type_params.is_empty())
        .map(|f| (f.name.clone(), f.clone()))
        .collect();
    if generic.is_empty() {
        return Ok(program.clone());
    }
    let mut signatures = HashMap::new();
    for f in &program.functions {
        signatures.insert(
            f.name.clone(),
            FunctionSignature {
                parameters: f.params.iter().map(|p| p.ty.clone()).collect(),
                result: f.result.clone(),
            },
        );
    }
    let mut generated = Vec::new();
    let mut cache = HashMap::<(String, String), String>::new();
    let mut errors = Vec::new();
    let mut output = Vec::new();
    for function in &program.functions {
        if !function.type_params.is_empty() {
            continue;
        }
        let mut env: HashMap<String, Type> = function
            .params
            .iter()
            .map(|p| (p.name.clone(), p.ty.clone()))
            .collect();
        let body = specialize_statements(
            &function.body,
            &mut env,
            &generic,
            &mut signatures,
            &mut cache,
            &mut generated,
            &mut errors,
        );
        let mut function = function.clone();
        function.body = body;
        output.push(function);
    }
    while let Some(function) = generated.pop() {
        let mut env: HashMap<String, Type> = function
            .params
            .iter()
            .map(|p| (p.name.clone(), p.ty.clone()))
            .collect();
        let body = specialize_statements(
            &function.body,
            &mut env,
            &generic,
            &mut signatures,
            &mut cache,
            &mut generated,
            &mut errors,
        );
        let mut function = function;
        function.body = body;
        output.push(function);
    }
    if errors.is_empty() {
        let mut program = program.clone();
        program.functions = output;
        Ok(program)
    } else {
        Err(errors)
    }
}

fn substitute_type(ty: &Type, substitutions: &HashMap<String, Type>) -> Type {
    match ty {
        Type::Generic(name) => substitutions
            .get(name)
            .cloned()
            .unwrap_or_else(|| ty.clone()),
        Type::List(inner) => Type::List(Box::new(substitute_type(inner, substitutions))),
        Type::Map(key, value) => Type::Map(
            Box::new(substitute_type(key, substitutions)),
            Box::new(substitute_type(value, substitutions)),
        ),
        Type::Tuple(items) => Type::Tuple(
            items
                .iter()
                .map(|t| substitute_type(t, substitutions))
                .collect(),
        ),
        Type::Function(params, result) => Type::Function(
            params
                .iter()
                .map(|t| substitute_type(t, substitutions))
                .collect(),
            Box::new(substitute_type(result, substitutions)),
        ),
        _ => ty.clone(),
    }
}

fn unify_generic(pattern: &Type, actual: &Type, substitutions: &mut HashMap<String, Type>) -> bool {
    match pattern {
        Type::Generic(name) => match substitutions.get(name) {
            Some(existing) => existing == actual,
            None => {
                substitutions.insert(name.clone(), actual.clone());
                true
            }
        },
        Type::List(inner) => {
            matches!(actual, Type::List(value) if unify_generic(inner, value, substitutions))
        }
        Type::Map(key, value) => {
            matches!(actual, Type::Map(actual_key, actual_value) if unify_generic(key, actual_key, substitutions) && unify_generic(value, actual_value, substitutions))
        }
        Type::Tuple(items) => {
            matches!(actual, Type::Tuple(actual_items) if items.len() == actual_items.len() && items.iter().zip(actual_items).all(|(a,b)| unify_generic(a,b,substitutions)))
        }
        Type::Function(params, result) => {
            matches!(actual, Type::Function(actual_params, actual_result) if params.len() == actual_params.len() && params.iter().zip(actual_params).all(|(a,b)| unify_generic(a,b,substitutions)) && unify_generic(result, actual_result, substitutions))
        }
        _ => pattern == actual,
    }
}

fn type_key(ty: &Type) -> String {
    match ty {
        Type::I32 => "i32".into(),
        Type::I64 => "i64".into(),
        Type::Float => "float".into(),
        Type::Bool => "bool".into(),
        Type::Null => "null".into(),
        Type::String => "string".into(),
        Type::Bytes => "bytes".into(),
        Type::Named(n) | Type::Generic(n) => n.clone(),
        Type::List(t) => format!("list_{}", type_key(t)),
        Type::Map(k, v) => format!("map_{}_{}", type_key(k), type_key(v)),
        Type::Tuple(ts) => format!(
            "tuple_{}",
            ts.iter().map(type_key).collect::<Vec<_>>().join("_")
        ),
        Type::Function(ps, r) => format!(
            "fn_{}_{}",
            ps.iter().map(type_key).collect::<Vec<_>>().join("_"),
            type_key(r)
        ),
    }
}

fn infer_generic_expr(
    expr: &Expr,
    env: &HashMap<String, Type>,
    signatures: &HashMap<String, FunctionSignature>,
) -> Option<Type> {
    match expr {
        Expr::Int(_, ty) => Some(ty.clone()),
        Expr::Float(_) => Some(Type::Float),
        Expr::Bool(_) => Some(Type::Bool),
        Expr::Null => Some(Type::Null),
        Expr::String(_) | Expr::Interpolated(_, _) => Some(Type::String),
        Expr::Bytes(_, _) => Some(Type::Bytes),
        Expr::Name(n) => env.get(n).cloned().or_else(|| {
            signatures
                .get(n)
                .map(|s| Type::Function(s.parameters.clone(), Box::new(s.result.clone())))
        }),
        Expr::List(items, _) => Some(Type::List(Box::new(infer_generic_expr(
            items.first()?,
            env,
            signatures,
        )?))),
        Expr::Map(items, _) => Some(Type::Map(
            Box::new(infer_generic_expr(&items.first()?.0, env, signatures)?),
            Box::new(infer_generic_expr(&items.first()?.1, env, signatures)?),
        )),
        Expr::Tuple(items) => Some(Type::Tuple(
            items
                .iter()
                .map(|e| infer_generic_expr(e, env, signatures))
                .collect::<Option<Vec<_>>>()?,
        )),
        Expr::Index { base, .. } => match infer_generic_expr(base, env, signatures)? {
            Type::List(t) | Type::Map(_, t) => Some(*t),
            Type::Bytes => Some(Type::I32),
            _ => None,
        },
        Expr::Call { name, .. } => signatures.get(name).map(|s| s.result.clone()),
        Expr::StructLiteral { name, .. }
        | Expr::EnumVariant {
            enum_name: name, ..
        } => Some(Type::Named(name.clone())),
        Expr::Field { .. } => None,
        Expr::Neg(e) | Expr::Not(e) => infer_generic_expr(e, env, signatures),
        Expr::Logical(_, _, _) | Expr::Compare(_, _, _) => Some(Type::Bool),
        Expr::Binary(left, _, _, _) => infer_generic_expr(left, env, signatures),
        Expr::Closure { params, result, .. } => Some(Type::Function(
            params.iter().map(|p| p.ty.clone()).collect(),
            Box::new(result.clone()),
        )),
        Expr::Match { arms, .. } => infer_generic_expr(&arms.first()?.body, env, signatures),
    }
}

fn specialize_call(
    name: &str,
    arguments: &[Expr],
    env: &HashMap<String, Type>,
    generic: &HashMap<String, Function>,
    signatures: &mut HashMap<String, FunctionSignature>,
    cache: &mut HashMap<(String, String), String>,
    generated: &mut Vec<Function>,
    errors: &mut Vec<Diagnostic>,
) -> String {
    let Some(template) = generic.get(name) else {
        return name.into();
    };
    let actuals: Option<Vec<_>> = arguments
        .iter()
        .map(|a| infer_generic_expr(a, env, signatures))
        .collect();
    let Some(actuals) = actuals else {
        errors.push(Diagnostic {
            message: format!("cannot infer type arguments for generic function `{name}`"),
            span: None,
        });
        return name.into();
    };
    if actuals.len() != template.params.len() {
        return name.into();
    }
    let mut substitutions = HashMap::new();
    for (param, actual) in template.params.iter().zip(&actuals) {
        if !unify_generic(&param.ty, actual, &mut substitutions) {
            errors.push(Diagnostic {
                message: format!("type arguments for `{name}` do not agree"),
                span: None,
            });
            return name.into();
        }
    }
    let concrete: Vec<_> = template
        .type_params
        .iter()
        .map(|n| substitutions.get(n).cloned())
        .collect::<Option<Vec<_>>>()
        .unwrap_or_default();
    if concrete.len() != template.type_params.len() {
        errors.push(Diagnostic {
            message: format!("cannot infer all type arguments for `{name}`"),
            span: None,
        });
        return name.into();
    }
    let concrete_key = concrete.iter().map(type_key).collect::<Vec<_>>().join("_");
    let key = (name.into(), concrete_key.clone());
    if let Some(existing) = cache.get(&key) {
        return existing.clone();
    }
    let mangled = format!(
        "{}__generic__{}",
        name,
        concrete.iter().map(type_key).collect::<Vec<_>>().join("_")
    );
    cache.insert(key, mangled.clone());
    let mut specialized = template.clone();
    specialized.name = mangled;
    specialized.type_params.clear();
    for p in &mut specialized.params {
        p.ty = substitute_type(&p.ty, &substitutions);
    }
    specialized.result = substitute_type(&specialized.result, &substitutions);
    signatures.insert(
        specialized.name.clone(),
        FunctionSignature {
            parameters: specialized.params.iter().map(|p| p.ty.clone()).collect(),
            result: specialized.result.clone(),
        },
    );
    substitute_statements(&mut specialized.body, &substitutions);
    generated.push(specialized);
    cache.get(&(name.into(), concrete_key)).cloned().unwrap()
}

fn specialize_function_value(
    name: &str,
    expected: &Type,
    generic: &HashMap<String, Function>,
    signatures: &mut HashMap<String, FunctionSignature>,
    cache: &mut HashMap<(String, String), String>,
    generated: &mut Vec<Function>,
    errors: &mut Vec<Diagnostic>,
) -> Option<String> {
    let template = generic.get(name)?;
    let Type::Function(parameters, result) = expected else {
        errors.push(Diagnostic { message: format!("generic function `{name}` needs a concrete function type"), span: None });
        return None;
    };
    if parameters.len() != template.params.len() {
        errors.push(Diagnostic { message: format!("function value `{name}` has the wrong number of parameters"), span: None });
        return None;
    }
    let mut substitutions = HashMap::new();
    for (parameter, actual) in template.params.iter().zip(parameters) {
        if !unify_generic(&parameter.ty, actual, &mut substitutions) {
            errors.push(Diagnostic { message: format!("cannot infer generic type parameters for `{name}`"), span: None });
            return None;
        }
    }
    if !unify_generic(&template.result, result, &mut substitutions) {
        errors.push(Diagnostic { message: format!("generic result type for `{name}` does not match the expected function type"), span: None });
        return None;
    }
    let concrete: Vec<_> = template.type_params.iter().map(|n| substitutions.get(n).cloned()).collect::<Option<Vec<_>>>()?;
    let key_text = concrete.iter().map(type_key).collect::<Vec<_>>().join("_");
    let key = (name.to_owned(), key_text.clone());
    if let Some(existing) = cache.get(&key) { return Some(existing.clone()); }
    let mangled = format!("{}__generic__{}", name, key_text);
    cache.insert(key, mangled.clone());
    let mut specialized = template.clone();
    specialized.name = mangled.clone();
    specialized.type_params.clear();
    for parameter in &mut specialized.params { parameter.ty = substitute_type(&parameter.ty, &substitutions); }
    specialized.result = substitute_type(&specialized.result, &substitutions);
    signatures.insert(mangled.clone(), FunctionSignature { parameters: specialized.params.iter().map(|p| p.ty.clone()).collect(), result: specialized.result.clone() });
    substitute_statements(&mut specialized.body, &substitutions);
    generated.push(specialized);
    Some(mangled)
}

fn substitute_expr(expr: &mut Expr, substitutions: &HashMap<String, Type>) {
    match expr {
        Expr::Closure {
            params,
            result,
            body,
            ..
        } => {
            for p in params {
                p.ty = substitute_type(&p.ty, substitutions);
            }
            *result = substitute_type(result, substitutions);
            substitute_expr(body, substitutions);
        }
        Expr::Call { arguments, .. } | Expr::Tuple(arguments) | Expr::List(arguments, _) => {
            for e in arguments {
                substitute_expr(e, substitutions);
            }
        }
        Expr::Map(items, _) => {
            for (k, v) in items {
                substitute_expr(k, substitutions);
                substitute_expr(v, substitutions);
            }
        }
        Expr::EnumVariant { arguments, .. } => {
            for e in arguments {
                substitute_expr(e, substitutions);
            }
        }
        Expr::Match { subject, arms } => {
            substitute_expr(subject, substitutions);
            for arm in arms {
                substitute_expr(&mut arm.body, substitutions);
            }
        }
        Expr::StructLiteral { fields, .. } => {
            for (_, e) in fields {
                substitute_expr(e, substitutions);
            }
        }
        Expr::Field { base, .. } => substitute_expr(base, substitutions),
        Expr::Index { base, index, .. } => {
            substitute_expr(base, substitutions);
            substitute_expr(index, substitutions);
        }
        Expr::Neg(e) | Expr::Not(e) => substitute_expr(e, substitutions),
        Expr::Logical(a, _, b) | Expr::Compare(a, _, b) | Expr::Binary(a, _, b, _) => {
            substitute_expr(a, substitutions);
            substitute_expr(b, substitutions);
        }
        _ => {}
    }
}
fn substitute_statements(statements: &mut [Stmt], substitutions: &HashMap<String, Type>) {
    for statement in statements {
        match statement {
            Stmt::While { condition, body } => {
                substitute_expr(condition, substitutions);
                substitute_statements(body, substitutions);
            }
            Stmt::For {
                init,
                condition,
                step,
                body,
            } => {
                substitute_statements(std::slice::from_mut(&mut **init), substitutions);
                substitute_expr(condition, substitutions);
                substitute_statements(std::slice::from_mut(&mut **step), substitutions);
                substitute_statements(body, substitutions);
            }
            Stmt::ForIn { iterable, body, .. } => {
                substitute_expr(iterable, substitutions);
                substitute_statements(body, substitutions);
            }
            Stmt::If {
                condition,
                then_body,
                else_body,
            } => {
                substitute_expr(condition, substitutions);
                substitute_statements(then_body, substitutions);
                substitute_statements(else_body, substitutions);
            }
            Stmt::Let { ty, value, .. } => {
                if let Some(t) = ty {
                    *t = substitute_type(t, substitutions);
                }
                substitute_expr(value, substitutions);
            }
            Stmt::Assign { value, .. } => substitute_expr(value, substitutions),
            Stmt::IndexAssign { base, index, value } => {
                substitute_expr(base, substitutions);
                substitute_expr(index, substitutions);
                substitute_expr(value, substitutions);
            }
            Stmt::Return(e) | Stmt::Expr(e) => substitute_expr(e, substitutions),
            Stmt::Panic { value, .. } => substitute_expr(value, substitutions),
            Stmt::TryCatch {
                body, catch_body, ..
            } => {
                substitute_statements(body, substitutions);
                substitute_statements(catch_body, substitutions);
            }
            Stmt::Stop | Stmt::Skip => {}
        }
    }
}

fn specialize_statements(
    statements: &[Stmt],
    env: &mut HashMap<String, Type>,
    generic: &HashMap<String, Function>,
    signatures: &mut HashMap<String, FunctionSignature>,
    cache: &mut HashMap<(String, String), String>,
    generated: &mut Vec<Function>,
    errors: &mut Vec<Diagnostic>,
) -> Vec<Stmt> {
    statements
        .iter()
        .map(|statement| match statement {
            Stmt::Let {
                name,
                mutable,
                ty,
                value,
            } => {
                let value =
                    specialize_expr(value, env, generic, signatures, cache, generated, errors);
                let inferred = ty
                    .clone()
                    .or_else(|| infer_generic_expr(&value, env, signatures));
                if let Some(t) = inferred {
                    env.insert(name.clone(), t);
                }
                Stmt::Let {
                    name: name.clone(),
                    mutable: *mutable,
                    ty: ty.clone(),
                    value,
                }
            }
            Stmt::Assign { name, value } => Stmt::Assign {
                name: name.clone(),
                value: specialize_expr(value, env, generic, signatures, cache, generated, errors),
            },
            Stmt::IndexAssign { base, index, value } => Stmt::IndexAssign {
                base: specialize_expr(base, env, generic, signatures, cache, generated, errors),
                index: specialize_expr(index, env, generic, signatures, cache, generated, errors),
                value: specialize_expr(value, env, generic, signatures, cache, generated, errors),
            },
            Stmt::Return(value) => Stmt::Return(specialize_expr(
                value, env, generic, signatures, cache, generated, errors,
            )),
            Stmt::Expr(value) => Stmt::Expr(specialize_expr(
                value, env, generic, signatures, cache, generated, errors,
            )),
            Stmt::Panic { value, span } => Stmt::Panic {
                value: specialize_expr(value, env, generic, signatures, cache, generated, errors),
                span: *span,
            },
            Stmt::If {
                condition,
                then_body,
                else_body,
            } => Stmt::If {
                condition: specialize_expr(
                    condition, env, generic, signatures, cache, generated, errors,
                ),
                then_body: specialize_statements(
                    then_body,
                    &mut env.clone(),
                    generic,
                    signatures,
                    cache,
                    generated,
                    errors,
                ),
                else_body: specialize_statements(
                    else_body,
                    &mut env.clone(),
                    generic,
                    signatures,
                    cache,
                    generated,
                    errors,
                ),
            },
            Stmt::While { condition, body } => Stmt::While {
                condition: specialize_expr(
                    condition, env, generic, signatures, cache, generated, errors,
                ),
                body: specialize_statements(
                    body,
                    &mut env.clone(),
                    generic,
                    signatures,
                    cache,
                    generated,
                    errors,
                ),
            },
            Stmt::ForIn {
                names,
                iterable,
                body,
            } => Stmt::ForIn {
                names: names.clone(),
                iterable: specialize_expr(
                    iterable, env, generic, signatures, cache, generated, errors,
                ),
                body: specialize_statements(
                    body,
                    &mut env.clone(),
                    generic,
                    signatures,
                    cache,
                    generated,
                    errors,
                ),
            },
            Stmt::For {
                init,
                condition,
                step,
                body,
            } => Stmt::For {
                init: Box::new(
                    specialize_statements(
                        std::slice::from_ref(&**init),
                        &mut env.clone(),
                        generic,
                        signatures,
                        cache,
                        generated,
                        errors,
                    )
                    .remove(0),
                ),
                condition: specialize_expr(
                    condition, env, generic, signatures, cache, generated, errors,
                ),
                step: Box::new(
                    specialize_statements(
                        std::slice::from_ref(&**step),
                        &mut env.clone(),
                        generic,
                        signatures,
                        cache,
                        generated,
                        errors,
                    )
                    .remove(0),
                ),
                body: specialize_statements(
                    body,
                    &mut env.clone(),
                    generic,
                    signatures,
                    cache,
                    generated,
                    errors,
                ),
            },
            Stmt::TryCatch {
                body,
                binding,
                catch_body,
            } => Stmt::TryCatch {
                body: specialize_statements(
                    body,
                    &mut env.clone(),
                    generic,
                    signatures,
                    cache,
                    generated,
                    errors,
                ),
                binding: binding.clone(),
                catch_body: specialize_statements(
                    catch_body,
                    &mut env.clone(),
                    generic,
                    signatures,
                    cache,
                    generated,
                    errors,
                ),
            },
            Stmt::Stop => Stmt::Stop,
            Stmt::Skip => Stmt::Skip,
        })
        .collect()
}

fn specialize_expr(
    expr: &Expr,
    env: &HashMap<String, Type>,
    generic: &HashMap<String, Function>,
    signatures: &mut HashMap<String, FunctionSignature>,
    cache: &mut HashMap<(String, String), String>,
    generated: &mut Vec<Function>,
    errors: &mut Vec<Diagnostic>,
) -> Expr {
    match expr {
        Expr::Call {
            name,
            arguments,
            span,
        } => {
            let expected_parameters = signatures.get(name).map(|signature| signature.parameters.clone());
            let arguments: Vec<_> = arguments.iter().enumerate().map(|(index, expression)| {
                if let (Expr::Name(value), Some(parameters)) = (expression, &expected_parameters) {
                    if let Some(expected) = parameters.get(index) {
                        if generic.contains_key(value) {
                            if let Some(mangled) = specialize_function_value(value, expected, generic, signatures, cache, generated, errors) {
                                return Expr::Name(mangled);
                            }
                        }
                    }
                }
                specialize_expr(expression, env, generic, signatures, cache, generated, errors)
            }).collect();
            let name = if generic.contains_key(name) {
                specialize_call(
                    name, &arguments, env, generic, signatures, cache, generated, errors,
                )
            } else {
                name.clone()
            };
            Expr::Call {
                name,
                arguments,
                span: *span,
            }
        }
        Expr::List(items, span) => Expr::List(
            items
                .iter()
                .map(|e| specialize_expr(e, env, generic, signatures, cache, generated, errors))
                .collect(),
            *span,
        ),
        Expr::Map(items, span) => Expr::Map(
            items
                .iter()
                .map(|(k, v)| {
                    (
                        specialize_expr(k, env, generic, signatures, cache, generated, errors),
                        specialize_expr(v, env, generic, signatures, cache, generated, errors),
                    )
                })
                .collect(),
            *span,
        ),
        Expr::Tuple(items) => Expr::Tuple(
            items
                .iter()
                .map(|e| specialize_expr(e, env, generic, signatures, cache, generated, errors))
                .collect(),
        ),
        Expr::Index { base, index, span } => Expr::Index {
            base: Box::new(specialize_expr(
                base, env, generic, signatures, cache, generated, errors,
            )),
            index: Box::new(specialize_expr(
                index, env, generic, signatures, cache, generated, errors,
            )),
            span: *span,
        },
        Expr::Field { base, name } => Expr::Field {
            base: Box::new(specialize_expr(
                base, env, generic, signatures, cache, generated, errors,
            )),
            name: name.clone(),
        },
        Expr::Neg(e) => Expr::Neg(Box::new(specialize_expr(
            e, env, generic, signatures, cache, generated, errors,
        ))),
        Expr::Not(e) => Expr::Not(Box::new(specialize_expr(
            e, env, generic, signatures, cache, generated, errors,
        ))),
        Expr::Logical(a, op, b) => Expr::Logical(
            Box::new(specialize_expr(
                a, env, generic, signatures, cache, generated, errors,
            )),
            op.clone(),
            Box::new(specialize_expr(
                b, env, generic, signatures, cache, generated, errors,
            )),
        ),
        Expr::Compare(a, op, b) => Expr::Compare(
            Box::new(specialize_expr(
                a, env, generic, signatures, cache, generated, errors,
            )),
            op.clone(),
            Box::new(specialize_expr(
                b, env, generic, signatures, cache, generated, errors,
            )),
        ),
        Expr::Binary(a, op, b, span) => Expr::Binary(
            Box::new(specialize_expr(
                a, env, generic, signatures, cache, generated, errors,
            )),
            *op,
            Box::new(specialize_expr(
                b, env, generic, signatures, cache, generated, errors,
            )),
            *span,
        ),
        Expr::StructLiteral { name, fields } => Expr::StructLiteral {
            name: name.clone(),
            fields: fields
                .iter()
                .map(|(n, e)| {
                    (
                        n.clone(),
                        specialize_expr(e, env, generic, signatures, cache, generated, errors),
                    )
                })
                .collect(),
        },
        Expr::Closure {
            params,
            result,
            body,
            span,
        } => Expr::Closure {
            params: params.clone(),
            result: result.clone(),
            body: Box::new(specialize_expr(
                body, env, generic, signatures, cache, generated, errors,
            )),
            span: *span,
        },
        Expr::EnumVariant {
            enum_name,
            variant,
            arguments,
        } => Expr::EnumVariant {
            enum_name: enum_name.clone(),
            variant: variant.clone(),
            arguments: arguments
                .iter()
                .map(|e| specialize_expr(e, env, generic, signatures, cache, generated, errors))
                .collect(),
        },
        Expr::Match { subject, arms } => Expr::Match {
            subject: Box::new(specialize_expr(
                subject, env, generic, signatures, cache, generated, errors,
            )),
            arms: arms
                .iter()
                .map(|a| MatchArm {
                    enum_name: a.enum_name.clone(),
                    variant: a.variant.clone(),
                    binding: a.binding.clone(),
                    body: Box::new(specialize_expr(
                        &a.body, env, generic, signatures, cache, generated, errors,
                    )),
                })
                .collect(),
        },
        other => other.clone(),
    }
}

fn validate_type(ty: &Type, layouts: &HashMap<String, StructLayout>, errors: &mut Vec<Diagnostic>) {
    match ty {
        Type::Named(name) if !layouts.contains_key(name) => errors.push(Diagnostic {
            message: format!("unknown type `{name}`"),
            span: None,
        }),
        Type::List(element) => validate_type(element, layouts, errors),
        Type::Map(key, value) => {
            validate_type(key, layouts, errors);
            validate_type(value, layouts, errors);
            if **key != Type::String {
                errors.push(Diagnostic {
                    message: "map keys must be string".into(),
                    span: None,
                });
            }
        }
        Type::Tuple(elements) => {
            for element in elements {
                validate_type(element, layouts, errors);
            }
        }
        Type::Function(parameters, result) => {
            for parameter in parameters {
                validate_type(parameter, layouts, errors);
            }
            validate_type(result, layouts, errors);
        }
        _ => {}
    }
}
fn layouts(program: &Program, errors: &mut Vec<Diagnostic>) -> HashMap<String, StructLayout> {
    let mut layouts = HashMap::new();
    layouts.insert(
        "Error".into(),
        StructLayout {
            fields: vec![
                ("message".into(), Type::String),
                ("code".into(), Type::String),
                ("name".into(), Type::String),
                ("hint".into(), Type::String),
                ("function".into(), Type::String),
                ("offset".into(), Type::I64),
                ("line".into(), Type::I64),
                ("column".into(), Type::I64),
                ("file".into(), Type::String),
            ],
            variants: None,
        },
    );
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
            .insert(
                structure.name.clone(),
                StructLayout {
                    fields,
                    variants: None,
                },
            )
            .is_some()
        {
            errors.push(Diagnostic {
                message: format!("duplicate struct `{}`", structure.name),
                span: None,
            });
        }
    }
    for enumeration in &program.enums {
        let variants = enumeration
            .variants
            .iter()
            .map(|variant| (variant.name.clone(), variant.payload.clone()))
            .collect();
        let max_payload = enumeration
            .variants
            .iter()
            .max_by_key(|variant| variant.payload.len())
            .map(|variant| variant.payload.clone())
            .unwrap_or_default();
        let mut fields = vec![("__tag".into(), Type::I32)];
        fields.extend(
            max_payload
                .into_iter()
                .enumerate()
                .map(|(index, ty)| (format!("__payload{index}"), ty)),
        );
        if layouts
            .insert(
                enumeration.name.clone(),
                StructLayout {
                    fields,
                    variants: Some(variants),
                },
            )
            .is_some()
        {
            errors.push(Diagnostic {
                message: format!("duplicate type `{}`", enumeration.name),
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
    signatures.insert(
        "fs_read".into(),
        FunctionSignature {
            parameters: vec![Type::String],
            result: Type::Tuple(vec![Type::String, Type::String]),
        },
    );
    signatures.insert(
        "fs_write".into(),
        FunctionSignature {
            parameters: vec![Type::String, Type::String],
            result: Type::String,
        },
    );
    for (name, parameters, result) in [
        ("auth_hash_password", vec![Type::String], Type::String),
        (
            "auth_verify_password",
            vec![Type::String, Type::String],
            Type::Bool,
        ),
        ("auth_session_open", vec![Type::I64], Type::String),
        (
            "auth_session_create",
            vec![Type::I64, Type::I64, Type::I64],
            Type::String,
        ),
        (
            "auth_session_user",
            vec![Type::I64, Type::String],
            Type::I64,
        ),
        (
            "auth_session_revoke",
            vec![Type::I64, Type::String],
            Type::String,
        ),
        (
            "auth_session_cookie_create",
            vec![Type::I64, Type::I64, Type::I64, Type::String],
            Type::String,
        ),
        (
            "auth_session_cookie_user",
            vec![Type::I64, Type::String, Type::String],
            Type::I64,
        ),
        (
            "auth_session_request_user",
            vec![Type::I64, Type::I64, Type::String],
            Type::I64,
        ),
        (
            "auth_role_grant",
            vec![Type::I64, Type::I64, Type::String],
            Type::String,
        ),
        (
            "auth_role_revoke",
            vec![Type::I64, Type::I64, Type::String],
            Type::String,
        ),
        (
            "auth_user_has_role",
            vec![Type::I64, Type::I64, Type::String],
            Type::Bool,
        ),
        (
            "auth_csrf_token",
            vec![Type::I64, Type::I64, Type::I64],
            Type::String,
        ),
        (
            "auth_csrf_verify",
            vec![Type::I64, Type::I64, Type::String],
            Type::Bool,
        ),
        ("fs_append", vec![Type::String, Type::String], Type::String),
        (
            "fs_write_bytes",
            vec![Type::String, Type::Bytes],
            Type::String,
        ),
        (
            "fs_append_bytes",
            vec![Type::String, Type::Bytes],
            Type::String,
        ),
        (
            "fs_read_bytes",
            vec![Type::String],
            Type::Tuple(vec![Type::Bytes, Type::String]),
        ),
        ("fs_remove", vec![Type::String], Type::String),
        ("fs_exists", vec![Type::String], Type::Bool),
        ("fs_is_file", vec![Type::String], Type::Bool),
        ("fs_is_dir", vec![Type::String], Type::Bool),
        (
            "fs_size",
            vec![Type::String],
            Type::Tuple(vec![Type::I64, Type::String]),
        ),
        (
            "fs_list",
            vec![Type::String],
            Type::Tuple(vec![Type::List(Box::new(Type::String)), Type::String]),
        ),
        ("fs_create_dir", vec![Type::String], Type::String),
        ("fs_remove_dir", vec![Type::String], Type::String),
        (
            "env_get",
            vec![Type::String],
            Type::Tuple(vec![Type::String, Type::Bool]),
        ),
        ("env_set", vec![Type::String, Type::String], Type::Null),
        ("env_unset", vec![Type::String], Type::Null),
        (
            "env_vars",
            vec![],
            Type::Map(Box::new(Type::String), Box::new(Type::String)),
        ),
        ("env_args", vec![], Type::List(Box::new(Type::String))),
        (
            "env_cwd",
            vec![],
            Type::Tuple(vec![Type::String, Type::String]),
        ),
        ("env_exit", vec![Type::I32], Type::Null),
        ("config_load", vec![Type::String], Type::I64),
        (
            "config_get_string",
            vec![Type::I64, Type::String, Type::String],
            Type::String,
        ),
        (
            "config_get_i32",
            vec![Type::I64, Type::String, Type::I32],
            Type::I32,
        ),
        (
            "config_get_i64",
            vec![Type::I64, Type::String, Type::I64],
            Type::I64,
        ),
        (
            "config_get_bool",
            vec![Type::I64, Type::String, Type::Bool],
            Type::Bool,
        ),
        (
            "config_required",
            vec![Type::I64, Type::String],
            Type::String,
        ),
        ("config_close", vec![Type::I64], Type::Null),
        ("db_open", vec![Type::String], Type::I64),
        (
            "db_pool_new",
            vec![Type::String, Type::I64, Type::I64],
            Type::I64,
        ),
        ("db_pool_acquire", vec![Type::I64], Type::I64),
        ("db_pool_release", vec![Type::I64, Type::I64], Type::Null),
        ("db_pool_close", vec![Type::I64], Type::Null),
        ("db_schema", vec![Type::String], Type::I64),
        (
            "db_field",
            vec![Type::I64, Type::String, Type::String],
            Type::String,
        ),
        ("db_nullable", vec![Type::I64, Type::String], Type::String),
        (
            "db_default",
            vec![Type::I64, Type::String, Type::String],
            Type::String,
        ),
        ("db_unique", vec![Type::I64, Type::String], Type::String),
        ("db_index", vec![Type::I64, Type::String], Type::String),
        (
            "db_migrate",
            vec![Type::I64, Type::String, Type::I64],
            Type::String,
        ),
        ("db_schema_close", vec![Type::I64], Type::Null),
        ("db_exec", vec![Type::I64, Type::String], Type::String),
        (
            "db_error_detail",
            vec![Type::String],
            Type::Tuple(vec![Type::String, Type::String]),
        ),
        ("db_error_status", vec![Type::String], Type::I32),
        ("db_close", vec![Type::I64], Type::Null),
        ("db_store", vec![Type::I64, Type::String], Type::I64),
        (
            "db_find",
            vec![Type::I64, Type::I64],
            Type::Tuple(vec![Type::String, Type::String]),
        ),
        (
            "db_find_map",
            vec![Type::I64, Type::I64],
            Type::Map(Box::new(Type::String), Box::new(Type::String)),
        ),
        (
            "db_find_model",
            vec![
                Type::I64,
                Type::I64,
                Type::Map(Box::new(Type::String), Box::new(Type::String)),
            ],
            Type::Map(Box::new(Type::String), Box::new(Type::String)),
        ),
        (
            "db_related",
            vec![Type::I64, Type::String, Type::I64],
            Type::List(Box::new(Type::Map(
                Box::new(Type::String),
                Box::new(Type::String),
            ))),
        ),
        (
            "db_map_i64",
            vec![
                Type::Map(Box::new(Type::String), Box::new(Type::String)),
                Type::String,
            ],
            Type::I64,
        ),
        (
            "db_map_i32",
            vec![
                Type::Map(Box::new(Type::String), Box::new(Type::String)),
                Type::String,
            ],
            Type::I32,
        ),
        (
            "db_map_bool",
            vec![
                Type::Map(Box::new(Type::String), Box::new(Type::String)),
                Type::String,
            ],
            Type::Bool,
        ),
        (
            "db_map_string",
            vec![
                Type::Map(Box::new(Type::String), Box::new(Type::String)),
                Type::String,
            ],
            Type::String,
        ),
        (
            "db_set_i32",
            vec![
                Type::Map(Box::new(Type::String), Box::new(Type::String)),
                Type::String,
                Type::I32,
            ],
            Type::Null,
        ),
        (
            "db_set_i64",
            vec![
                Type::Map(Box::new(Type::String), Box::new(Type::String)),
                Type::String,
                Type::I64,
            ],
            Type::Null,
        ),
        (
            "db_set_bool",
            vec![
                Type::Map(Box::new(Type::String), Box::new(Type::String)),
                Type::String,
                Type::Bool,
            ],
            Type::Null,
        ),
        (
            "db_set_string",
            vec![
                Type::Map(Box::new(Type::String), Box::new(Type::String)),
                Type::String,
                Type::String,
            ],
            Type::Null,
        ),
        (
            "db_validate",
            vec![
                Type::Map(Box::new(Type::String), Box::new(Type::String)),
                Type::Map(Box::new(Type::String), Box::new(Type::String)),
            ],
            Type::String,
        ),
        (
            "db_insert_validated",
            vec![
                Type::I64,
                Type::Map(Box::new(Type::String), Box::new(Type::String)),
                Type::Map(Box::new(Type::String), Box::new(Type::String)),
            ],
            Type::I64,
        ),
        (
            "db_insert_model",
            vec![
                Type::I64,
                Type::Map(Box::new(Type::String), Box::new(Type::String)),
                Type::Map(Box::new(Type::String), Box::new(Type::String)),
            ],
            Type::I64,
        ),
        (
            "db_update_validated",
            vec![
                Type::I64,
                Type::I64,
                Type::Map(Box::new(Type::String), Box::new(Type::String)),
                Type::Map(Box::new(Type::String), Box::new(Type::String)),
            ],
            Type::String,
        ),
        (
            "db_update_model",
            vec![
                Type::I64,
                Type::I64,
                Type::Map(Box::new(Type::String), Box::new(Type::String)),
                Type::Map(Box::new(Type::String), Box::new(Type::String)),
            ],
            Type::String,
        ),
        (
            "db_insert",
            vec![
                Type::I64,
                Type::Map(Box::new(Type::String), Box::new(Type::String)),
            ],
            Type::I64,
        ),
        ("db_delete", vec![Type::I64, Type::I64], Type::String),
        (
            "db_update",
            vec![
                Type::I64,
                Type::I64,
                Type::Map(Box::new(Type::String), Box::new(Type::String)),
            ],
            Type::String,
        ),
        (
            "db_all",
            vec![Type::I64],
            Type::List(Box::new(Type::Map(
                Box::new(Type::String),
                Box::new(Type::String),
            ))),
        ),
        (
            "db_filter",
            vec![Type::I64, Type::String, Type::String],
            Type::List(Box::new(Type::Map(
                Box::new(Type::String),
                Box::new(Type::String),
            ))),
        ),
        (
            "db_order",
            vec![Type::I64, Type::String, Type::Bool],
            Type::List(Box::new(Type::Map(
                Box::new(Type::String),
                Box::new(Type::String),
            ))),
        ),
        (
            "db_page",
            vec![Type::I64, Type::I64, Type::I64],
            Type::List(Box::new(Type::Map(
                Box::new(Type::String),
                Box::new(Type::String),
            ))),
        ),
        (
            "db_query",
            vec![
                Type::I64,
                Type::String,
                Type::String,
                Type::String,
                Type::Bool,
                Type::I64,
                Type::I64,
            ],
            Type::List(Box::new(Type::Map(
                Box::new(Type::String),
                Box::new(Type::String),
            ))),
        ),
        ("db_begin", vec![Type::I64], Type::String),
        ("db_commit", vec![Type::I64], Type::String),
        ("db_rollback", vec![Type::I64], Type::String),
        ("http_bind", vec![Type::String], Type::I64),
        ("http_accept", vec![Type::I64], Type::I64),
        (
            "http_read",
            vec![Type::I64],
            Type::Tuple(vec![Type::String, Type::String]),
        ),
        (
            "http_write",
            vec![Type::I64, Type::I32, Type::String],
            Type::String,
        ),
        ("http_close", vec![Type::I64], Type::Null),
        ("http_router_new", vec![], Type::I64),
        (
            "http_router_get",
            vec![
                Type::I64,
                Type::String,
                Type::Function(
                    vec![Type::String, Type::String, Type::String],
                    Box::new(Type::Tuple(vec![Type::I32, Type::String])),
                ),
            ],
            Type::String,
        ),
        (
            "http_router_post",
            vec![
                Type::I64,
                Type::String,
                Type::Function(
                    vec![Type::String, Type::String, Type::String],
                    Box::new(Type::Tuple(vec![Type::I32, Type::String])),
                ),
            ],
            Type::String,
        ),
        (
            "http_router_use",
            vec![
                Type::I64,
                Type::Function(
                    vec![Type::String, Type::String, Type::String],
                    Box::new(Type::Tuple(vec![Type::I32, Type::String])),
                ),
            ],
            Type::String,
        ),
        (
            "http_router_get_request",
            vec![
                Type::I64,
                Type::String,
                Type::Function(
                    vec![Type::I64],
                    Box::new(Type::Tuple(vec![Type::I32, Type::String])),
                ),
            ],
            Type::String,
        ),
        (
            "http_router_post_request",
            vec![
                Type::I64,
                Type::String,
                Type::Function(
                    vec![Type::I64],
                    Box::new(Type::Tuple(vec![Type::I32, Type::String])),
                ),
            ],
            Type::String,
        ),
        (
            "http_router_get_response",
            vec![
                Type::I64,
                Type::String,
                Type::Function(vec![Type::I64], Box::new(Type::I64)),
            ],
            Type::String,
        ),
        (
            "http_router_post_response",
            vec![
                Type::I64,
                Type::String,
                Type::Function(vec![Type::I64], Box::new(Type::I64)),
            ],
            Type::String,
        ),
        (
            "http_router_use_request",
            vec![
                Type::I64,
                Type::Function(vec![Type::I64], Box::new(Type::I64)),
            ],
            Type::String,
        ),
        (
            "http_router_use_validation",
            vec![
                Type::I64,
                Type::String,
                Type::String,
                Type::Map(Box::new(Type::String), Box::new(Type::I64)),
            ],
            Type::String,
        ),
        (
            "http_router_use_session",
            vec![Type::I64, Type::I64, Type::String],
            Type::String,
        ),
        (
            "http_router_use_role",
            vec![Type::I64, Type::I64, Type::String, Type::String],
            Type::String,
        ),
        (
            "http_router_use_csrf",
            vec![Type::I64, Type::I64, Type::String],
            Type::String,
        ),
        (
            "http_router_dispatch_request",
            vec![Type::I64, Type::String],
            Type::Tuple(vec![Type::I32, Type::String]),
        ),
        (
            "http_router_dispatch_request_pool",
            vec![Type::I64, Type::I64, Type::String],
            Type::Tuple(vec![Type::I32, Type::String]),
        ),
        (
            "http_router_dispatch_response",
            vec![Type::I64, Type::String],
            Type::I64,
        ),
        ("http_request_db", vec![], Type::I64),
        ("http_request_begin", vec![], Type::String),
        ("http_request_commit", vec![], Type::String),
        ("http_request_rollback", vec![], Type::String),
        ("http_request_method", vec![Type::I64], Type::String),
        ("http_request_path", vec![Type::I64], Type::String),
        ("http_request_body", vec![Type::I64], Type::String),
        (
            "http_request_header",
            vec![Type::I64, Type::String],
            Type::String,
        ),
        (
            "http_request_cookie",
            vec![Type::I64, Type::String],
            Type::String,
        ),
        (
            "http_request_query",
            vec![Type::I64, Type::String],
            Type::String,
        ),
        (
            "http_request_param",
            vec![Type::I64, Type::String, Type::String],
            Type::String,
        ),
        (
            "http_require_header",
            vec![Type::I64, Type::String],
            Type::Tuple(vec![Type::String, Type::String]),
        ),
        (
            "http_require_query",
            vec![Type::I64, Type::String],
            Type::Tuple(vec![Type::String, Type::String]),
        ),
        (
            "http_require_param",
            vec![Type::I64, Type::String, Type::String],
            Type::Tuple(vec![Type::String, Type::String]),
        ),
        (
            "http_json_validate_schema",
            vec![
                Type::String,
                Type::Map(Box::new(Type::String), Box::new(Type::String)),
            ],
            Type::String,
        ),
        (
            "http_validate",
            vec![
                Type::Map(Box::new(Type::String), Box::new(Type::String)),
                Type::Map(Box::new(Type::String), Box::new(Type::I64)),
            ],
            Type::Map(Box::new(Type::String), Box::new(Type::String)),
        ),
        (
            "http_validate_json",
            vec![
                Type::String,
                Type::Map(Box::new(Type::String), Box::new(Type::I64)),
            ],
            Type::Map(Box::new(Type::String), Box::new(Type::String)),
        ),
        (
            "http_validate_query",
            vec![
                Type::I64,
                Type::Map(Box::new(Type::String), Box::new(Type::I64)),
            ],
            Type::Map(Box::new(Type::String), Box::new(Type::String)),
        ),
        (
            "http_validate_params",
            vec![
                Type::I64,
                Type::String,
                Type::Map(Box::new(Type::String), Box::new(Type::I64)),
            ],
            Type::Map(Box::new(Type::String), Box::new(Type::String)),
        ),
        (
            "http_validate_form",
            vec![
                Type::I64,
                Type::Map(Box::new(Type::String), Box::new(Type::I64)),
            ],
            Type::Map(Box::new(Type::String), Box::new(Type::String)),
        ),
        (
            "http_validate_form_body",
            vec![
                Type::String,
                Type::Map(Box::new(Type::String), Box::new(Type::I64)),
            ],
            Type::Map(Box::new(Type::String), Box::new(Type::String)),
        ),
        (
            "http_validate_status",
            vec![Type::Map(Box::new(Type::String), Box::new(Type::String))],
            Type::I32,
        ),
        (
            "http_validation_error",
            vec![
                Type::Map(Box::new(Type::String), Box::new(Type::String)),
                Type::String,
            ],
            Type::Tuple(vec![Type::String, Type::String]),
        ),
        (
            "http_error_response",
            vec![Type::String, Type::String, Type::I32, Type::String],
            Type::I64,
        ),
        (
            "http_validation_response",
            vec![Type::Map(Box::new(Type::String), Box::new(Type::String))],
            Type::I64,
        ),
        ("http_rule_required", vec![], Type::I64),
        ("http_rule_optional", vec![], Type::I64),
        ("http_rule_string", vec![], Type::I64),
        ("http_rule_i64", vec![], Type::I64),
        ("http_rule_email", vec![], Type::I64),
        ("http_rule_min", vec![Type::I64], Type::I64),
        ("http_rule_max", vec![Type::I64], Type::I64),
        ("http_rules", vec![Type::I64], Type::I64),
        (
            "http_response_new",
            vec![Type::I32, Type::String],
            Type::I64,
        ),
        (
            "http_response_header",
            vec![Type::I64, Type::String, Type::String],
            Type::String,
        ),
        (
            "http_response_cookie",
            vec![
                Type::I64,
                Type::String,
                Type::String,
                Type::I64,
                Type::Bool,
                Type::Bool,
            ],
            Type::String,
        ),
        ("http_response_status", vec![Type::I64], Type::I32),
        ("http_response_body", vec![Type::I64], Type::String),
        (
            "http_response_header_value",
            vec![Type::I64, Type::String],
            Type::String,
        ),
        ("http_response_close", vec![Type::I64], Type::Null),
        (
            "http_router_dispatch",
            vec![Type::I64, Type::String, Type::String, Type::String],
            Type::Tuple(vec![Type::I32, Type::String]),
        ),
        ("http_router_close", vec![Type::I64], Type::Null),
        ("http_serve", vec![Type::I64, Type::String], Type::String),
        ("http_query", vec![Type::String, Type::String], Type::String),
        (
            "http_json",
            vec![Type::I32, Type::String],
            Type::Tuple(vec![Type::I32, Type::String]),
        ),
        (
            "http_json_body",
            vec![Type::String],
            Type::Tuple(vec![Type::String, Type::String]),
        ),
        ("http_json_validate", vec![Type::String], Type::String),
        ("http_json_escape", vec![Type::String], Type::String),
        (
            "http_json_object",
            vec![Type::String],
            Type::Tuple(vec![
                Type::Map(Box::new(Type::String), Box::new(Type::String)),
                Type::String,
            ]),
        ),
        (
            "http_json_string",
            vec![Type::String],
            Type::Tuple(vec![Type::String, Type::String]),
        ),
        (
            "http_json_i64",
            vec![Type::String],
            Type::Tuple(vec![Type::I64, Type::String]),
        ),
        (
            "http_json_bool",
            vec![Type::String],
            Type::Tuple(vec![Type::Bool, Type::String]),
        ),
        (
            "http_header",
            vec![Type::String, Type::String],
            Type::String,
        ),
        (
            "http_param",
            vec![Type::String, Type::String, Type::String],
            Type::String,
        ),
        ("http_server_bind", vec![Type::String], Type::I64),
        (
            "http_server_bind_config",
            vec![Type::String, Type::I64, Type::I64, Type::I64, Type::I64],
            Type::I64,
        ),
        ("http_server_shutdown", vec![Type::I64], Type::Null),
        ("http_server_run", vec![Type::I64, Type::I64], Type::String),
        (
            "http_server_run_pool",
            vec![Type::I64, Type::I64, Type::I64],
            Type::String,
        ),
        (
            "http_server_run_for",
            vec![Type::I64, Type::I64, Type::I64],
            Type::String,
        ),
        (
            "http_server_run_for_pool",
            vec![Type::I64, Type::I64, Type::I64, Type::I64],
            Type::String,
        ),
        (
            "http_server_run_workers",
            vec![Type::I64, Type::I64, Type::I64],
            Type::String,
        ),
        (
            "http_server_run_workers_pool",
            vec![Type::I64, Type::I64, Type::I64, Type::I64],
            Type::String,
        ),
    ] {
        signatures.insert(name.into(), FunctionSignature { parameters, result });
    }
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
            Stmt::ForIn {
                names: bindings,
                iterable,
                body,
            } => {
                let iterable_type = expr(iterable, &names, signatures, layouts, errors);
                let mut loop_names = names.clone();
                match (bindings.as_slice(), iterable_type) {
                    ([binding], Some(Type::List(element))) => {
                        loop_names.insert(binding.clone(), (*element, false));
                    }
                    ([binding], Some(Type::Map(key, _))) => {
                        loop_names.insert(binding.clone(), (*key, false));
                    }
                    ([key, value], Some(Type::Map(key_ty, value_ty))) => {
                        loop_names.insert(key.clone(), (*key_ty, false));
                        loop_names.insert(value.clone(), (*value_ty, false));
                    }
                    _ => errors.push(Diagnostic {
                        message: "for-in expects a list or map with one or two bindings".into(),
                        span: None,
                    }),
                }
                let _ = check_block(body, &loop_names, function, signatures, layouts, errors, 1);
            }
            Stmt::For {
                init,
                condition,
                step,
                body,
            } => {
                let mut loop_names = names.clone();
                if let Stmt::Let {
                    name,
                    mutable,
                    ty,
                    value,
                } = init.as_ref()
                {
                    let value_type = expr(value, &loop_names, signatures, layouts, errors);
                    if let Some(annotation) = ty {
                        validate_type(annotation, layouts, errors);
                    }
                    loop_names.insert(
                        name.clone(),
                        (ty.clone().or(value_type).unwrap_or(Type::I32), *mutable),
                    );
                } else {
                    errors.push(Diagnostic {
                        message: "for initializer must be a binding".into(),
                        span: None,
                    });
                }
                if expr(condition, &loop_names, signatures, layouts, errors) != Some(Type::Bool) {
                    errors.push(Diagnostic {
                        message: "for condition must be bool".into(),
                        span: None,
                    });
                }
                if !matches!(step.as_ref(), Stmt::Assign { .. }) {
                    errors.push(Diagnostic {
                        message: "for step must be an assignment".into(),
                        span: None,
                    });
                }
                let _ = check_block(
                    std::slice::from_ref(step),
                    &loop_names,
                    function,
                    signatures,
                    layouts,
                    errors,
                    1,
                );
                let _ = check_block(body, &loop_names, function, signatures, layouts, errors, 1);
            }
            Stmt::While { condition, body } => {
                if expr(condition, &names, signatures, layouts, errors) != Some(Type::Bool) {
                    errors.push(Diagnostic {
                        message: "while condition must be bool".into(),
                        span: None,
                    });
                }
                let body_flow = check_block(body, &names, function, signatures, layouts, errors, 1);
                returned |= matches!(condition, Expr::Bool(true))
                    && body_flow.returns
                    && !body_flow.breaks_loop;
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
                let then_flow =
                    check_block(then_body, &names, function, signatures, layouts, errors, 0);
                let else_flow =
                    check_block(else_body, &names, function, signatures, layouts, errors, 0);
                returned |= then_flow.returns && else_flow.returns;
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
                let value_type = typed_literal(value, ty, &names, signatures, layouts, errors);
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
            Stmt::IndexAssign { base, index, value } => {
                let base_type = expr(base, &names, signatures, layouts, errors);
                if let Some(name) = root_name(base) {
                    if !names.get(name).is_some_and(|(_, mutable)| *mutable) {
                        errors.push(Diagnostic {
                            message: format!("immutable binding `{name}`"),
                            span: None,
                        });
                    }
                }
                let index_type = expr(index, &names, signatures, layouts, errors);
                let value_type = expr(value, &names, signatures, layouts, errors);
                let expected = match base_type {
                    Some(Type::List(element)) => {
                        if index_type != Some(Type::I32) {
                            errors.push(Diagnostic {
                                message: "list index must be i32".into(),
                                span: None,
                            });
                        }
                        Some(*element)
                    }
                    Some(Type::Map(key, value)) => {
                        if index_type != Some(*key.clone()) {
                            errors.push(Diagnostic {
                                message: "map key type mismatch".into(),
                                span: None,
                            });
                        }
                        Some(*value)
                    }
                    Some(Type::Bytes) => {
                        if index_type != Some(Type::I32) {
                            errors.push(Diagnostic {
                                message: "bytes index must be i32".into(),
                                span: None,
                            });
                        }
                        Some(Type::I32)
                    }
                    _ => {
                        errors.push(Diagnostic {
                            message: "indexed assignment requires a collection".into(),
                            span: None,
                        });
                        None
                    }
                };
                if expected != value_type {
                    errors.push(Diagnostic {
                        message: "indexed assignment type mismatch".into(),
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
            Stmt::Panic { value, .. } => {
                if expr(value, &names, signatures, layouts, errors) != Some(Type::String) {
                    errors.push(Diagnostic {
                        message: "panic expects one string message".into(),
                        span: None,
                    });
                }
            }
            Stmt::TryCatch {
                body,
                binding,
                catch_body,
            } => {
                let _ = check_block(body, &names, function, signatures, layouts, errors, 0);
                let mut catch_names = names.clone();
                catch_names.insert(binding.clone(), (Type::Named("Error".into()), false));
                let catch_flow = check_block(
                    catch_body,
                    &catch_names,
                    function,
                    signatures,
                    layouts,
                    errors,
                    0,
                );
                returned |= catch_flow.returns;
            }
            Stmt::Expr(value) => {
                let _ = expr(value, &names, signatures, layouts, errors);
            }
            Stmt::Stop => {
                errors.push(Diagnostic {
                    message: "stop outside a loop".into(),
                    span: None,
                });
            }
            Stmt::Skip => {
                errors.push(Diagnostic {
                    message: "skip outside a loop".into(),
                    span: None,
                });
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
    loop_depth: usize,
) -> Flow {
    let mut names = inherited.clone();
    let mut declared = HashSet::new();
    let mut flow = Flow {
        can_fall_through: true,
        ..Flow::default()
    };
    for statement in statements {
        if !flow.can_fall_through {
            break;
        }
        match statement {
            Stmt::ForIn {
                names: bindings,
                iterable,
                body,
            } => {
                let iterable_type = expr(iterable, &names, signatures, layouts, errors);
                let mut loop_names = names.clone();
                match (bindings.as_slice(), iterable_type) {
                    ([binding], Some(Type::List(element))) => {
                        loop_names.insert(binding.clone(), (*element, false));
                    }
                    ([binding], Some(Type::Map(key, _))) => {
                        loop_names.insert(binding.clone(), (*key, false));
                    }
                    ([key, value], Some(Type::Map(key_ty, value_ty))) => {
                        loop_names.insert(key.clone(), (*key_ty, false));
                        loop_names.insert(value.clone(), (*value_ty, false));
                    }
                    _ => errors.push(Diagnostic {
                        message: "for-in expects a list or map with one or two bindings".into(),
                        span: None,
                    }),
                }
                let _ = check_block(
                    body,
                    &loop_names,
                    function,
                    signatures,
                    layouts,
                    errors,
                    loop_depth + 1,
                );
            }
            Stmt::For {
                init,
                condition,
                step,
                body,
            } => {
                let mut loop_names = names.clone();
                if let Stmt::Let {
                    name,
                    mutable,
                    ty,
                    value,
                } = init.as_ref()
                {
                    let value_type = expr(value, &loop_names, signatures, layouts, errors);
                    if let Some(annotation) = ty {
                        validate_type(annotation, layouts, errors);
                    }
                    loop_names.insert(
                        name.clone(),
                        (ty.clone().or(value_type).unwrap_or(Type::I32), *mutable),
                    );
                } else {
                    errors.push(Diagnostic {
                        message: "for initializer must be a binding".into(),
                        span: None,
                    });
                }
                if expr(condition, &loop_names, signatures, layouts, errors) != Some(Type::Bool) {
                    errors.push(Diagnostic {
                        message: "for condition must be bool".into(),
                        span: None,
                    });
                }
                if !matches!(step.as_ref(), Stmt::Assign { .. }) {
                    errors.push(Diagnostic {
                        message: "for step must be an assignment".into(),
                        span: None,
                    });
                }
                let _ = check_block(
                    std::slice::from_ref(step),
                    &loop_names,
                    function,
                    signatures,
                    layouts,
                    errors,
                    loop_depth + 1,
                );
                let _ = check_block(
                    body,
                    &loop_names,
                    function,
                    signatures,
                    layouts,
                    errors,
                    loop_depth + 1,
                );
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
                let value_type = typed_literal(value, ty, &names, signatures, layouts, errors);
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
            Stmt::IndexAssign { base, index, value } => {
                let base_type = expr(base, &names, signatures, layouts, errors);
                if let Some(name) = root_name(base) {
                    if !names.get(name).is_some_and(|(_, mutable)| *mutable) {
                        errors.push(Diagnostic {
                            message: format!("immutable binding `{name}`"),
                            span: None,
                        });
                    }
                }
                let index_type = expr(index, &names, signatures, layouts, errors);
                let value_type = expr(value, &names, signatures, layouts, errors);
                let expected = match base_type {
                    Some(Type::List(element)) => {
                        if index_type != Some(Type::I32) {
                            errors.push(Diagnostic {
                                message: "list index must be i32".into(),
                                span: None,
                            });
                        }
                        Some(*element)
                    }
                    Some(Type::Map(key, value)) => {
                        if index_type != Some(*key.clone()) {
                            errors.push(Diagnostic {
                                message: "map key type mismatch".into(),
                                span: None,
                            });
                        }
                        Some(*value)
                    }
                    Some(Type::Bytes) => {
                        if index_type != Some(Type::I32) {
                            errors.push(Diagnostic {
                                message: "bytes index must be i32".into(),
                                span: None,
                            });
                        }
                        Some(Type::I32)
                    }
                    _ => None,
                };
                if expected != value_type {
                    errors.push(Diagnostic {
                        message: "indexed assignment type mismatch".into(),
                        span: None,
                    });
                }
            }
            Stmt::Return(value) => {
                flow.returns = true;
                flow.can_fall_through = false;
                if expr(value, &names, signatures, layouts, errors) != Some(function.result.clone())
                {
                    errors.push(Diagnostic {
                        message: format!("return type mismatch in `{}`", function.name),
                        span: None,
                    });
                }
            }
            Stmt::Panic { value, .. } => {
                if expr(value, &names, signatures, layouts, errors) != Some(Type::String) {
                    errors.push(Diagnostic {
                        message: "panic expects one string message".into(),
                        span: None,
                    });
                }
            }
            Stmt::TryCatch {
                body,
                binding,
                catch_body,
            } => {
                let _ = check_block(
                    body, &names, function, signatures, layouts, errors, loop_depth,
                );
                let mut catch_names = names.clone();
                catch_names.insert(binding.clone(), (Type::Named("Error".into()), false));
                let catch_flow = check_block(
                    catch_body,
                    &catch_names,
                    function,
                    signatures,
                    layouts,
                    errors,
                    loop_depth,
                );
                if catch_flow.returns {
                    flow.returns = true;
                }
            }
            Stmt::Expr(value) => {
                let _ = expr(value, &names, signatures, layouts, errors);
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
                let then_flow = check_block(
                    then_body, &names, function, signatures, layouts, errors, loop_depth,
                );
                let else_flow = check_block(
                    else_body, &names, function, signatures, layouts, errors, loop_depth,
                );
                flow.returns = then_flow.returns && else_flow.returns;
                flow.breaks_loop |= then_flow.breaks_loop || else_flow.breaks_loop;
                flow.can_fall_through = then_flow.can_fall_through || else_flow.can_fall_through;
            }
            Stmt::While { condition, body } => {
                if expr(condition, &names, signatures, layouts, errors) != Some(Type::Bool) {
                    errors.push(Diagnostic {
                        message: "while condition must be bool".into(),
                        span: None,
                    });
                }
                let body_flow = check_block(
                    body,
                    &names,
                    function,
                    signatures,
                    layouts,
                    errors,
                    loop_depth + 1,
                );
                flow.returns = matches!(condition, Expr::Bool(true))
                    && body_flow.returns
                    && !body_flow.breaks_loop;
                flow.can_fall_through =
                    !matches!(condition, Expr::Bool(true)) || body_flow.breaks_loop;
            }
            Stmt::Stop => {
                if loop_depth == 0 {
                    errors.push(Diagnostic {
                        message: "stop outside a loop".into(),
                        span: None,
                    });
                } else {
                    flow.breaks_loop = true;
                    flow.can_fall_through = false;
                }
            }
            Stmt::Skip => {
                if loop_depth == 0 {
                    errors.push(Diagnostic {
                        message: "skip outside a loop".into(),
                        span: None,
                    });
                } else {
                    flow.can_fall_through = false;
                }
            }
        }
    }
    flow
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
        Expr::Float(_) => Some(Type::Float),
        Expr::Bool(_) => Some(Type::Bool),
        Expr::Null => Some(Type::Null),
        Expr::Tuple(elements) => Some(Type::Tuple(
            elements
                .iter()
                .filter_map(|element| expr(element, names, signatures, layouts, errors))
                .collect(),
        )),
        Expr::String(_) => Some(Type::String),
        Expr::Interpolated(parts, _) => {
            for part in parts {
                if let InterpolationPart::Name(name) = part {
                    if names.get(name).map(|(ty, _)| ty) != Some(&Type::String) {
                        errors.push(Diagnostic {
                            message: format!("interpolation `{name}` requires a string"),
                            span: None,
                        });
                    }
                }
            }
            Some(Type::String)
        }
        Expr::Bytes(_, _) => Some(Type::Bytes),
        Expr::Closure {
            params,
            result,
            body,
            ..
        } => {
            let mut closure_names = names.clone();
            let mut duplicate = HashSet::new();
            for parameter in params {
                if !duplicate.insert(parameter.name.clone()) {
                    errors.push(Diagnostic {
                        message: format!("duplicate closure parameter `{}`", parameter.name),
                        span: None,
                    });
                }
                closure_names.insert(parameter.name.clone(), (parameter.ty.clone(), false));
                validate_type(&parameter.ty, layouts, errors);
            }
            let mut captured = HashSet::new();
            collect_expr_names(body, &mut captured);
            for parameter in params {
                captured.remove(&parameter.name);
            }
            for name in captured {
                if names.get(&name).is_some_and(|(_, mutable)| *mutable) {
                    errors.push(Diagnostic {
                        message: format!("closures cannot capture mutable binding `{name}`"),
                        span: None,
                    });
                }
            }
            validate_type(result, layouts, errors);
            let body_type = expr(body, &closure_names, signatures, layouts, errors);
            if body_type != Some(result.clone()) {
                errors.push(Diagnostic {
                    message: "closure body does not match its result type".into(),
                    span: None,
                });
            }
            Some(Type::Function(
                params
                    .iter()
                    .map(|parameter| parameter.ty.clone())
                    .collect(),
                Box::new(result.clone()),
            ))
        }
        Expr::List(elements, _) => {
            let inferred: Vec<_> = elements
                .iter()
                .map(|element| expr(element, names, signatures, layouts, errors))
                .collect();
            let first = inferred.first().cloned().flatten();
            if first.is_none() {
                errors.push(Diagnostic {
                    message: "list literals cannot be empty without a type".into(),
                    span: None,
                });
            }
            if inferred.iter().any(|ty| *ty != first) {
                errors.push(Diagnostic {
                    message: "list elements must have the same type".into(),
                    span: None,
                });
            }
            first.map(|ty| Type::List(Box::new(ty)))
        }
        Expr::Map(pairs, _) => {
            let keys: Vec<_> = pairs
                .iter()
                .map(|(key, _)| expr(key, names, signatures, layouts, errors))
                .collect();
            let values: Vec<_> = pairs
                .iter()
                .map(|(_, value)| expr(value, names, signatures, layouts, errors))
                .collect();
            let key = keys.first().cloned().flatten();
            let value = values.first().cloned().flatten();
            if key.is_none() || value.is_none() {
                errors.push(Diagnostic {
                    message: "map literals cannot be empty without a type".into(),
                    span: None,
                });
            }
            if keys.iter().any(|ty| *ty != key) || values.iter().any(|ty| *ty != value) {
                errors.push(Diagnostic {
                    message: "map keys and values must each have the same type".into(),
                    span: None,
                });
            }
            if key != Some(Type::String) {
                errors.push(Diagnostic {
                    message: "map keys must be string".into(),
                    span: None,
                });
            }
            match (key, value) {
                (Some(key), Some(value)) => Some(Type::Map(Box::new(key), Box::new(value))),
                _ => None,
            }
        }
        Expr::EnumVariant {
            enum_name,
            variant,
            arguments,
        } => {
            let Some(layout) = layouts.get(enum_name) else {
                errors.push(Diagnostic {
                    message: format!("unknown enum `{enum_name}`"),
                    span: None,
                });
                return None;
            };
            let Some(variants) = &layout.variants else {
                errors.push(Diagnostic {
                    message: format!("`{enum_name}` is not an enum"),
                    span: None,
                });
                return None;
            };
            let Some((_, payload)) = variants.iter().find(|(name, _)| name == variant) else {
                errors.push(Diagnostic {
                    message: format!("enum `{enum_name}` has no variant `{variant}`"),
                    span: None,
                });
                return None;
            };
            if arguments.len() != payload.len() {
                errors.push(Diagnostic {
                    message: format!(
                        "variant `{enum_name}::{variant}` expects {} payload values",
                        payload.len()
                    ),
                    span: None,
                });
            }
            for (argument, expected) in arguments.iter().zip(payload) {
                if expr(argument, names, signatures, layouts, errors) != Some(expected.clone()) {
                    errors.push(Diagnostic {
                        message: format!("variant `{enum_name}::{variant}` payload type mismatch"),
                        span: None,
                    });
                }
            }
            Some(Type::Named(enum_name.clone()))
        }
        Expr::Match { subject, arms } => {
            let subject_type = expr(subject, names, signatures, layouts, errors);
            let Some(Type::Named(enum_name)) = subject_type else {
                errors.push(Diagnostic {
                    message: "match subject must be an enum".into(),
                    span: None,
                });
                return None;
            };
            let Some(layout) = layouts.get(&enum_name) else {
                return None;
            };
            let Some(variants) = &layout.variants else {
                return None;
            };
            let mut seen = HashSet::new();
            let mut result: Option<Type> = None;
            for arm in arms {
                let mut arm_names = names.clone();
                if let Some(variant) = &arm.variant {
                    let Some((_, payload)) = variants.iter().find(|(name, _)| name == variant)
                    else {
                        errors.push(Diagnostic {
                            message: format!("enum `{enum_name}` has no variant `{variant}`"),
                            span: None,
                        });
                        continue;
                    };
                    seen.insert(variant.clone());
                    if let Some(binding) = &arm.binding {
                        if payload.len() != 1 {
                            errors.push(Diagnostic {
                                message: "variant binding requires exactly one payload".into(),
                                span: None,
                            });
                        } else {
                            arm_names.insert(binding.clone(), (payload[0].clone(), false));
                        }
                    }
                }
                let arm_type = expr(&arm.body, &arm_names, signatures, layouts, errors);
                if let Some(previous) = &result {
                    if Some(previous.clone()) != arm_type {
                        errors.push(Diagnostic {
                            message: "match arms must have the same type".into(),
                            span: None,
                        });
                    }
                } else {
                    result = arm_type;
                }
            }
            if !arms.iter().any(|arm| arm.variant.is_none())
                && variants.iter().any(|(name, _)| !seen.contains(name))
            {
                errors.push(Diagnostic {
                    message: format!("match on `{enum_name}` is not exhaustive"),
                    span: None,
                });
            }
            result
        }
        Expr::Neg(value) => {
            let ty = expr(value, names, signatures, layouts, errors);
            if !matches!(ty, Some(Type::I32 | Type::I64 | Type::Float)) {
                errors.push(Diagnostic {
                    message: "unary `-` requires an integer".into(),
                    span: None,
                });
            } else if constant_negation_overflows(value, ty.as_ref()) {
                errors.push(Diagnostic {
                    message: "integer overflow".into(),
                    span: None,
                });
            }
            ty
        }
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
                signatures.get(name).map(|signature| {
                    Type::Function(
                        signature.parameters.clone(),
                        Box::new(signature.result.clone()),
                    )
                })
            })
            .or_else(|| {
                errors.push(Diagnostic {
                    message: format!("unknown name `{name}`"),
                    span: None,
                });
                None
            }),
        Expr::Call {
            name, arguments, ..
        } => {
            if let Some((Type::Function(parameters, result), _)) = names.get(name) {
                if arguments.len() != parameters.len() {
                    errors.push(Diagnostic {
                        message: format!(
                            "function value `{name}` expects {} arguments",
                            parameters.len()
                        ),
                        span: None,
                    });
                }
                for (argument, expected) in arguments.iter().zip(parameters) {
                    if expr(argument, names, signatures, layouts, errors) != Some(expected.clone())
                    {
                        errors.push(Diagnostic {
                            message: format!("function value `{name}` argument type mismatch"),
                            span: None,
                        });
                    }
                }
                return Some((**result).clone());
            }
            if name == "echo" {
                if arguments.len() != 1
                    || expr(&arguments[0], names, signatures, layouts, errors) != Some(Type::String)
                {
                    errors.push(Diagnostic {
                        message: "echo expects one string".into(),
                        span: None,
                    });
                }
                return Some(Type::I32);
            }
            if name == "len" {
                if arguments.len() != 1 {
                    errors.push(Diagnostic {
                        message: "len expects one argument".into(),
                        span: None,
                    });
                    return Some(Type::I32);
                }
                let argument_type = expr(&arguments[0], names, signatures, layouts, errors);
                if !matches!(
                    argument_type,
                    Some(Type::String | Type::Bytes | Type::List(_) | Type::Map(_, _))
                ) {
                    errors.push(Diagnostic {
                        message: "len expects string, bytes, list, or map".into(),
                        span: None,
                    });
                }
                return Some(Type::I32);
            }
            if matches!(
                name.as_str(),
                "required" | "optional" | "string" | "i64" | "email"
            ) {
                if !arguments.is_empty() {
                    errors.push(Diagnostic {
                        message: format!("rule `{name}` takes no arguments"),
                        span: None,
                    });
                }
                return Some(Type::I64);
            }
            if name == "min" || name == "max" {
                if arguments.len() != 1
                    || expr(&arguments[0], names, signatures, layouts, errors) != Some(Type::I32)
                        && expr(&arguments[0], names, signatures, layouts, errors)
                            != Some(Type::I64)
                {
                    errors.push(Diagnostic {
                        message: format!("rule `{name}` expects one integer"),
                        span: None,
                    });
                }
                return Some(Type::I64);
            }
            if name == "rules" {
                for argument in arguments {
                    expr(argument, names, signatures, layouts, errors);
                }
                return Some(Type::I64);
            }
            if name == "validate" {
                if arguments.len() != 2 {
                    errors.push(Diagnostic {
                        message: "validate expects input and schema".into(),
                        span: None,
                    });
                    return Some(Type::Map(Box::new(Type::String), Box::new(Type::String)));
                }
                let input = expr(&arguments[0], names, signatures, layouts, errors);
                let schema = expr(&arguments[1], names, signatures, layouts, errors);
                if input != Some(Type::Map(Box::new(Type::String), Box::new(Type::String))) {
                    errors.push(Diagnostic {
                        message: "validate input must be map<string, string>".into(),
                        span: None,
                    });
                }
                if schema != Some(Type::Map(Box::new(Type::String), Box::new(Type::I64))) {
                    errors.push(Diagnostic {
                        message: "validate schema must map field names to rules".into(),
                        span: None,
                    });
                }
                return Some(Type::Map(Box::new(Type::String), Box::new(Type::String)));
            }
            if name == "append" {
                if arguments.len() != 2 {
                    errors.push(Diagnostic {
                        message: "append expects a list and one value".into(),
                        span: None,
                    });
                    return None;
                }
                let list_type = expr(&arguments[0], names, signatures, layouts, errors);
                let value_type = expr(&arguments[1], names, signatures, layouts, errors);
                match list_type {
                    Some(Type::List(element)) if Some(*element.clone()) == value_type => {
                        Some(Type::List(element))
                    }
                    Some(Type::List(_)) => {
                        errors.push(Diagnostic {
                            message: "append value type mismatch".into(),
                            span: None,
                        });
                        None
                    }
                    _ => {
                        errors.push(Diagnostic {
                            message: "append expects a list".into(),
                            span: None,
                        });
                        None
                    }
                }
            } else {
                if name == "remove" {
                    if arguments.len() != 2 {
                        errors.push(Diagnostic {
                            message: "remove expects a map and a string key".into(),
                            span: None,
                        });
                        return Some(Type::I32);
                    }
                    let map_type = expr(&arguments[0], names, signatures, layouts, errors);
                    let key_type = expr(&arguments[1], names, signatures, layouts, errors);
                    if !matches!(map_type, Some(Type::Map(_, _))) {
                        errors.push(Diagnostic {
                            message: "remove expects a map".into(),
                            span: None,
                        });
                    }
                    if key_type != Some(Type::String) {
                        errors.push(Diagnostic {
                            message: "remove key must be string".into(),
                            span: None,
                        });
                    }
                    return Some(Type::I32);
                }
                if name == "contains" {
                    if arguments.len() != 2 {
                        errors.push(Diagnostic {
                            message: "contains expects a map and a string key".into(),
                            span: None,
                        });
                        return Some(Type::Bool);
                    }
                    let map_type = expr(&arguments[0], names, signatures, layouts, errors);
                    let key_type = expr(&arguments[1], names, signatures, layouts, errors);
                    if !matches!(map_type, Some(Type::Map(_, _))) {
                        errors.push(Diagnostic {
                            message: "contains expects a map".into(),
                            span: None,
                        });
                    }
                    if key_type != Some(Type::String) {
                        errors.push(Diagnostic {
                            message: "contains key must be string".into(),
                            span: None,
                        });
                    }
                    return Some(Type::Bool);
                }
                if let Some((receiver, method)) = name.split_once("__") {
                    if (method == "find"
                        || method == "all"
                        || method == "filter"
                        || method == "related"
                        || method == "order"
                        || method == "page"
                        || method == "query")
                        && layouts.contains_key(receiver)
                    {
                        let expected = if method == "find" {
                            2
                        } else if method == "all" {
                            1
                        } else if method == "query" {
                            7
                        } else {
                            3
                        };
                        if arguments.len() != expected {
                            errors.push(Diagnostic {
                                message: format!(
                                    "model `{receiver}.{method}` expects {expected} arguments"
                                ),
                                span: None,
                            });
                        }
                        for argument in arguments {
                            expr(argument, names, signatures, layouts, errors);
                        }
                        if method == "find" {
                            return Some(Type::Named(receiver.to_string()));
                        }
                        return Some(Type::List(Box::new(Type::Named(receiver.to_string()))));
                    }
                }
                let Some(signature) = signatures.get(name) else {
                    if let Some((receiver, method)) = name.split_once("__") {
                        if let (Some((receiver_type, _)), Some(method_signature)) =
                            (names.get(receiver), signatures.get(method))
                        {
                            let mut argument_types = vec![Some(receiver_type.clone())];
                            argument_types.extend(arguments.iter().map(|argument| {
                                expr(argument, names, signatures, layouts, errors)
                            }));
                            if argument_types.len() != method_signature.parameters.len() {
                                errors.push(Diagnostic {
                                    message: format!(
                                        "method `{method}` expects {} arguments including receiver",
                                        method_signature.parameters.len()
                                    ),
                                    span: None,
                                });
                            }
                            for (actual, expected) in
                                argument_types.iter().zip(&method_signature.parameters)
                            {
                                if *actual != Some(expected.clone()) {
                                    errors.push(Diagnostic {
                                        message: format!(
                                            "method `{method}` argument type mismatch"
                                        ),
                                        span: None,
                                    });
                                }
                            }
                            return Some(method_signature.result.clone());
                        }
                    }
                    errors.push(Diagnostic {
                        message: format!("unknown function `{name}`"),
                        span: None,
                    });
                    for argument in arguments {
                        expr(argument, names, signatures, layouts, errors);
                    }
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
                let argument_types: Vec<_> = arguments
                    .iter()
                    .map(|argument| expr(argument, names, signatures, layouts, errors))
                    .collect();
                for (argument_type, parameter_type) in
                    argument_types.iter().zip(&signature.parameters)
                {
                    if *argument_type != Some(parameter_type.clone()) {
                        errors.push(Diagnostic {
                            message: format!("argument type does not match `{name}`"),
                            span: None,
                        });
                    }
                }
                Some(signature.result.clone())
            }
        }
        Expr::StructLiteral { name, fields } => {
            let Some(layout) = layouts.get(name) else {
                errors.push(Diagnostic {
                    message: format!("unknown struct `{name}`"),
                    span: None,
                });
                for (_, value) in fields {
                    expr(value, names, signatures, layouts, errors);
                }
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
            let field_types: Vec<_> = fields
                .iter()
                .map(|(_, value)| expr(value, names, signatures, layouts, errors))
                .collect();
            for (field, ty) in &layout.fields {
                match fields.iter().position(|(given, _)| given == field) {
                    Some(index) if field_types[index] == Some(ty.clone()) => {}
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
        Expr::Index { base, index, .. } => match (
            expr(base, names, signatures, layouts, errors),
            expr(index, names, signatures, layouts, errors),
        ) {
            (Some(Type::List(element)), Some(Type::I32)) => Some(*element),
            (Some(Type::Map(key, value)), Some(index)) if *key == index => Some(*value),
            (Some(Type::Tuple(elements)), Some(Type::I32)) => {
                let Expr::Int(index, _) = index.as_ref() else {
                    errors.push(Diagnostic {
                        message: "tuple index must be a constant integer".into(),
                        span: None,
                    });
                    return None;
                };
                if *index < 0 {
                    return None;
                }
                elements.get(*index as usize).cloned().or_else(|| {
                    errors.push(Diagnostic {
                        message: "tuple index out of bounds".into(),
                        span: None,
                    });
                    None
                })
            }
            (Some(Type::Bytes), Some(Type::I32)) => Some(Type::I32),
            (Some(Type::List(_)), _) | (Some(Type::Bytes), _) => {
                errors.push(Diagnostic {
                    message: "collection index must be i32".into(),
                    span: None,
                });
                None
            }
            (Some(Type::Map(_, _)), _) => {
                errors.push(Diagnostic {
                    message: "map key type mismatch".into(),
                    span: None,
                });
                None
            }
            _ => {
                errors.push(Diagnostic {
                    message: "indexing requires a collection".into(),
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
                && !matches!(left, Some(Type::I32 | Type::I64 | Type::Float))
            {
                errors.push(Diagnostic {
                    message: "ordering comparisons require integer operands".into(),
                    span: None,
                });
            }
            Some(Type::Bool)
        }
        Expr::Binary(left, operator, right, _) => {
            let left_type = expr(left, names, signatures, layouts, errors);
            let right_type = expr(right, names, signatures, layouts, errors);
            if left_type != right_type {
                errors.push(Diagnostic {
                    message: "numeric operands must have same type".into(),
                    span: None,
                });
            } else if !matches!(left_type, Some(Type::I32 | Type::I64 | Type::Float)) {
                errors.push(Diagnostic {
                    message: "arithmetic operands require integers".into(),
                    span: None,
                });
            } else if *operator == '/' && matches!(right.as_ref(), Expr::Int(0, _)) {
                errors.push(Diagnostic {
                    message: "division by zero".into(),
                    span: None,
                });
            } else if constant_arithmetic_overflows(left, *operator, right, left_type.as_ref()) {
                errors.push(Diagnostic {
                    message: "integer overflow".into(),
                    span: None,
                });
            }
            left_type
        }
    }
}

fn typed_literal(
    value: &Expr,
    annotation: &Option<Type>,
    names: &HashMap<String, (Type, bool)>,
    signatures: &HashMap<String, FunctionSignature>,
    layouts: &HashMap<String, StructLayout>,
    errors: &mut Vec<Diagnostic>,
) -> Option<Type> {
    match (annotation, value) {
        (Some(Type::List(element)), Expr::List(elements, _)) if elements.is_empty() => {
            Some(Type::List(element.clone()))
        }
        (Some(Type::Map(key, value)), Expr::Map(pairs, _)) if pairs.is_empty() => {
            Some(Type::Map(key.clone(), value.clone()))
        }
        _ => expr(value, names, signatures, layouts, errors),
    }
}

fn root_name(expression: &Expr) -> Option<&str> {
    match expression {
        Expr::Name(name) => Some(name),
        Expr::Index { base, .. } => root_name(base),
        _ => None,
    }
}

fn collect_expr_names(expression: &Expr, names: &mut HashSet<String>) {
    match expression {
        Expr::Name(name) => {
            names.insert(name.clone());
        }
        Expr::Closure { body, .. } => collect_expr_names(body, names),
        Expr::Call { arguments, .. } | Expr::Tuple(arguments) | Expr::List(arguments, _) => {
            for argument in arguments {
                collect_expr_names(argument, names);
            }
        }
        Expr::Map(pairs, _) => {
            for (key, value) in pairs {
                collect_expr_names(key, names);
                collect_expr_names(value, names);
            }
        }
        Expr::EnumVariant { arguments, .. } => {
            for argument in arguments {
                collect_expr_names(argument, names);
            }
        }
        Expr::Match { subject, arms } => {
            collect_expr_names(subject, names);
            for arm in arms {
                collect_expr_names(&arm.body, names);
            }
        }
        Expr::StructLiteral { fields, .. } => {
            for (_, value) in fields {
                collect_expr_names(value, names);
            }
        }
        Expr::Field { base, .. } => collect_expr_names(base, names),
        Expr::Index { base, index, .. } => {
            collect_expr_names(base, names);
            collect_expr_names(index, names);
        }
        Expr::Neg(value) | Expr::Not(value) => collect_expr_names(value, names),
        Expr::Logical(left, _, right)
        | Expr::Compare(left, _, right)
        | Expr::Binary(left, _, right, _) => {
            collect_expr_names(left, names);
            collect_expr_names(right, names);
        }
        _ => {}
    }
}

fn constant_arithmetic_overflows(
    left: &Expr,
    operator: char,
    right: &Expr,
    ty: Option<&Type>,
) -> bool {
    let (Expr::Int(left, _), Expr::Int(right, _)) = (left, right) else {
        return false;
    };
    match ty {
        Some(Type::I32) => {
            let (Ok(left), Ok(right)) = (i32::try_from(*left), i32::try_from(*right)) else {
                return true;
            };
            match operator {
                '+' => left.checked_add(right),
                '-' => left.checked_sub(right),
                '*' => left.checked_mul(right),
                '/' => left.checked_div(right),
                _ => Some(left),
            }
            .is_none()
        }
        Some(Type::I64) => match operator {
            '+' => left.checked_add(*right),
            '-' => left.checked_sub(*right),
            '*' => left.checked_mul(*right),
            '/' => left.checked_div(*right),
            _ => Some(*left),
        }
        .is_none(),
        _ => false,
    }
}

fn constant_negation_overflows(value: &Expr, ty: Option<&Type>) -> bool {
    let Expr::Int(value, _) = value else {
        return false;
    };
    match ty {
        Some(Type::I32) => {
            i32::try_from(*value).is_err() || (*value as i32).checked_neg().is_none()
        }
        Some(Type::I64) => value.checked_neg().is_none(),
        _ => false,
    }
}
