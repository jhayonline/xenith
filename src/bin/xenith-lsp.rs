//! # Xenith Language Server
//!
//! Speaks LSP over stdio. On every edit it re-lexes and re-parses the buffer,
//! publishes the resulting diagnostics, and rebuilds an index of the symbols the
//! file defines and the places it mentions them.
//!
//! The index is name-based, not scope-aware: two locals called `i` in different
//! functions are treated as the same symbol by go-to-definition and rename.
//! Making that precise needs the resolver pass that the interpreter is also
//! waiting on; until then, prefer rename on distinctly-named symbols.

use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use xenith::builtins::registry::{
    BUILTIN_CONSTANTS, BUILTIN_FUNCTIONS, KEYWORDS_CONTROL, KEYWORDS_DECLARATION, KEYWORDS_MODULE,
    LITERALS, TYPE_NAMES, keyword_doc,
};
use xenith::error::Error as XenithError;
use xenith::lexer::Lexer;
use xenith::nodes::*;
use xenith::parser::Parser;
use xenith::position::Position as XenithPosition;
use xenith::tokens::Token;

// ---------------------------------------------------------------------------
// Document model
// ---------------------------------------------------------------------------

/// A definition the file introduces.
#[derive(Debug, Clone)]
struct Symbol {
    name: String,
    kind: SymbolKind,
    /// The whole construct, used for the document outline.
    range: Range,
    /// Just the identifier, used for go-to-definition and rename.
    selection_range: Range,
    detail: Option<String>,
    children: Vec<Symbol>,
}

/// A place the file mentions a name it did not define on that line.
#[derive(Debug, Clone)]
struct Reference {
    name: String,
    range: Range,
}

struct Document {
    text: String,
    /// Source split on newlines, kept so char columns can be converted to the
    /// UTF-16 offsets LSP positions are measured in.
    lines: Vec<String>,
    symbols: Vec<Symbol>,
    references: Vec<Reference>,
}

impl Document {
    fn new(text: String) -> Self {
        let lines = text.split('\n').map(|l| l.to_string()).collect();
        Self {
            text,
            lines,
            symbols: Vec::new(),
            references: Vec::new(),
        }
    }

    /// Converts a Xenith position (0-based line, 0-based *character* column)
    /// into an LSP position (0-based line, 0-based *UTF-16 code unit* column).
    /// They differ as soon as a line contains a non-ASCII character.
    fn to_lsp(&self, pos: &XenithPosition) -> Position {
        let character = match self.lines.get(pos.line) {
            Some(line) => line
                .chars()
                .take(pos.column)
                .map(|c| c.len_utf16() as u32)
                .sum(),
            None => pos.column as u32,
        };
        Position {
            line: pos.line as u32,
            character,
        }
    }

    fn range_of(&self, start: &XenithPosition, end: &XenithPosition) -> Range {
        Range {
            start: self.to_lsp(start),
            end: self.to_lsp(end),
        }
    }

    fn range_of_token(&self, token: &Token) -> Range {
        self.range_of(&token.position_start, &token.position_end)
    }

    /// Depth-first search for the innermost definition whose identifier covers
    /// `position`.
    fn symbol_at(&self, position: Position) -> Option<&Symbol> {
        fn search(symbols: &[Symbol], position: Position) -> Option<&Symbol> {
            for symbol in symbols {
                if let Some(hit) = search(&symbol.children, position) {
                    return Some(hit);
                }
                if contains(symbol.selection_range, position) {
                    return Some(symbol);
                }
            }
            None
        }
        search(&self.symbols, position)
    }

    /// The name under the cursor, whether it sits on a definition or a mention.
    fn name_at(&self, position: Position) -> Option<String> {
        if let Some(symbol) = self.symbol_at(position) {
            return Some(symbol.name.clone());
        }
        self.references
            .iter()
            .find(|r| contains(r.range, position))
            .map(|r| r.name.clone())
    }

