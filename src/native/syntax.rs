use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    I32,
    I64,
    Bool,
    Named(String),
}
#[derive(Debug, Clone)]
pub struct Program {
    pub structs: Vec<Struct>,
    pub functions: Vec<Function>,
}
#[derive(Debug, Clone)]
pub struct Struct {
    pub name: String,
    pub fields: Vec<Field>,
}
#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub ty: Type,
}
#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub params: Vec<Parameter>,
    pub result: Type,
    pub body: Vec<Stmt>,
}
#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub ty: Type,
}
#[derive(Debug, Clone)]
pub enum Stmt {
    If {
        condition: Expr,
        then_body: Vec<Stmt>,
        else_body: Vec<Stmt>,
    },
    Let {
        name: String,
        mutable: bool,
        ty: Option<Type>,
        value: Expr,
    },
    Assign {
        name: String,
        value: Expr,
    },
    Return(Expr),
}
#[derive(Debug, Clone)]
pub enum Expr {
    Int(i64, Type),
    Name(String),
    Call {
        name: String,
        arguments: Vec<Expr>,
    },
    StructLiteral {
        name: String,
        fields: Vec<(String, Expr)>,
    },
    Field {
        base: Box<Expr>,
        name: String,
    },
    Compare(Box<Expr>, String, Box<Expr>),
    Binary(Box<Expr>, char, Box<Expr>),
}
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub message: String,
}
fn lex(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut word = String::new();
    for c in source.chars() {
        if c.is_whitespace() {
            if !word.is_empty() {
                out.push(std::mem::take(&mut word));
            }
        } else if c == '>' && word == "-" {
            word.push(c);
            out.push(std::mem::take(&mut word));
        } else if "{}(),:;=+-*/.<>!".contains(c) {
            if !word.is_empty() {
                out.push(std::mem::take(&mut word));
            }
            if c == '-' {
                word.push(c);
            } else {
                out.push(c.to_string());
            }
        } else {
            word.push(c);
        }
    }
    if !word.is_empty() {
        out.push(word)
    }
    out
}
pub fn parse(_file: &Path, source: &str) -> Result<Program, Vec<Diagnostic>> {
    let tokens = lex(source);
    let mut p = Parser { tokens, i: 0 };
    p.program()
}
struct Parser {
    tokens: Vec<String>,
    i: usize,
}
impl Parser {
    fn take(&mut self) -> Option<String> {
        let x = self.tokens.get(self.i).cloned();
        self.i += 1;
        x
    }
    fn need(&mut self, s: &str) -> Result<(), Vec<Diagnostic>> {
        if self.take().as_deref() == Some(s) {
            Ok(())
        } else {
            Err(vec![Diagnostic {
                message: format!("expected `{s}`"),
            }])
        }
    }
    fn ty(&mut self) -> Result<Type, Vec<Diagnostic>> {
        match self.take().as_deref() {
            Some("i32") => Ok(Type::I32),
            Some("i64") => Ok(Type::I64),
            Some(name) => Ok(Type::Named(name.to_owned())),
            None => Err(vec![Diagnostic {
                message: "expected type".into(),
            }]),
        }
    }
    fn program(&mut self) -> Result<Program, Vec<Diagnostic>> {
        let mut functions = vec![];
        let mut structs = vec![];
        while self.i < self.tokens.len() {
            if self.tokens.get(self.i).map(String::as_str) == Some("struct") {
                self.take();
                let name = self.take().ok_or_else(|| {
                    vec![Diagnostic {
                        message: "expected struct name".into(),
                    }]
                })?;
                self.need("{")?;
                let mut fields = vec![];
                while self.tokens.get(self.i).map(String::as_str) != Some("}") {
                    let field = self.take().ok_or_else(|| {
                        vec![Diagnostic {
                            message: "expected field".into(),
                        }]
                    })?;
                    self.need(":")?;
                    fields.push(Field {
                        name: field,
                        ty: self.ty()?,
                    });
                    if self.tokens.get(self.i).map(String::as_str) == Some(",") {
                        self.take();
                    }
                }
                self.need("}")?;
                structs.push(Struct { name, fields });
                continue;
            }
            self.need("fn")?;
            let name = self.take().ok_or_else(|| {
                vec![Diagnostic {
                    message: "expected function name".into(),
                }]
            })?;
            self.need("(")?;
            let mut params = vec![];
            while self.tokens.get(self.i).map(String::as_str) != Some(")") {
                let parameter_name = self.take().ok_or_else(|| {
                    vec![Diagnostic {
                        message: "expected parameter name".into(),
                    }]
                })?;
                self.need(":")?;
                let parameter_type = self.ty()?;
                params.push(Parameter {
                    name: parameter_name,
                    ty: parameter_type,
                });
                if self.tokens.get(self.i).map(String::as_str) == Some(",") {
                    self.take();
                } else if self.tokens.get(self.i).map(String::as_str) != Some(")") {
                    return Err(vec![Diagnostic {
                        message: "expected `,` or `)` after parameter".into(),
                    }]);
                }
            }
            self.need(")")?;
            self.need("->")?;
            let result = self.ty()?;
            self.need("{")?;
            let mut body = vec![];
            while self.tokens.get(self.i).map(String::as_str) != Some("}") {
                body.push(self.stmt()?);
            }
            self.need("}")?;
            functions.push(Function {
                name,
                params,
                result,
                body,
            });
        }
        Ok(Program { structs, functions })
    }
    fn stmt(&mut self) -> Result<Stmt, Vec<Diagnostic>> {
        match self.tokens.get(self.i).map(String::as_str) {
            Some("if") => {
                self.take();
                let left = self.expr()?;
                let operator = self.take().ok_or_else(|| {
                    vec![Diagnostic {
                        message: "expected comparison".into(),
                    }]
                })?;
                let right = self.expr()?;
                self.need("{")?;
                let mut then_body = vec![];
                while self.tokens.get(self.i).map(String::as_str) != Some("}") {
                    then_body.push(self.stmt()?);
                }
                self.need("}")?;
                self.need("else")?;
                self.need("{")?;
                let mut else_body = vec![];
                while self.tokens.get(self.i).map(String::as_str) != Some("}") {
                    else_body.push(self.stmt()?);
                }
                self.need("}")?;
                Ok(Stmt::If {
                    condition: Expr::Compare(Box::new(left), operator, Box::new(right)),
                    then_body,
                    else_body,
                })
            }
            Some("let") => {
                self.take();
                let mutable = if self.tokens.get(self.i).map(String::as_str) == Some("mut") {
                    self.take();
                    true
                } else {
                    false
                };
                let name = self.take().unwrap();
                let ty = if self.tokens.get(self.i).map(String::as_str) == Some(":") {
                    self.take();
                    Some(self.ty()?)
                } else {
                    None
                };
                self.need("=")?;
                Ok(Stmt::Let {
                    name,
                    mutable,
                    ty,
                    value: self.expr()?,
                })
            }
            Some("return") => {
                self.take();
                Ok(Stmt::Return(self.expr()?))
            }
            Some(_) => {
                let name = self.take().unwrap();
                self.need("=")?;
                Ok(Stmt::Assign {
                    name,
                    value: self.expr()?,
                })
            }
            None => Err(vec![Diagnostic {
                message: "unexpected end".into(),
            }]),
        }
    }
    fn expr(&mut self) -> Result<Expr, Vec<Diagnostic>> {
        let atom = self.atom()?;
        let mut left = self.postfix(atom)?;
        while matches!(
            self.tokens.get(self.i).map(String::as_str),
            Some("+") | Some("-") | Some("*") | Some("/")
        ) {
            let op = self.take().unwrap().chars().next().unwrap();
            let atom = self.atom()?;
            let right = self.postfix(atom)?;
            left = Expr::Binary(Box::new(left), op, Box::new(right));
        }
        Ok(left)
    }
    fn postfix(&mut self, mut left: Expr) -> Result<Expr, Vec<Diagnostic>> {
        while self.tokens.get(self.i).map(String::as_str) == Some(".") {
            self.take();
            let name = self.take().ok_or_else(|| {
                vec![Diagnostic {
                    message: "expected field name".into(),
                }]
            })?;
            left = Expr::Field {
                base: Box::new(left),
                name,
            };
        }
        Ok(left)
    }
    fn atom(&mut self) -> Result<Expr, Vec<Diagnostic>> {
        let s = self.take().ok_or_else(|| {
            vec![Diagnostic {
                message: "expected expression".into(),
            }]
        })?;
        if let Some(n) = s.strip_suffix("i64").or_else(|| s.strip_suffix("i32")) {
            if let Ok(value) = n.parse() {
                return Ok(Expr::Int(
                    value,
                    if s.ends_with("i64") {
                        Type::I64
                    } else {
                        Type::I32
                    },
                ));
            }
        }
        if let Ok(n) = s.parse() {
            Ok(Expr::Int(n, Type::I32))
        } else if self.tokens.get(self.i).map(String::as_str) == Some("{") {
            self.take();
            let mut fields = vec![];
            while self.tokens.get(self.i).map(String::as_str) != Some("}") {
                let field = self.take().ok_or_else(|| {
                    vec![Diagnostic {
                        message: "expected field name".into(),
                    }]
                })?;
                self.need(":")?;
                let value = self.expr()?;
                fields.push((field, value));
                if self.tokens.get(self.i).map(String::as_str) == Some(",") {
                    self.take();
                }
            }
            self.need("}")?;
            Ok(Expr::StructLiteral { name: s, fields })
        } else if self.tokens.get(self.i).map(String::as_str) == Some("(") {
            self.take();
            let mut arguments = vec![];
            while self.tokens.get(self.i).map(String::as_str) != Some(")") {
                arguments.push(self.expr()?);
                if self.tokens.get(self.i).map(String::as_str) == Some(",") {
                    self.take();
                } else if self.tokens.get(self.i).map(String::as_str) != Some(")") {
                    return Err(vec![Diagnostic {
                        message: "expected `,` or `)` after argument".into(),
                    }]);
                }
            }
            self.need(")")?;
            Ok(Expr::Call { name: s, arguments })
        } else {
            Ok(Expr::Name(s))
        }
    }
}
