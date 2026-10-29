use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    I32,
    I64,
}
#[derive(Debug, Clone)]
pub struct Program {
    pub functions: Vec<Function>,
}
#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub params: usize,
    pub result: Type,
    pub body: Vec<Stmt>,
}
#[derive(Debug, Clone)]
pub enum Stmt {
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
        } else if "{}(),:;=+-*/".contains(c) {
            if !word.is_empty() {
                out.push(std::mem::take(&mut word));
            }
            if c == '-' {
                word.push(c);
            } else {
                out.push(c.to_string());
            }
        } else if c == '>' && word == "-" {
            word.push(c);
            out.push(std::mem::take(&mut word));
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
            _ => Err(vec![Diagnostic {
                message: "expected integer type".into(),
            }]),
        }
    }
    fn program(&mut self) -> Result<Program, Vec<Diagnostic>> {
        let mut functions = vec![];
        while self.i < self.tokens.len() {
            self.need("fn")?;
            let name = self.take().ok_or_else(|| {
                vec![Diagnostic {
                    message: "expected function name".into(),
                }]
            })?;
            self.need("(")?;
            let mut params = 0;
            while self.tokens.get(self.i).map(String::as_str) != Some(")") {
                params += 1;
                self.take();
                if self.tokens.get(self.i).map(String::as_str) == Some(":") {
                    self.take();
                    self.ty()?;
                }
                if self.tokens.get(self.i).map(String::as_str) == Some(",") {
                    self.take();
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
        Ok(Program { functions })
    }
    fn stmt(&mut self) -> Result<Stmt, Vec<Diagnostic>> {
        match self.tokens.get(self.i).map(String::as_str) {
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
        let mut left = self.atom()?;
        while matches!(
            self.tokens.get(self.i).map(String::as_str),
            Some("+") | Some("-") | Some("*") | Some("/")
        ) {
            let op = self.take().unwrap().chars().next().unwrap();
            let right = self.atom()?;
            left = Expr::Binary(Box::new(left), op, Box::new(right));
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
            return Ok(Expr::Int(
                n.parse().unwrap(),
                if s.ends_with("i64") {
                    Type::I64
                } else {
                    Type::I32
                },
            ));
        }
        if let Ok(n) = s.parse() {
            Ok(Expr::Int(n, Type::I32))
        } else {
            Ok(Expr::Name(s))
        }
    }
}
