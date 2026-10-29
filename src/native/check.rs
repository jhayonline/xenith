use super::syntax::*;
use std::collections::HashMap;
#[derive(Debug, Clone)]
pub struct TypedProgram(pub Program);
pub fn check(program: &Program) -> Result<TypedProgram, Vec<Diagnostic>> {
    let mains: Vec<_> = program
        .functions
        .iter()
        .filter(|f| f.name == "main")
        .collect();
    let mut e = vec![];
    if mains.len() != 1 {
        e.push(Diagnostic {
            message: "expected exactly one main".into(),
        });
        return Err(e);
    }
    let f = mains[0];
    if !f.params.is_empty() {
        e.push(Diagnostic {
            message: "main takes no parameters".into(),
        })
    }
    if f.result != Type::I32 {
        e.push(Diagnostic {
            message: "main must return -> i32".into(),
        })
    }
    let mut names = HashMap::new();
    let mut returned = false;
    for s in &f.body {
        match s {
            Stmt::Let {
                name,
                mutable,
                ty,
                value,
            } => {
                let t = expr(value, &names, &mut e);
                if let (Some(a), Some(b)) = (ty, t.clone()) {
                    if *a != b {
                        e.push(Diagnostic {
                            message: "type mismatch".into(),
                        })
                    }
                }
                names.insert(
                    name.clone(),
                    (ty.clone().or(t).unwrap_or(Type::I32), *mutable),
                );
            }
            Stmt::Assign { name, value } => {
                if let Some((t, m)) = names.get(name).cloned() {
                    if !m {
                        e.push(Diagnostic {
                            message: "immutable binding".into(),
                        })
                    }
                    if let Some(v) = expr(value, &names, &mut e) {
                        if v != t {
                            e.push(Diagnostic {
                                message: "type mismatch".into(),
                            })
                        }
                    }
                } else {
                    e.push(Diagnostic {
                        message: "unknown name".into(),
                    })
                }
            }
            Stmt::Return(v) => {
                returned = true;
                if expr(v, &names, &mut e) != Some(Type::I32) {
                    e.push(Diagnostic {
                        message: "return must be i32".into(),
                    })
                }
            }
        }
    }
    if !returned {
        e.push(Diagnostic {
            message: "main must return".into(),
        })
    }
    if e.is_empty() {
        Ok(TypedProgram(program.clone()))
    } else {
        Err(e)
    }
}
fn expr(x: &Expr, n: &HashMap<String, (Type, bool)>, e: &mut Vec<Diagnostic>) -> Option<Type> {
    match x {
        Expr::Int(_, t) => Some(t.clone()),
        Expr::Name(s) => n.get(s).map(|x| x.0.clone()).or_else(|| {
            e.push(Diagnostic {
                message: "unknown name".into(),
            });
            None
        }),
        Expr::Call { .. } => {
            e.push(Diagnostic {
                message: "function calls are not yet supported".into(),
            });
            None
        }
        Expr::Binary(a, _, b) => {
            let x = expr(a, n, e);
            let y = expr(b, n, e);
            if x != y {
                e.push(Diagnostic {
                    message: "numeric operands must have same type".into(),
                })
            }
            x
        }
    }
}
