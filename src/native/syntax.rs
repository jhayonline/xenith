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
    While {
        condition: Expr,
        body: Vec<Stmt>,
    },
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
    Bool(bool),
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
    Not(Box<Expr>),
    Logical(Box<Expr>, String, Box<Expr>),
    Compare(Box<Expr>, String, Box<Expr>),
    Binary(Box<Expr>, char, Box<Expr>),
}
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub message: String,
    pub span: Option<Span>,
}

impl Diagnostic {
    fn at(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span: Some(span),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

#[derive(Debug, Clone)]
struct Token {
    text: String,
    span: Span,
}

fn lex(source: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut word = String::new();
    let mut word_start = 0;
    for (index, c) in source.char_indices() {
        if c.is_whitespace() {
            if !word.is_empty() {
                out.push(Token {
                    text: std::mem::take(&mut word),
                    span: Span::new(word_start, index),
                });
            }
        } else if c == '>' && word == "-" {
            word.push(c);
            out.push(Token {
                text: std::mem::take(&mut word),
                span: Span::new(word_start, index + c.len_utf8()),
            });
        } else if "{}(),:;=+-*/.<>!&|".contains(c) {
            if !word.is_empty() {
                out.push(Token {
                    text: std::mem::take(&mut word),
                    span: Span::new(word_start, index),
                });
            }
            if c == '-' {
                word_start = index;
                word.push(c);
            } else {
                out.push(Token {
                    text: c.to_string(),
                    span: Span::new(index, index + c.len_utf8()),
                });
            }
        } else {
            if word.is_empty() {
                word_start = index;
            }
            word.push(c);
        }
    }
    let source_end = source.len();
    if !word.is_empty() {
        out.push(Token {
            text: word,
            span: Span::new(word_start, source_end),
        });
    }
    let mut merged = Vec::new();
    let mut index = 0;
    while index < out.len() {
        if matches!(out[index].text.as_str(), "<" | ">" | "!" | "=")
            && out.get(index + 1).map(|token| token.text.as_str()) == Some("=")
        {
            merged.push(Token {
                text: format!("{}=", out[index].text),
                span: Span::new(out[index].span.start, out[index + 1].span.end),
            });
            index += 2;
        } else if matches!(out[index].text.as_str(), "&" | "|")
            && out.get(index + 1).map(|token| token.text.as_str()) == Some(out[index].text.as_str())
        {
            merged.push(Token {
                text: format!("{}{}", out[index].text, out[index].text),
                span: Span::new(out[index].span.start, out[index + 1].span.end),
            });
            index += 2;
        } else {
            merged.push(out[index].clone());
            index += 1;
        }
    }
    merged
}
pub fn parse(_file: &Path, source: &str) -> Result<Program, Vec<Diagnostic>> {
    let tokens = lex(source);
    let mut p = Parser {
        spans: tokens.iter().map(|token| token.span).collect(),
        tokens: tokens.into_iter().map(|token| token.text).collect(),
        i: 0,
        source_end: source.len(),
    };
    p.program()
}
struct Parser {
    tokens: Vec<String>,
    spans: Vec<Span>,
    i: usize,
    source_end: usize,
}
impl Parser {
    fn peek(&self) -> Option<&str> {
        self.tokens.get(self.i).map(String::as_str)
    }
    fn location(&self) -> Span {
        self.tokens
            .get(self.i)
            .and_then(|_| self.spans.get(self.i).copied())
            .unwrap_or_else(|| Span::new(self.source_end, self.source_end))
    }
    fn previous_location(&self) -> Span {
        self.spans
            .get(self.i.saturating_sub(1))
            .copied()
            .unwrap_or_else(|| self.location())
    }
    fn take(&mut self) -> Option<String> {
        let x = self.tokens.get(self.i).cloned();
        self.i += 1;
        x
    }
    fn need(&mut self, s: &str) -> Result<(), Vec<Diagnostic>> {
        if self.take().as_deref() == Some(s) {
            Ok(())
        } else {
            Err(vec![Diagnostic::at(
                format!("expected `{s}`"),
                self.previous_location(),
            )])
        }
    }
    fn ty(&mut self) -> Result<Type, Vec<Diagnostic>> {
        match self.take().as_deref() {
            Some("i32") => Ok(Type::I32),
            Some("i64") => Ok(Type::I64),
            Some("bool") => Ok(Type::Bool),
            Some(name) => Ok(Type::Named(name.to_owned())),
            None => Err(vec![Diagnostic::at("expected type", self.location())]),
        }
    }
    fn program(&mut self) -> Result<Program, Vec<Diagnostic>> {
        let mut functions = vec![];
        let mut structs = vec![];
        let mut errors = vec![];
        while self.i < self.tokens.len() {
            if self.tokens.get(self.i).map(String::as_str) == Some("struct") {
                self.take();
                let name = self.take().ok_or_else(|| {
                    vec![Diagnostic {
                        message: "expected struct name".into(),
                        span: Some(self.location()),
                    }]
                })?;
                self.need("{")?;
                let mut fields = vec![];
                while self.peek() != Some("}") && self.peek().is_some() {
                    let Some(field) = self.take() else {
                        errors.push(Diagnostic::at("expected field", self.location()));
                        break;
                    };
                    if let Err(mut field_errors) = self.need(":") {
                        errors.append(&mut field_errors);
                        self.synchronize_struct_field();
                        continue;
                    }
                    let ty = match self.ty() {
                        Ok(ty) => ty,
                        Err(mut field_errors) => {
                            errors.append(&mut field_errors);
                            self.synchronize_struct_field();
                            continue;
                        }
                    };
                    fields.push(Field { name: field, ty });
                    if self.tokens.get(self.i).map(String::as_str) == Some(",") {
                        self.take();
                    }
                }
                if let Err(mut closing_errors) = self.need("}") {
                    errors.append(&mut closing_errors);
                }
                structs.push(Struct { name, fields });
                continue;
            }
            self.need("fn")?;
            let name = self.take().ok_or_else(|| {
                vec![Diagnostic {
                    message: "expected function name".into(),
                    span: Some(self.location()),
                }]
            })?;
            self.need("(")?;
            let mut params = vec![];
            while self.tokens.get(self.i).map(String::as_str) != Some(")") {
                let parameter_name = self.take().ok_or_else(|| {
                    vec![Diagnostic {
                        message: "expected parameter name".into(),
                        span: Some(self.location()),
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
                        span: Some(self.location()),
                    }]);
                }
            }
            self.need(")")?;
            self.need("->")?;
            let result = self.ty()?;
            let body = match self.block() {
                Ok(body) => body,
                Err(mut block_errors) => {
                    errors.append(&mut block_errors);
                    vec![]
                }
            };
            functions.push(Function {
                name,
                params,
                result,
                body,
            });
        }
        if errors.is_empty() {
            Ok(Program { structs, functions })
        } else {
            Err(errors)
        }
    }
    fn synchronize_statement(&mut self) {
        while !matches!(self.peek(), Some("let" | "return" | "if" | "while" | "}"))
            && self.peek().is_some()
        {
            self.take();
        }
    }
    fn synchronize_struct_field(&mut self) {
        while !matches!(self.peek(), Some("," | "}" | "fn" | "struct")) && self.peek().is_some() {
            self.take();
        }
        if self.peek() == Some(",") {
            self.take();
        }
    }
    fn block(&mut self) -> Result<Vec<Stmt>, Vec<Diagnostic>> {
        self.need("{")?;
        let mut body = vec![];
        let mut errors = vec![];
        while self.peek() != Some("}") && self.peek().is_some() {
            match self.stmt() {
                Ok(statement) => body.push(statement),
                Err(mut statement_errors) => {
                    errors.append(&mut statement_errors);
                    self.synchronize_statement();
                }
            }
        }
        if let Err(mut closing_errors) = self.need("}") {
            errors.append(&mut closing_errors);
        }
        if errors.is_empty() {
            Ok(body)
        } else {
            Err(errors)
        }
    }
    fn stmt(&mut self) -> Result<Stmt, Vec<Diagnostic>> {
        match self.tokens.get(self.i).map(String::as_str) {
            Some("while") => {
                self.take();
                let condition = self.condition()?;
                let body = self.block()?;
                Ok(Stmt::While { condition, body })
            }
            Some("if") => {
                self.take();
                let condition = self.condition()?;
                let then_body = self.block()?;
                self.need("else")?;
                let else_body = self.block()?;
                Ok(Stmt::If {
                    condition,
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
                span: Some(self.location()),
            }]),
        }
    }
    fn expr(&mut self) -> Result<Expr, Vec<Diagnostic>> {
        self.logical_or()
    }
    fn condition(&mut self) -> Result<Expr, Vec<Diagnostic>> {
        self.expr()
    }
    fn logical_or(&mut self) -> Result<Expr, Vec<Diagnostic>> {
        let mut left = self.logical_and()?;
        while self.tokens.get(self.i).map(String::as_str) == Some("||") {
            self.take();
            left = Expr::Logical(Box::new(left), "||".into(), Box::new(self.logical_and()?));
        }
        Ok(left)
    }
    fn logical_and(&mut self) -> Result<Expr, Vec<Diagnostic>> {
        let mut left = self.comparison()?;
        while self.tokens.get(self.i).map(String::as_str) == Some("&&") {
            self.take();
            left = Expr::Logical(Box::new(left), "&&".into(), Box::new(self.comparison()?));
        }
        Ok(left)
    }
    fn comparison(&mut self) -> Result<Expr, Vec<Diagnostic>> {
        let left = self.arithmetic()?;
        if matches!(
            self.tokens.get(self.i).map(String::as_str),
            Some("<") | Some(">") | Some("<=") | Some(">=") | Some("==") | Some("!=")
        ) {
            let operator = self.take().unwrap();
            return Ok(Expr::Compare(
                Box::new(left),
                operator,
                Box::new(self.arithmetic()?),
            ));
        }
        Ok(left)
    }
    fn arithmetic(&mut self) -> Result<Expr, Vec<Diagnostic>> {
        let mut left = self.multiplicative()?;
        while matches!(
            self.tokens.get(self.i).map(String::as_str),
            Some("+") | Some("-")
        ) {
            let op = self.take().unwrap().chars().next().unwrap();
            let right = self.multiplicative()?;
            left = Expr::Binary(Box::new(left), op, Box::new(right));
        }
        Ok(left)
    }
    fn multiplicative(&mut self) -> Result<Expr, Vec<Diagnostic>> {
        let atom = self.atom()?;
        let mut left = self.postfix(atom)?;
        while matches!(
            self.tokens.get(self.i).map(String::as_str),
            Some("*") | Some("/")
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
                    span: Some(self.location()),
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
                span: Some(self.location()),
            }]
        })?;
        let span = self.previous_location();
        if matches!(s.as_str(), ")" | "}" | "," | ";" | ":" | "=" | "else") {
            return Err(vec![Diagnostic::at("expected expression", span)]);
        }
        if s == "!" {
            return Ok(Expr::Not(Box::new(self.atom()?)));
        }
        if s == "(" {
            let expression = self.expr()?;
            self.need(")")?;
            return Ok(expression);
        }
        if s == "true" {
            return Ok(Expr::Bool(true));
        }
        if s == "false" {
            return Ok(Expr::Bool(false));
        }
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
        } else if self.tokens.get(self.i).map(String::as_str) == Some("{")
            && self.tokens.get(self.i + 2).map(String::as_str) == Some(":")
        {
            self.take();
            let mut fields = vec![];
            while self.tokens.get(self.i).map(String::as_str) != Some("}") {
                let field = self.take().ok_or_else(|| {
                    vec![Diagnostic {
                        message: "expected field name".into(),
                        span: Some(self.location()),
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
                        span: Some(self.location()),
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