    /// The bare word under the cursor, read straight out of the buffer.
    ///
    /// Falls back to this when the cursor is on something the AST walk does not
    /// index -- a keyword, a builtin the parser handles specially, or text
    /// inside a region that failed to parse.
    fn word_at(&self, position: Position) -> Option<String> {
        let line = self.lines.get(position.line as usize)?;
        let chars: Vec<char> = line.chars().collect();

        // The cursor column counts UTF-16 units; walk the line to find the
        // char index it lands on.
        let mut index = 0;
        let mut units = 0u32;
        while index < chars.len() && units < position.character {
            units += chars[index].len_utf16() as u32;
            index += 1;
        }

        let is_word = |c: char| c.is_alphanumeric() || c == '_';
        if index >= chars.len() || !is_word(chars[index]) {
            // A cursor just past the end of a word should still resolve it.
            if index == 0 || !is_word(chars[index - 1]) {
                return None;
            }
            index -= 1;
        }

        let mut start = index;
        while start > 0 && is_word(chars[start - 1]) {
            start -= 1;
        }
        let mut end = index;
        while end + 1 < chars.len() && is_word(chars[end + 1]) {
            end += 1;
        }

        Some(chars[start..=end].iter().collect())
    }

    fn find_definition(&self, name: &str) -> Option<&Symbol> {
        fn search<'a>(symbols: &'a [Symbol], name: &str) -> Option<&'a Symbol> {
            for symbol in symbols {
                if symbol.name == name {
                    return Some(symbol);
                }
                if let Some(hit) = search(&symbol.children, name) {
                    return Some(hit);
                }
            }
            None
        }
        search(&self.symbols, name)
    }

    /// Every definition and mention of `name`, for references and rename.
    fn occurrences(&self, name: &str) -> Vec<Range> {
        fn walk(symbols: &[Symbol], name: &str, out: &mut Vec<Range>) {
            for symbol in symbols {
                if symbol.name == name {
                    out.push(symbol.selection_range);
                }
                walk(&symbol.children, name, out);
            }
        }
        let mut out = Vec::new();
        walk(&self.symbols, name, &mut out);
        out.extend(
            self.references
                .iter()
                .filter(|r| r.name == name)
                .map(|r| r.range),
        );
        out
    }
}

fn contains(range: Range, position: Position) -> bool {
    if position.line < range.start.line || position.line > range.end.line {
        return false;
    }
    if position.line == range.start.line && position.character < range.start.character {
        return false;
    }
    if position.line == range.end.line && position.character > range.end.character {
        return false;
    }
    true
}

// ---------------------------------------------------------------------------
// Analysis
// ---------------------------------------------------------------------------

/// Lexes and parses `text`, returning the diagnostics and, when parsing got far
/// enough to produce a tree, the symbols and references it contains.
fn analyze(uri: &str, doc: &mut Document) -> Vec<Diagnostic> {
    let mut lexer = Lexer::new(uri.to_string(), doc.text.clone());
    let tokens = match lexer.make_tokens() {
        Ok(tokens) => tokens,
        Err(e) => return vec![to_diagnostic(doc, &e.base)],
    };

    let mut parser = Parser::new(tokens);
    let parse_result = parser.parse();

    let mut diagnostics = Vec::new();
    if let Some(error) = &parse_result.error {
        diagnostics.push(to_diagnostic(doc, error));
    }

    // Type errors, from the same static pass the interpreter runs before
    // executing, so the editor reports exactly what the command line will.
    // Only run it on a file that parsed cleanly: checking a partial tree
    // produces errors about code the user is still in the middle of writing.
    if parse_result.error.is_none() {
        if let Some(node) = &parse_result.node {
            for error in xenith::checker::check(node, &parser.type_aliases) {
                diagnostics.push(to_diagnostic(doc, &error));
            }
        }
    }

    // A parse error still leaves a partial tree; index whatever survived so
    // completion and hover keep working while the file is mid-edit.
    let mut symbols = Vec::new();
    let mut references = Vec::new();
    if let Some(node) = &parse_result.node {
        let mut collector = Collector {
            doc,
            references: &mut references,
        };
        collector.walk(node, &mut symbols);
    }

    doc.symbols = symbols;
    doc.references = references;
    diagnostics
}

