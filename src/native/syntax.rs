use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    I32,
    I64,
    Float,
    Bool,
    Null,
    String,
    Bytes,
    List(Box<Type>),
    Map(Box<Type>, Box<Type>),
    Tuple(Vec<Type>),
    Function(Vec<Type>, Box<Type>),
    Named(String),
}
#[derive(Debug, Clone)]
pub struct Program {
    pub file: String,
    pub source: String,
    pub imports: Vec<Import>,
    pub structs: Vec<Struct>,
    pub enums: Vec<Enum>,
    pub functions: Vec<Function>,
}
#[derive(Debug, Clone)]
pub struct Enum {
    pub name: String,
    pub exported: bool,
    pub variants: Vec<EnumVariant>,
}
#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub payload: Vec<Type>,
}
#[derive(Debug, Clone)]
pub struct Import {
    pub names: Vec<ImportName>,
    pub namespace: Option<String>,
    pub path: String,
}
#[derive(Debug, Clone)]
pub struct ImportName {
    pub original: String,
    pub local: String,
}
#[derive(Debug, Clone)]
pub struct Struct {
    pub name: String,
    pub exported: bool,
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
    pub file: String,
    pub source: String,
    pub span: Span,
    pub exported: bool,
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
    Stop,
    Skip,
    While {
        condition: Expr,
        body: Vec<Stmt>,
    },
    For {
        init: Box<Stmt>,
        condition: Expr,
        step: Box<Stmt>,
        body: Vec<Stmt>,
    },
    ForIn {
        names: Vec<String>,
        iterable: Expr,
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
    IndexAssign {
        base: Expr,
        index: Expr,
        value: Expr,
    },
    Return(Expr),
    Panic { value: Expr, span: Span },
    TryCatch {
        body: Vec<Stmt>,
        binding: String,
        catch_body: Vec<Stmt>,
    },
    Expr(Expr),
}
#[derive(Debug, Clone)]
pub enum Expr {
    Int(i64, Type),
    Float(f64),
    Bool(bool),
    Null,
    String(String),
    Interpolated(Vec<InterpolationPart>, Span),
    Bytes(String, Span),
    List(Vec<Expr>, Span),
    Map(Vec<(Expr, Expr)>, Span),
    Tuple(Vec<Expr>),
    Name(String),
    Closure {
        params: Vec<Parameter>,
        result: Type,
        body: Box<Expr>,
        span: Span,
    },
    EnumVariant {
        enum_name: String,
        variant: String,
        arguments: Vec<Expr>,
    },
    Match {
        subject: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    Call {
        name: String,
        arguments: Vec<Expr>,
        span: Span,
    },
    StructLiteral {
        name: String,
        fields: Vec<(String, Expr)>,
    },
    Field {
        base: Box<Expr>,
        name: String,
    },
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },
    Neg(Box<Expr>),
    Not(Box<Expr>),
    Logical(Box<Expr>, String, Box<Expr>),
    Compare(Box<Expr>, String, Box<Expr>),
    Binary(Box<Expr>, char, Box<Expr>, Span),
}
#[derive(Debug, Clone)]
pub enum InterpolationPart {
    Text(String),
    Name(String),
}
#[derive(Debug, Clone)]
pub struct MatchArm {
    pub enum_name: Option<String>,
    pub variant: Option<String>,
    pub binding: Option<String>,
    pub body: Box<Expr>,
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
    let mut in_comment = false;
    let mut in_string = false;
    for (index, c) in source.char_indices() {
        if in_string {
            word.push(c);
            if c == '"' {
                out.push(Token {
                    text: std::mem::take(&mut word),
                    span: Span::new(word_start, index + c.len_utf8()),
                });
                in_string = false;
            }
            continue;
        }
        if in_comment {
            if c == '\n' {
                in_comment = false;
            } else {
                continue;
            }
        }
        if c == '"' {
            if !word.is_empty() {
                out.push(Token {
                    text: std::mem::take(&mut word),
                    span: Span::new(word_start, index),
                });
            }
            word_start = index;
            word.push(c);
            in_string = true;
        } else if c == '#' {
            if !word.is_empty() {
                out.push(Token {
                    text: std::mem::take(&mut word),
                    span: Span::new(word_start, index),
                });
            }
            in_comment = true;
        } else if c.is_whitespace() {
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
        } else if "{}[](),:;=+-*/.<>!&|".contains(c) {
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
        if out[index].text == ":"
            && out[index].span.end
                == out
                    .get(index + 1)
                    .map(|token| token.span.start)
                    .unwrap_or_default()
            && out.get(index + 1).map(|token| token.text.as_str()) == Some(":")
        {
            merged.push(Token {
                text: "::".into(),
                span: Span::new(out[index].span.start, out[index + 1].span.end),
            });
            index += 2;
        } else if out[index].text == "="
            && out[index].span.end
                == out
                    .get(index + 1)
                    .map(|token| token.span.start)
                    .unwrap_or_default()
            && out.get(index + 1).map(|token| token.text.as_str()) == Some(">")
        {
            merged.push(Token {
                text: "=>".into(),
                span: Span::new(out[index].span.start, out[index + 1].span.end),
            });
            index += 2;
        } else if matches!(out[index].text.as_str(), "<" | ">" | "!" | "=")
            && out[index].span.end
                == out
                    .get(index + 1)
                    .map(|token| token.span.start)
                    .unwrap_or_default()
            && out.get(index + 1).map(|token| token.text.as_str()) == Some("=")
        {
            merged.push(Token {
                text: format!("{}=", out[index].text),
                span: Span::new(out[index].span.start, out[index + 1].span.end),
            });
            index += 2;
        } else if matches!(out[index].text.as_str(), "&" | "|")
            && out[index].span.end
                == out
                    .get(index + 1)
                    .map(|token| token.span.start)
                    .unwrap_or_default()
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
pub fn parse(file: &Path, source: &str) -> Result<Program, Vec<Diagnostic>> {
    let tokens = lex(source);
    let mut p = Parser {
        spans: tokens.iter().map(|token| token.span).collect(),
        tokens: tokens.into_iter().map(|token| token.text).collect(),
        i: 0,
        source_end: source.len(),
        source: source.to_string(),
        file: file.display().to_string(),
    };
    p.program()
}
struct Parser {
    tokens: Vec<String>,
    spans: Vec<Span>,
    i: usize,
    source_end: usize,
    source: String,
    file: String,
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
    fn identifier(&mut self, description: &str) -> Result<String, Vec<Diagnostic>> {
        let name = self.take().ok_or_else(|| {
            vec![Diagnostic::at(
                format!("expected {description}"),
                self.location(),
            )]
        })?;
        if is_identifier(&name) {
            Ok(name)
        } else {
            Err(vec![Diagnostic::at(
                format!("invalid {description} `{name}`"),
                self.previous_location(),
            )])
        }
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
            Some("float") => Ok(Type::Float),
            Some("bool") => Ok(Type::Bool),
            Some("null") => Ok(Type::Null),
            Some("string") => Ok(Type::String),
            Some("bytes") => Ok(Type::Bytes),
            Some("list") => {
                self.need("<")?;
                let element = self.ty()?;
                self.need(">")?;
                Ok(Type::List(Box::new(element)))
            }
            Some("map") => {
                self.need("<")?;
                let key = self.ty()?;
                self.need(",")?;
                let value = self.ty()?;
                self.need(">")?;
                Ok(Type::Map(Box::new(key), Box::new(value)))
            }
            Some("tuple") => {
                self.need("<")?;
                let mut elements = vec![self.ty()?];
                while self.peek() == Some(",") {
                    self.take();
                    elements.push(self.ty()?);
                }
                self.need(">")?;
                Ok(Type::Tuple(elements))
            }
            Some("fn") => {
                self.need("(")?;
                let mut parameters = Vec::new();
                if self.peek() != Some(")") {
                    parameters.push(self.ty()?);
                    while self.peek() == Some(",") {
                        self.take();
                        parameters.push(self.ty()?);
                    }
                }
                self.need(")")?;
                self.need("->")?;
                Ok(Type::Function(parameters, Box::new(self.ty()?)))
            }
            Some(name) if is_identifier(name) => Ok(Type::Named(name.to_owned())),
            Some(name) => Err(vec![Diagnostic::at(
                format!("invalid type name `{name}`"),
                self.previous_location(),
            )]),
            None => Err(vec![Diagnostic::at("expected type", self.location())]),
        }
    }
    fn program(&mut self) -> Result<Program, Vec<Diagnostic>> {
        let mut functions = vec![];
        let mut structs = vec![];
        let mut enums = vec![];
        let mut imports = vec![];
        let mut errors = vec![];
        while self.i < self.tokens.len() {
            if self.peek() == Some("grab") {
                self.take();
                if self.peek() == Some("*") {
                    self.take();
                    self.need("as")?;
                    let namespace = self.identifier("module namespace")?;
                    self.need("from")?;
                    let path = self.take().ok_or_else(|| {
                        vec![Diagnostic::at("expected module path", self.location())]
                    })?;
                    imports.push(Import {
                        names: vec![],
                        namespace: Some(namespace),
                        path: path.trim_matches('"').to_string(),
                    });
                    continue;
                }
                self.need("{")?;
                let mut names = vec![];
                while self.peek() != Some("}") {
                    let original = self.identifier("import name")?;
                    let local = if self.peek() == Some("as") {
                        self.take();
                        self.identifier("import alias")?
                    } else {
                        original.clone()
                    };
                    names.push(ImportName { original, local });
                    if self.peek() == Some(",") {
                        self.take();
                    } else {
                        break;
                    }
                }
                self.need("}")?;
                self.need("from")?;
                let path = self
                    .take()
                    .ok_or_else(|| vec![Diagnostic::at("expected module path", self.location())])?;
                let path = path.trim_matches('"').to_string();
                imports.push(Import {
                    names,
                    namespace: None,
                    path,
                });
                continue;
            }
            let exported = if self.peek() == Some("export") {
                self.take();
                true
            } else {
                false
            };
            if self.tokens.get(self.i).map(String::as_str) == Some("struct") {
                self.take();
                let name = self.identifier("struct name")?;
                self.need("{")?;
                let mut fields = vec![];
                while self.peek() != Some("}") && self.peek().is_some() {
                    let field = match self.identifier("field name") {
                        Ok(field) => field,
                        Err(mut field_errors) => {
                            errors.append(&mut field_errors);
                            self.synchronize_struct_field();
                            continue;
                        }
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
                structs.push(Struct {
                    name,
                    exported,
                    fields,
                });
                continue;
            }
            if self.tokens.get(self.i).map(String::as_str) == Some("enum") {
                self.take();
                let name = self.identifier("enum name")?;
                self.need("{")?;
                let mut variants = vec![];
                while self.peek() != Some("}") && self.peek().is_some() {
                    let variant_name = self.identifier("enum variant name")?;
                    let payload = if self.peek() == Some("(") {
                        self.take();
                        let mut payload = vec![];
                        if self.peek() != Some(")") {
                            payload.push(self.ty()?);
                            while self.peek() == Some(",") {
                                self.take();
                                payload.push(self.ty()?);
                            }
                        }
                        self.need(")")?;
                        payload
                    } else {
                        vec![]
                    };
                    variants.push(EnumVariant {
                        name: variant_name,
                        payload,
                    });
                    if self.peek() == Some(",") {
                        self.take();
                    }
                }
                self.need("}")?;
                enums.push(Enum {
                    name,
                    exported,
                    variants,
                });
                continue;
            }
            if !matches!(self.peek(), Some("function" | "fn")) {
                if errors.is_empty() {
                    errors.push(Diagnostic::at("expected `function`", self.location()));
                }
                self.synchronize_top_level();
                continue;
            }
            let function_span = self.location();
            self.take();
            let name = self.identifier("function name")?;
            self.need("(")?;
            let mut params = vec![];
            while self.tokens.get(self.i).map(String::as_str) != Some(")") {
                let parameter_name = self.identifier("parameter name")?;
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
                file: self.file.clone(),
                source: self.source.clone(),
                span: function_span,
                exported,
                params,
                result,
                body,
            });
        }
        if errors.is_empty() {
            Ok(Program {
                file: self.file.clone(),
                source: self.source.to_string(),
                imports,
                structs,
                enums,
                functions,
            })
        } else {
            Err(errors)
        }
    }
    fn synchronize_statement(&mut self) {
        while !matches!(
            self.peek(),
            Some("let" | "return" | "if" | "while" | "for" | "stop" | "skip" | "}")
        ) && self.peek().is_some()
        {
            self.take();
        }
    }
    fn synchronize_struct_field(&mut self) {
        while !matches!(self.peek(), Some("," | "}" | "fn" | "function" | "struct")) && self.peek().is_some() {
            self.take();
        }
        if self.peek() == Some(",") {
            self.take();
        }
    }
    fn synchronize_top_level(&mut self) {
        while !matches!(self.peek(), Some("fn" | "function" | "struct")) && self.peek().is_some() {
            self.take();
        }
    }
    fn block(&mut self) -> Result<Vec<Stmt>, Vec<Diagnostic>> {
        self.need("{")?;
        let mut body = vec![];
        let mut errors = vec![];
        while self.peek() != Some("}") && self.peek().is_some() {
            match self.stmt() {
                Ok(statement) => {
                    body.push(statement);
                    while self.peek() == Some(";") {
                        self.take();
                    }
                }
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
            Some("try") => {
                self.take();
                let body = self.block()?;
                self.need("catch")?;
                self.need("(")?;
                let binding = self.identifier("catch binding")?;
                self.need(")")?;
                let catch_body = self.block()?;
                Ok(Stmt::TryCatch { body, binding, catch_body })
            }
            Some("panic") => {
                let span = self.location();
                self.take();
                Ok(Stmt::Panic { value: self.expr()?, span })
            }
            Some("stop") => {
                self.take();
                Ok(Stmt::Stop)
            }
            Some("skip") => {
                self.take();
                Ok(Stmt::Skip)
            }
            Some("while") => {
                self.take();
                let condition = self.condition()?;
                let body = self.block()?;
                Ok(Stmt::While { condition, body })
            }
            Some("for") => {
                self.take();
                self.need("(")?;
                if self.peek().is_some_and(is_identifier)
                    && self.tokens.get(self.i + 1).map(String::as_str) == Some("in")
                {
                    let first = self.identifier("iteration binding")?;
                    self.take();
                    let mut names = vec![first];
                    if self.peek() == Some(",") {
                        self.take();
                        names.push(self.identifier("iteration binding")?);
                    }
                    let iterable = self.expr()?;
                    self.need(")")?;
                    let body = self.block()?;
                    return Ok(Stmt::ForIn {
                        names,
                        iterable,
                        body,
                    });
                }
                let init = self.stmt()?;
                self.need(";")?;
                let condition = self.condition()?;
                self.need(";")?;
                let step = self.stmt()?;
                self.need(")")?;
                let body = self.block()?;
                Ok(Stmt::For {
                    init: Box::new(init),
                    condition,
                    step: Box::new(step),
                    body,
                })
            }
            Some("if") => {
                self.take();
                let condition = self.condition()?;
                let then_body = self.block()?;
                let else_body = if self.peek() == Some("else") {
                    self.take();
                    if self.peek() == Some("if") {
                        vec![self.stmt()?]
                    } else {
                        self.block()?
                    }
                } else {
                    vec![]
                };
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
                let name = self.identifier("binding name")?;
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
                if self.tokens.get(self.i + 1).map(String::as_str) == Some("=") {
                    let name = self.identifier("assignment target")?;
                    self.need("=")?;
                    return Ok(Stmt::Assign {
                        name,
                        value: self.expr()?,
                    });
                }
                let target = self.expr()?;
                if self.peek() != Some("=") {
                    return Ok(Stmt::Expr(target));
                }
                self.take();
                let value = self.expr()?;
                match target {
                    Expr::Name(name) => Ok(Stmt::Assign { name, value }),
                    Expr::Index { base, index, .. } => Ok(Stmt::IndexAssign {
                        base: *base,
                        index: *index,
                        value,
                    }),
                    _ => Err(vec![Diagnostic::at(
                        "invalid assignment target",
                        self.previous_location(),
                    )]),
                }
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
            let span = self.location();
            let op = self.take().unwrap().chars().next().unwrap();
            let right = self.multiplicative()?;
            left = Expr::Binary(Box::new(left), op, Box::new(right), span);
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
            let span = self.location();
            let op = self.take().unwrap().chars().next().unwrap();
            let atom = self.atom()?;
            let right = self.postfix(atom)?;
            left = Expr::Binary(Box::new(left), op, Box::new(right), span);
        }
        Ok(left)
    }
    fn postfix(&mut self, mut left: Expr) -> Result<Expr, Vec<Diagnostic>> {
        while matches!(
            self.tokens.get(self.i).map(String::as_str),
            Some(".") | Some("[")
        ) {
            if self.peek() == Some(".") {
                self.take();
                let name = self.identifier("field name")?;
                if self.peek() == Some("(") {
                    self.take();
                    let mut arguments = vec![left];
                    while self.peek() != Some(")") {
                        arguments.push(self.expr()?);
                        if self.peek() == Some(",") {
                            self.take();
                        } else {
                            break;
                        }
                    }
                    self.need(")")?;
                    left = Expr::Call { name, arguments, span: Span::new(0, 0) };
                } else {
                    left = Expr::Field {
                        base: Box::new(left),
                        name,
                    };
                }
            } else {
                let span = self.location();
                self.take();
                let index = self.expr()?;
                self.need("]")?;
                left = Expr::Index {
                    base: Box::new(left),
                    index: Box::new(index),
                    span,
                };
            }
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
        if s == "fn" {
            self.need("(")?;
            let mut params = Vec::new();
            while self.peek() != Some(")") {
                let name = self.identifier("parameter name")?;
                self.need(":")?;
                let ty = self.ty()?;
                params.push(Parameter { name, ty });
                if self.peek() == Some(",") {
                    self.take();
                } else {
                    break;
                }
            }
            self.need(")")?;
            self.need("->")?;
            let result = self.ty()?;
            self.need("=>")?;
            let body = self.expr()?;
            return Ok(Expr::Closure {
                params,
                result,
                body: Box::new(body),
                span,
            });
        }
        if s == "-" {
            return Ok(Expr::Neg(Box::new(self.atom()?)));
        }
        if let Some(name) = s.strip_prefix('-') {
            if is_identifier(name) {
                if self.tokens.get(self.i).map(String::as_str) == Some("(") {
                    return Ok(Expr::Neg(Box::new(self.call(name.into())?)));
                }
                return Ok(Expr::Neg(Box::new(Expr::Name(name.into()))));
            }
        }
        if s == "(" {
            let expression = self.expr()?;
            if self.peek() == Some(",") {
                let mut elements = vec![expression];
                while self.peek() == Some(",") {
                    self.take();
                    elements.push(self.expr()?);
                }
                self.need(")")?;
                return Ok(Expr::Tuple(elements));
            }
            self.need(")")?;
            return Ok(expression);
        }
        if s == "[" {
            let mut elements = vec![];
            while self.peek() != Some("]") {
                elements.push(self.expr()?);
                if self.peek() == Some(",") {
                    self.take();
                } else {
                    break;
                }
            }
            self.need("]")?;
            return Ok(Expr::List(elements, span));
        }
        if s == "match" {
            let subject = self.expr()?;
            self.need("{")?;
            let mut arms = Vec::new();
            while self.peek() != Some("}") {
                let (enum_name, variant, binding) = if self.peek() == Some("_") {
                    self.take();
                    (None, None, None)
                } else {
                    let mut enum_name = self.identifier("enum name")?;
                    self.need("::")?;
                    if self.peek() != Some("=>")
                        && self.tokens.get(self.i + 1).map(String::as_str) == Some("::")
                    {
                        let namespace_enum = self.identifier("enum name")?;
                        self.need("::")?;
                        enum_name = format!("{enum_name}__{namespace_enum}");
                    }
                    let variant = self.identifier("enum variant name")?;
                    let binding = if self.peek() == Some("(") {
                        self.take();
                        let name = self.identifier("variant binding")?;
                        self.need(")")?;
                        Some(name)
                    } else {
                        None
                    };
                    (Some(enum_name), Some(variant), binding)
                };
                self.need("=>")?;
                let body = self.expr()?;
                arms.push(MatchArm {
                    enum_name,
                    variant,
                    binding,
                    body: Box::new(body),
                });
            }
            self.need("}")?;
            return Ok(Expr::Match {
                subject: Box::new(subject),
                arms,
            });
        }
        if s == "{" {
            let mut pairs = vec![];
            while self.peek() != Some("}") {
                let key = self.expr()?;
                self.need(":")?;
                let value = self.expr()?;
                pairs.push((key, value));
                if self.peek() == Some(",") {
                    self.take();
                } else {
                    break;
                }
            }
            self.need("}")?;
            return Ok(Expr::Map(pairs, span));
        }
        if s == "true" {
            return Ok(Expr::Bool(true));
        }
        if s == "false" {
            return Ok(Expr::Bool(false));
        }
        if s == "null" {
            return Ok(Expr::Null);
        }
        if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
            let value = s[1..s.len() - 1].to_owned();
            if value.contains('{') {
                let mut parts = Vec::new();
                let mut rest = value.as_str();
                while let Some(start) = rest.find('{') {
                    if start > 0 {
                        parts.push(InterpolationPart::Text(rest[..start].into()));
                    }
                    let Some(end) = rest[start + 1..].find('}') else {
                        return Err(vec![Diagnostic::at("unclosed interpolation", span)]);
                    };
                    let end = start + 1 + end;
                    let name = &rest[start + 1..end];
                    if !is_identifier(name) {
                        return Err(vec![Diagnostic::at(
                            "interpolation expects an identifier",
                            span,
                        )]);
                    }
                    parts.push(InterpolationPart::Name(name.into()));
                    rest = &rest[end + 1..];
                }
                if !rest.is_empty() {
                    parts.push(InterpolationPart::Text(rest.into()));
                }
                return Ok(Expr::Interpolated(parts, span));
            }
            return Ok(Expr::String(value));
        }
        if let Some(literal) = non_decimal_integer_literal(&s, span) {
            return literal;
        }
        if let Some(n) = s.strip_suffix("i64").or_else(|| s.strip_suffix("i32")) {
            if n.chars().enumerate().all(|(index, character)| {
                character.is_ascii_digit() || (index == 0 && character == '-')
            }) {
                let ty = if s.ends_with("i64") {
                    Type::I64
                } else {
                    Type::I32
                };
                let value = n.parse::<i64>().map_err(|_| {
                    vec![Diagnostic::at(
                        format!("{:?} integer literal out of range", ty).to_lowercase(),
                        span,
                    )]
                })?;
                if ty == Type::I32 && i32::try_from(value).is_err() {
                    return Err(vec![Diagnostic::at(
                        "i32 integer literal out of range",
                        span,
                    )]);
                }
                return Ok(Expr::Int(value, ty));
            }
        }
        if self.peek() == Some(".")
            && self
                .tokens
                .get(self.i + 1)
                .is_some_and(|part| part.chars().all(|c| c.is_ascii_digit()))
            && s.chars().all(|c| c.is_ascii_digit())
        {
            self.take();
            let fraction = self.take().unwrap();
            let literal = format!("{s}.{fraction}");
            return literal
                .parse::<f64>()
                .map(Expr::Float)
                .map_err(|_| vec![Diagnostic::at("invalid float literal", span)]);
        }
        if let Ok(n) = s.parse::<i64>() {
            if i32::try_from(n).is_err() {
                return Err(vec![Diagnostic::at(
                    "i32 integer literal out of range",
                    span,
                )]);
            } else {
                return Ok(Expr::Int(n, Type::I32));
            }
        }
        if s.strip_prefix('-')
            .unwrap_or(&s)
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_digit())
        {
            return Err(vec![Diagnostic::at("invalid numeric literal", span)]);
        }
        if !is_identifier(&s) {
            return Err(vec![Diagnostic::at("invalid identifier", span)]);
        }
        let mut s = s;
        if self.tokens.get(self.i).map(String::as_str) == Some(".")
            && self.tokens.get(self.i + 2).map(String::as_str) == Some("{")
            && self.tokens.get(self.i + 4).map(String::as_str) == Some(":")
        {
            self.take();
            let member = self.identifier("struct name")?;
            s = format!("{s}__{member}");
        }
        if self.tokens.get(self.i).map(String::as_str) == Some("::")
            && self.tokens.get(self.i + 2).map(String::as_str) == Some("::")
        {
            self.take();
            let enumeration = self.identifier("enum name")?;
            self.need("::")?;
            let variant = self.identifier("enum variant name")?;
            let arguments = if self.peek() == Some("(") {
                self.take();
                let mut arguments = Vec::new();
                if self.peek() != Some(")") {
                    arguments.push(self.expr()?);
                    while self.peek() == Some(",") {
                        self.take();
                        arguments.push(self.expr()?);
                    }
                }
                self.need(")")?;
                arguments
            } else {
                Vec::new()
            };
            Ok(Expr::EnumVariant {
                enum_name: format!("{s}__{enumeration}"),
                variant,
                arguments,
            })
        } else if self.tokens.get(self.i).map(String::as_str) == Some("::") {
            self.take();
            let variant = self.identifier("enum variant name")?;
            let arguments = if self.peek() == Some("(") {
                self.take();
                let mut arguments = Vec::new();
                if self.peek() != Some(")") {
                    arguments.push(self.expr()?);
                    while self.peek() == Some(",") {
                        self.take();
                        arguments.push(self.expr()?);
                    }
                }
                self.need(")")?;
                arguments
            } else {
                Vec::new()
            };
            Ok(Expr::EnumVariant {
                enum_name: s,
                variant,
                arguments,
            })
        } else if self.tokens.get(self.i).map(String::as_str) == Some("{")
            && self.tokens.get(self.i + 2).map(String::as_str) == Some(":")
        {
            self.take();
            let mut fields = vec![];
            while self.tokens.get(self.i).map(String::as_str) != Some("}") {
                let field = self.identifier("field name")?;
                self.need(":")?;
                let value = self.expr()?;
                fields.push((field, value));
                if self.tokens.get(self.i).map(String::as_str) == Some(",") {
                    self.take();
                }
            }
            self.need("}")?;
            Ok(Expr::StructLiteral { name: s, fields })
        } else if self.tokens.get(self.i).map(String::as_str) == Some(".")
            && self.tokens.get(self.i + 2).map(String::as_str) == Some("(")
        {
            self.take();
            let member = self.identifier("module member")?;
            self.call(format!("{s}__{member}"))
        } else if s == "bytes" && self.tokens.get(self.i).map(String::as_str) == Some("(") {
            self.need("(")?;
            let value = self
                .take()
                .ok_or_else(|| vec![Diagnostic::at("expected bytes string", self.location())])?;
            if !(value.starts_with('"') && value.ends_with('"')) {
                return Err(vec![Diagnostic::at(
                    "bytes expects a string literal",
                    self.previous_location(),
                )]);
            }
            self.need(")")?;
            Ok(Expr::Bytes(value.trim_matches('"').to_owned(), self.previous_location()))
        } else if self.tokens.get(self.i).map(String::as_str) == Some("(") {
            self.call(s)
        } else {
            Ok(Expr::Name(s))
        }
    }
    fn call(&mut self, name: String) -> Result<Expr, Vec<Diagnostic>> {
        let span = self.location();
        self.need("(")?;
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
        Ok(Expr::Call { name, arguments, span })
    }
}

fn non_decimal_integer_literal(text: &str, span: Span) -> Option<Result<Expr, Vec<Diagnostic>>> {
    let (negative, text) = match text.strip_prefix('-') {
        Some(text) => (true, text),
        None => (false, text),
    };
    let (radix, digits) = text
        .strip_prefix("0x")
        .map(|digits| (16, digits))
        .or_else(|| text.strip_prefix("0b").map(|digits| (2, digits)))?;
    let (digits, ty) = if let Some(digits) = digits.strip_suffix("i64") {
        (digits, Type::I64)
    } else if let Some(digits) = digits.strip_suffix("i32") {
        (digits, Type::I32)
    } else {
        (digits, Type::I32)
    };
    let magnitude = match i128::from_str_radix(digits, radix) {
        Ok(value) => value,
        Err(_) => return Some(Err(vec![Diagnostic::at("invalid numeric literal", span)])),
    };
    let value = if negative { -magnitude } else { magnitude };
    let in_range = match ty {
        Type::I32 => value >= i32::MIN as i128 && value <= i32::MAX as i128,
        Type::I64 => value >= i64::MIN as i128 && value <= i64::MAX as i128,
        _ => unreachable!("integer literals only have integer types"),
    };
    if !in_range {
        return Some(Err(vec![Diagnostic::at(
            format!("{:?} integer literal out of range", ty).to_lowercase(),
            span,
        )]));
    }
    Some(Ok(Expr::Int(value as i64, ty)))
}

fn is_identifier(name: &str) -> bool {
    let mut characters = name.chars();
    matches!(characters.next(), Some(character) if character.is_ascii_alphabetic() || character == '_')
        && characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
        && !matches!(
            name,
            "fn" | "struct"
                | "let"
                | "mut"
                | "return"
                | "if"
                | "else"
                | "while"
                | "true"
                | "false"
                | "i32"
                | "i64"
                | "bool"
                | "string"
        )
}