fn to_diagnostic(doc: &Document, error: &XenithError) -> Diagnostic {
    // A zero-width range renders as an invisible squiggle; widen it to a
    // single character so the underline is actually visible.
    let mut range = doc.range_of(&error.position_start, &error.position_end);
    if range.start == range.end {
        range.end.character += 1;
    }

    let mut message = if error.details.is_empty() {
        error.error_name.clone()
    } else {
        format!("{}: {}", error.error_name, error.details)
    };
    if let Some(help) = &error.help {
        message.push_str(&format!("\n\nhelp: {}", help));
    }

    Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String(error.code.clone())),
        source: Some("xenith".to_string()),
        message,
        ..Default::default()
    }
}

/// Walks the AST accumulating definitions (into a caller-supplied tree) and
/// mentions (into a flat list).
struct Collector<'a> {
    doc: &'a Document,
    references: &'a mut Vec<Reference>,
}

impl Collector<'_> {
    fn walk(&mut self, node: &Node, out: &mut Vec<Symbol>) {
        match node {
            Node::List(n) => {
                for element in &n.element_nodes {
                    self.walk(element, out);
                }
            }

            Node::VarAssign(n) => {
                if let Some(name) = n.variable_name_token.value.as_ref() {
                    if n.is_declaration {
                        out.push(Symbol {
                            name: name.clone(),
                            kind: if n.is_constant {
                                SymbolKind::CONSTANT
                            } else {
                                SymbolKind::VARIABLE
                            },
                            range: self.doc.range_of(&n.position_start, &n.position_end),
                            selection_range: self.doc.range_of_token(&n.variable_name_token),
                            detail: n.var_type.as_ref().map(|t| t.to_string()),
                            children: Vec::new(),
                        });
                    } else {
                        // A bare `x = ...` mentions an existing binding.
                        self.reference(name, &n.variable_name_token);
                    }
                }
                self.walk(&n.value_node, out);
            }

            Node::FuncDef(n) => {
                let mut children = Vec::new();
                for (index, param) in n.param_names.iter().enumerate() {
                    if let Some(name) = param.value.as_ref() {
                        children.push(Symbol {
                            name: name.clone(),
                            kind: SymbolKind::VARIABLE,
                            range: self.doc.range_of_token(param),
                            selection_range: self.doc.range_of_token(param),
                            detail: n.param_types.get(index).map(|t| t.to_string()),
                            children: Vec::new(),
                        });
                    }
                }
                self.walk(&n.body_node, &mut children);

                match &n.variable_name_token {
                    Some(token) => {
                        let name = token.value.clone().unwrap_or_default();
                        out.push(Symbol {
                            name,
                            kind: SymbolKind::FUNCTION,
                            range: self.doc.range_of(&n.position_start, &n.position_end),
                            selection_range: self.doc.range_of_token(token),
                            detail: Some(signature_of(n)),
                            children,
                        });
                    }
                    // An anonymous method has nothing to hang its body on, so
                    // its inner definitions are lifted to the enclosing scope.
                    None => out.extend(children),
                }
            }

            Node::StructDef(n) => {
                let children = n
                    .fields
                    .iter()
                    .filter_map(|field| {
                        let name = field.name.value.clone()?;
                        Some(Symbol {
                            name,
                            kind: SymbolKind::FIELD,
                            range: self.doc.range_of(&field.position_start, &field.position_end),
                            selection_range: self.doc.range_of_token(&field.name),
                            detail: Some(field.field_type.to_string()),
                            children: Vec::new(),
                        })
                    })
                    .collect();

                let fields = n
                    .fields
                    .iter()
                    .map(|f| {
                        format!(
                            "{}: {}",
                            f.name.value.clone().unwrap_or_default(),
                            f.field_type.to_string()
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");

                out.push(Symbol {
                    name: n.name.value.clone().unwrap_or_default(),
                    kind: SymbolKind::STRUCT,
                    range: self.doc.range_of(&n.position_start, &n.position_end),
                    selection_range: self.doc.range_of_token(&n.name),
                    detail: Some(format!("struct {{ {} }}", fields)),
                    children,
                });
            }

            Node::EnumDef(n) => {
                let children = n
                    .variants
                    .iter()
                    .filter_map(|variant| {
                        let name = variant.name.value.clone()?;
                        let payload = variant
                            .payload_types
                            .iter()
                            .map(|t| t.to_string())
                            .collect::<Vec<_>>()
                            .join(", ");
                        Some(Symbol {
                            name,
                            kind: SymbolKind::ENUM_MEMBER,
                            range: self
                                .doc
                                .range_of(&variant.position_start, &variant.position_end),
                            selection_range: self.doc.range_of_token(&variant.name),
                            detail: (!payload.is_empty()).then(|| format!("({})", payload)),
                            children: Vec::new(),
                        })
                    })
                    .collect();

                let variants = n
                    .variants
                    .iter()
                    .map(|v| v.name.value.clone().unwrap_or_default())
                    .collect::<Vec<_>>()
                    .join(", ");

                out.push(Symbol {
                    name: n.name.value.clone().unwrap_or_default(),
                    kind: SymbolKind::ENUM,
                    range: self.doc.range_of(&n.position_start, &n.position_end),
                    selection_range: self.doc.range_of_token(&n.name),
                    detail: Some(format!("enum {{ {} }}", variants)),
                    children,
                });
            }

            Node::EnumVariant(n) => {
                for argument in &n.arguments {
                    self.walk(argument, out);
                }
            }

            Node::Match(n) => {
                self.walk(&n.subject, out);
                for arm in &n.arms {
                    // A pattern's bindings are local to its arm, so they go in
                    // the arm's own list rather than the enclosing scope's.
                    let mut arm_symbols = Vec::new();
                    for pattern in &arm.patterns {
                        let mut bound = Vec::new();
                        pattern.bindings(&mut bound);
                        for token in bound {
                            if let Some(name) = token.value.clone() {
                                arm_symbols.push(Symbol {
                                    name,
                                    kind: SymbolKind::VARIABLE,
                                    range: self.doc.range_of_token(&token),
                                    selection_range: self.doc.range_of_token(&token),
                                    detail: Some("bound by a pattern".to_string()),
                                    children: Vec::new(),
                                });
                            }
                        }
                    }
                    if let Some(guard) = &arm.guard {
                        self.walk(guard, &mut arm_symbols);
                    }
                    self.walk(&arm.body, &mut arm_symbols);
                    out.extend(arm_symbols);
                }
            }

            Node::TypeAlias(n) => {
                out.push(Symbol {
                    name: n.name.value.clone().unwrap_or_default(),
                    kind: SymbolKind::TYPE_PARAMETER,
                    range: self.doc.range_of(&n.position_start, &n.position_end),
                    selection_range: self.doc.range_of_token(&n.name),
                    detail: Some(format!("type = {}", n.alias_type.to_string())),
                    children: Vec::new(),
                });
            }

            Node::Destructure(n) => {
                self.destructure(&n.patterns, out);
                self.walk(&n.value_node, out);
            }
            Node::DestructurePattern(n) => {
                self.destructure(std::slice::from_ref(&n.pattern), out);
            }

            Node::VarAccess(n) => {
                if let Some(name) = n.variable_name_token.value.as_ref() {
                    self.reference(name, &n.variable_name_token);
                }
            }

            Node::StructInstantiation(n) => {
                // The type name is a mention of the struct definition.
                let range = self.doc.range_of(&n.position_start, &n.position_end);
                self.references.push(Reference {
                    name: n.struct_name.clone(),
                    range: Range {
                        start: range.start,
                        end: Position {
                            line: range.start.line,
                            character: range.start.character
                                + n.struct_name.chars().map(|c| c.len_utf16() as u32).sum::<u32>(),
                        },
                    },
                });
                for (_, value) in &n.fields {
                    self.walk(value, out);
                }
            }

            Node::BinaryOperator(n) => {
                self.walk(&n.left_node, out);
                self.walk(&n.right_node, out);
            }
            Node::UnaryOp(n) => self.walk(&n.node, out),
            Node::Ternary(n) => {
                self.walk(&n.condition, out);
                self.walk(&n.true_expression, out);
                self.walk(&n.false_expression, out);
            }
            Node::If(n) => {
                for (condition, body) in &n.cases {
                    self.walk(condition, out);
                    self.walk(body, out);
                }
                if let Some((body, _)) = &n.else_case {
                    self.walk(body, out);
                }
            }
            Node::For(n) => {
                // `for k, v in map` encodes both names in one token as "(k,v)".
                let raw = n.variable_name_token.value.clone().unwrap_or_default();
                let range = self.doc.range_of_token(&n.variable_name_token);
                for name in raw.trim_matches(['(', ')']).split(',') {
                    let name = name.trim();
                    if !name.is_empty() {
                        out.push(Symbol {
                            name: name.to_string(),
                            kind: SymbolKind::VARIABLE,
                            range,
                            selection_range: range,
                            detail: Some("loop variable".to_string()),
                            children: Vec::new(),
                        });
                    }
                }
                self.walk(&n.iterable_node, out);
                self.walk(&n.body_node, out);
            }
            Node::ForClassic(n) => {
                if let Some(init) = &n.init_node {
                    self.walk(init, out);
                }
                if let Some(condition) = &n.condition_node {
                    self.walk(condition, out);
                }
                if let Some(step) = &n.step_node {
                    self.walk(step, out);
                }
                self.walk(&n.body_node, out);
            }
            Node::While(n) => {
                self.walk(&n.condition_node, out);
                self.walk(&n.body_node, out);
            }
            Node::Call(n) => {
                self.walk(&n.node_to_call, out);
                for argument in &n.argument_nodes {
                    self.walk(argument, out);
                }
            }
            Node::Return(n) => {
                if let Some(value) = &n.node_to_return {
                    self.walk(value, out);
                }
            }
            Node::MethodAccess(n) => self.walk(&n.object, out),
            Node::Map(n) => {
                for pair in &n.pairs {
                    self.walk(&pair.key_node, out);
                    self.walk(&pair.value_node, out);
                }
            }
            Node::TupleLiteral(n) => {
                for element in &n.elements {
                    self.walk(element, out);
                }
            }
            Node::Panic(n) => self.walk(&n.message_node, out),
            Node::Export(n) => self.walk(&n.node, out),

            Node::Number(_)
            | Node::String(_)
            | Node::InterpolatedString(_)
            | Node::BoolLiteral(_)
            | Node::NullLiteral(_)
            | Node::Continue(_)
            | Node::Break(_)
            | Node::Grab(_) => {}
        }
    }

    fn destructure(&mut self, patterns: &[DestructurePattern], out: &mut Vec<Symbol>) {
        for pattern in patterns {
            match pattern {
                DestructurePattern::Variable(token) => {
                    if let Some(name) = token.value.as_ref() {
                        out.push(Symbol {
                            name: name.clone(),
                            kind: SymbolKind::VARIABLE,
                            range: self.doc.range_of_token(token),
                            selection_range: self.doc.range_of_token(token),
                            detail: None,
                            children: Vec::new(),
                        });
                    }
                }
                DestructurePattern::Tuple(nested) => self.destructure(nested, out),
                DestructurePattern::Ignore => {}
            }
        }
    }

    fn reference(&mut self, name: &str, token: &Token) {
        self.references.push(Reference {
            name: name.to_string(),
            range: self.doc.range_of_token(token),
        });
    }
}

fn signature_of(node: &FuncDefNode) -> String {
    let params = node
        .param_names
        .iter()
        .enumerate()
        .map(|(index, param)| {
            let name = param.value.clone().unwrap_or_default();
            match node.param_types.get(index) {
                Some(t) => format!("{}: {}", name, t.to_string()),
                None => name,
            }
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        "method {}({}) -> {}",
        node.variable_name_token
            .as_ref()
            .and_then(|t| t.value.clone())
            .unwrap_or_default(),
        params,
        node.return_type.to_string()
    )
}

fn to_document_symbol(symbol: &Symbol) -> DocumentSymbol {
    #[allow(deprecated)]
    DocumentSymbol {
        name: symbol.name.clone(),
        detail: symbol.detail.clone(),
        kind: symbol.kind,
        tags: None,
        deprecated: None,
        range: symbol.range,
        selection_range: symbol.selection_range,
        children: if symbol.children.is_empty() {
            None
        } else {
            Some(symbol.children.iter().map(to_document_symbol).collect())
        },
    }
}

// ---------------------------------------------------------------------------
// Server
// ---------------------------------------------------------------------------

struct XenithLanguageServer {
    client: Client,
    documents: Arc<DashMap<String, Document>>,
}

impl XenithLanguageServer {
    async fn refresh(&self, uri: Url, text: String, version: i32) {
        let key = uri.to_string();
        let mut document = Document::new(text);
        let diagnostics = analyze(&key, &mut document);
        self.documents.insert(key, document);

        self.client
            .publish_diagnostics(uri, diagnostics, Some(version))
            .await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for XenithLanguageServer {
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "xenith-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![".".to_string()]),
                    ..Default::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "xenith-lsp ready")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let document = params.text_document;
        self.refresh(document.uri, document.text, document.version)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        // Sync is FULL, so the single change carries the whole buffer.
        if let Some(change) = params.content_changes.into_iter().next() {
            self.refresh(
                params.text_document.uri,
                change.text,
                params.text_document.version,
            )
            .await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.documents.remove(&uri.to_string());
        // Clear the squiggles; nothing is watching this file any more.
        self.client.publish_diagnostics(uri, Vec::new(), None).await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let location = params.text_document_position_params;
        let Some(document) = self.documents.get(&location.text_document.uri.to_string()) else {
            return Ok(None);
        };
        let Some(name) = document
            .name_at(location.position)
            .or_else(|| document.word_at(location.position))
        else {
            return Ok(None);
        };

        let markdown = if let Some(symbol) = document.find_definition(&name) {
            match &symbol.detail {
                Some(detail) => format!("```xenith\n{}\n```", detail),
                None => format!("```xenith\n{}\n```", symbol.name),
            }
        } else if let Some(builtin) = BUILTIN_FUNCTIONS.iter().find(|b| b.name == name) {
            format!("```xenith\n{}\n```\n\n{}", builtin.signature, builtin.doc)
        } else if let Some(constant) = BUILTIN_CONSTANTS.iter().find(|c| c.name == name) {
            format!(
                "```xenith\n{}: {}\n```\n\n{}",
                constant.name, constant.type_name, constant.doc
            )
        } else if let Some(doc) = keyword_doc(&name) {
            format!("**{}**\n\n{}", name, doc)
        } else {
            return Ok(None);
        };

        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: markdown,
            }),
            range: None,
        }))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let location = params.text_document_position_params;
        let uri = location.text_document.uri;
        let Some(document) = self.documents.get(&uri.to_string()) else {
            return Ok(None);
        };
        let Some(name) = document
            .name_at(location.position)
            .or_else(|| document.word_at(location.position))
        else {
            return Ok(None);
        };
        let Some(symbol) = document.find_definition(&name) else {
            return Ok(None);
        };

        Ok(Some(GotoDefinitionResponse::Scalar(Location {
            uri,
            range: symbol.selection_range,
        })))
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let location = params.text_document_position;
        let uri = location.text_document.uri;
        let Some(document) = self.documents.get(&uri.to_string()) else {
            return Ok(None);
        };
        let Some(name) = document
            .name_at(location.position)
            .or_else(|| document.word_at(location.position))
        else {
            return Ok(None);
        };

        Ok(Some(
            document
                .occurrences(&name)
                .into_iter()
                .map(|range| Location {
                    uri: uri.clone(),
                    range,
                })
                .collect(),
        ))
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let location = params.text_document_position;
        let uri = location.text_document.uri;
        let Some(document) = self.documents.get(&uri.to_string()) else {
            return Ok(None);
        };
        let Some(name) = document.name_at(location.position) else {
            return Ok(None);
        };

        let edits: Vec<TextEdit> = document
            .occurrences(&name)
            .into_iter()
            .map(|range| TextEdit {
                range,
                new_text: params.new_name.clone(),
            })
            .collect();

        if edits.is_empty() {
            return Ok(None);
        }

        let mut changes = HashMap::new();
        changes.insert(uri, edits);
        Ok(Some(WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        }))
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let Some(document) = self.documents.get(&params.text_document.uri.to_string()) else {
            return Ok(None);
        };
        Ok(Some(DocumentSymbolResponse::Nested(
            document.symbols.iter().map(to_document_symbol).collect(),
        )))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let mut items = Vec::new();

        let keywords = KEYWORDS_DECLARATION
            .iter()
            .chain(KEYWORDS_CONTROL)
            .chain(KEYWORDS_MODULE);
        for keyword in keywords {
            items.push(CompletionItem {
                label: keyword.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: keyword_doc(keyword).map(|d| d.to_string()),
                ..Default::default()
            });
        }

        for literal in LITERALS {
            items.push(CompletionItem {
                label: literal.to_string(),
                kind: Some(CompletionItemKind::VALUE),
                ..Default::default()
            });
        }

        for name in TYPE_NAMES {
            items.push(CompletionItem {
                label: name.to_string(),
                kind: Some(CompletionItemKind::CLASS),
                detail: Some("built-in type".to_string()),
                ..Default::default()
            });
        }

        for builtin in BUILTIN_FUNCTIONS {
            items.push(CompletionItem {
                label: builtin.name.to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some(builtin.signature.to_string()),
                documentation: Some(Documentation::String(builtin.doc.to_string())),
                ..Default::default()
            });
        }

        for constant in BUILTIN_CONSTANTS {
            items.push(CompletionItem {
                label: constant.name.to_string(),
                kind: Some(CompletionItemKind::CONSTANT),
                detail: Some(constant.type_name.to_string()),
                documentation: Some(Documentation::String(constant.doc.to_string())),
                ..Default::default()
            });
        }

        if let Some(document) = self
            .documents
            .get(&params.text_document_position.text_document.uri.to_string())
        {
            fn add(symbols: &[Symbol], items: &mut Vec<CompletionItem>) {
                for symbol in symbols {
                    items.push(CompletionItem {
                        label: symbol.name.clone(),
                        kind: Some(match symbol.kind {
                            SymbolKind::FUNCTION => CompletionItemKind::FUNCTION,
                            SymbolKind::STRUCT => CompletionItemKind::STRUCT,
                            SymbolKind::CONSTANT => CompletionItemKind::CONSTANT,
                            SymbolKind::FIELD => CompletionItemKind::FIELD,
                            SymbolKind::TYPE_PARAMETER => CompletionItemKind::CLASS,
                            _ => CompletionItemKind::VARIABLE,
                        }),
                        detail: symbol.detail.clone(),
                        ..Default::default()
                    });
                    add(&symbol.children, items);
                }
            }
            add(&document.symbols, &mut items);
        }

        Ok(Some(CompletionResponse::Array(items)))
    }
}

#[tokio::main]
async fn main() {
    let (service, socket) = LspService::build(|client| XenithLanguageServer {
        client,
        documents: Arc::new(DashMap::new()),
    })
    .finish();

    Server::new(tokio::io::stdin(), tokio::io::stdout(), socket)
        .serve(service)
        .await;
}
