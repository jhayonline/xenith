//! Xenith Language Server Protocol implementation

use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

// Import Xenith modules
use xenith::lexer::Lexer;
use xenith::nodes::Node;
use xenith::parser::Parser;
use xenith::position::Position as XenithPosition;

// Document store
#[derive(Clone)]
struct Document {
    uri: String,
    text: String,
    ast: Option<Node>,
    symbols: Vec<Symbol>,
    version: i32,
}

#[derive(Debug, Clone)]
struct Symbol {
    name: String,
    kind: SymbolKind,
    range: Range,
    location: Location,
    detail: Option<String>,
}

// LSP Backend
struct XenithLanguageServer {
    client: Client,
    documents: Arc<DashMap<String, Document>>,
}

// Helper function to convert Xenith Position to LSP Position
fn to_lsp_position(pos: &XenithPosition) -> Position {
    Position {
        line: pos.line as u32,
        character: pos.column as u32,
    }
}

// Helper function to convert LSP Position to Xenith Position (approximate)
fn from_lsp_position(pos: Position, file_name: &str, file_text: &str) -> XenithPosition {
    XenithPosition::new(
        pos.character as usize,
        pos.line as usize,
        pos.character as usize,
        file_name,
        file_text,
    )
}

// Helper to create LSP Range from Xenith positions
fn to_lsp_range(start: &XenithPosition, end: &XenithPosition) -> Range {
    Range {
        start: to_lsp_position(start),
        end: to_lsp_position(end),
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for XenithLanguageServer {
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
        self.client
            .log_message(MessageType::INFO, "Xenith Language Server initialized")
            .await;

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![":".to_string(), ".".to_string()]),
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
            .log_message(MessageType::INFO, "Xenith LSP ready")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        let text = params.text_document.text;
        let version = params.text_document.version;

        self.parse_and_update_document(&uri, &text, version).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        let version = params.text_document.version;

        if let Some(change) = params.content_changes.first() {
            let text = change.text.clone();
            self.parse_and_update_document(&uri, &text, version).await;
        }
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .to_string();
        let position = params.text_document_position_params.position;

        if let Some(doc) = self.documents.get(&uri) {
            let xenith_pos = from_lsp_position(position, &uri, &doc.text);
            if let Some(symbol) = self.find_symbol_at_position(&doc, &xenith_pos) {
                let content = if let Some(detail) = &symbol.detail {
                    format!("**{}**\n\n```xenith\n{}\n```", symbol.name, detail)
                } else {
                    format!("**{}**", symbol.name)
                };

                return Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: content,
                    }),
                    range: Some(symbol.range),
                }));
            }
        }

        Ok(None)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .to_string();
        let position = params.text_document_position_params.position;

        if let Some(doc) = self.documents.get(&uri) {
            let xenith_pos = from_lsp_position(position, &uri, &doc.text);
            if let Some(symbol) = self.find_symbol_at_position(&doc, &xenith_pos) {
                return Ok(Some(GotoDefinitionResponse::Scalar(symbol.location)));
            }
        }

        Ok(None)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri.to_string();
        let position = params.text_document_position.position;

        if let Some(doc) = self.documents.get(&uri) {
            let xenith_pos = from_lsp_position(position, &uri, &doc.text);
            if let Some(symbol) = self.find_symbol_at_position(&doc, &xenith_pos) {
                let mut locations = Vec::new();
                for doc_ref in self.documents.iter() {
                    let doc = doc_ref.value();
                    for sym in &doc.symbols {
                        if sym.name == symbol.name {
                            locations.push(sym.location.clone());
                        }
                    }
                }
                if !locations.is_empty() {
                    return Ok(Some(locations));
                }
            }
        }

        Ok(None)
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri.to_string();
        let position = params.text_document_position.position;
        let new_name = params.new_name;

        if let Some(doc) = self.documents.get(&uri) {
            let xenith_pos = from_lsp_position(position, &uri, &doc.text);
            if let Some(symbol) = self.find_symbol_at_position(&doc, &xenith_pos) {
                let mut changes = HashMap::new();
                let mut text_edit = Vec::new();

                for doc_ref in self.documents.iter() {
                    let doc = doc_ref.value();
                    let doc_uri = Url::parse(&doc.uri).unwrap();

                    for sym in &doc.symbols {
                        if sym.name == symbol.name {
                            text_edit.push(TextEdit {
                                range: sym.range,
                                new_text: new_name.clone(),
                            });
                        }
                    }

                    if !text_edit.is_empty() {
                        changes.insert(doc_uri, text_edit.clone());
                        text_edit.clear();
                    }
                }

                return Ok(Some(WorkspaceEdit {
                    changes: Some(changes),
                    ..Default::default()
                }));
            }
        }

        Ok(None)
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri.to_string();

        if let Some(doc) = self.documents.get(&uri) {
            let symbols: Vec<DocumentSymbol> = doc
                .symbols
                .iter()
                .map(|sym| DocumentSymbol {
                    name: sym.name.clone(),
                    detail: sym.detail.clone(),
                    kind: sym.kind,
                    tags: None,
                    deprecated: None,
                    range: sym.range,
                    selection_range: sym.range,
                    children: None,
                })
                .collect();

            if !symbols.is_empty() {
                return Ok(Some(DocumentSymbolResponse::Nested(symbols)));
            }
        }

        Ok(None)
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let mut items = Vec::new();

        // Keywords
        let keywords = vec![
            "let",
            "const",
            "method",
            "release",
            "when",
            "or",
            "otherwise",
            "for",
            "to",
            "step",
            "while",
            "skip",
            "stop",
            "match",
            "in",
            "try",
            "catch",
            "panic",
            "grab",
            "export",
            "as",
            "from",
            "struct",
            "impl",
            "type",
            "true",
            "false",
            "null",
        ];

        for kw in keywords {
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("Xenith keyword".to_string()),
                insert_text: Some(format!("{} ", kw)),
                ..Default::default()
            });
        }

        // Built-in types
        let types = vec!["int", "float", "string", "bool", "list", "map", "json"];
        for typ in types {
            items.push(CompletionItem {
                label: typ.to_string(),
                kind: Some(CompletionItemKind::TYPE_PARAMETER),
                detail: Some("Xenith built-in type".to_string()),
                insert_text: Some(format!("{} ", typ)),
                ..Default::default()
            });
        }

        // Built-in functions
        let builtins = vec![
            "echo",
            "ret",
            "input",
            "input_int",
            "clear",
            "len",
            "run",
            "format",
        ];
        for func in builtins {
            items.push(CompletionItem {
                label: func.to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some("Xenith built-in function".to_string()),
                insert_text: Some(format!("{}()", func)),
                ..Default::default()
            });
        }

        // User-defined symbols from current document
        let uri = params.text_document_position.text_document.uri.to_string();
        if let Some(doc) = self.documents.get(&uri) {
            for sym in &doc.symbols {
                let kind = match sym.kind {
                    SymbolKind::FUNCTION => CompletionItemKind::FUNCTION,
                    SymbolKind::VARIABLE => CompletionItemKind::VARIABLE,
                    SymbolKind::STRUCT => CompletionItemKind::STRUCT,
                    _ => CompletionItemKind::FIELD,
                };
                items.push(CompletionItem {
                    label: sym.name.clone(),
                    kind: Some(kind),
                    detail: sym.detail.clone(),
                    insert_text: Some(sym.name.clone()),
                    ..Default::default()
                });
            }
        }

        if items.is_empty() {
            Ok(None)
        } else {
            Ok(Some(CompletionResponse::Array(items)))
        }
    }
}

impl XenithLanguageServer {
    async fn parse_and_update_document(&self, uri: &str, text: &str, version: i32) {
        let (ast, symbols) = self.parse_and_collect_symbols(uri, text);

        let doc = Document {
            uri: uri.to_string(),
            text: text.to_string(),
            ast,
            symbols,
            version,
        };

        self.documents.insert(uri.to_string(), doc);
    }

    fn parse_and_collect_symbols(&self, uri: &str, text: &str) -> (Option<Node>, Vec<Symbol>) {
        let mut lexer = Lexer::new(uri.to_string(), text.to_string());
        let tokens = match lexer.make_tokens() {
            Ok(t) => t,
            Err(_) => return (None, Vec::new()),
        };

        let mut parser = Parser::new(tokens);
        let parse_result = parser.parse();

        let mut symbols = Vec::new();

        if let Some(node) = &parse_result.node {
            self.collect_symbols(node, &mut symbols, uri);
        }

        (parse_result.node, symbols)
    }

    fn collect_symbols(&self, node: &Node, symbols: &mut Vec<Symbol>, uri: &str) {
        match node {
            Node::VarAssign(n) => {
                if let Some(name) = n.variable_name_token.value.as_ref() {
                    let range = to_lsp_range(&n.position_start, &n.position_end);

                    symbols.push(Symbol {
                        name: name.clone(),
                        kind: SymbolKind::VARIABLE,
                        range,
                        location: Location {
                            uri: Url::parse(uri).unwrap(),
                            range,
                        },
                        detail: n
                            .var_type
                            .as_ref()
                            .map(|t| format!("type: {}", t.to_string())),
                    });
                }
            }
            Node::FuncDef(n) => {
                if let Some(token) = &n.variable_name_token {
                    if let Some(name) = token.value.as_ref() {
                        let range = to_lsp_range(&n.position_start, &n.position_end);

                        let param_str = n
                            .param_names
                            .iter()
                            .map(|p| p.value.as_ref().unwrap().clone())
                            .collect::<Vec<_>>()
                            .join(", ");

                        symbols.push(Symbol {
                            name: name.clone(),
                            kind: SymbolKind::FUNCTION,
                            range,
                            location: Location {
                                uri: Url::parse(uri).unwrap(),
                                range,
                            },
                            detail: Some(format!(
                                "({}) -> {}",
                                param_str,
                                n.return_type.to_string()
                            )),
                        });
                    }
                }
            }
            Node::StructDef(n) => {
                if let Some(name) = n.name.value.as_ref() {
                    let range = to_lsp_range(&n.position_start, &n.position_end);

                    let fields: Vec<String> = n
                        .fields
                        .iter()
                        .map(|f| {
                            format!(
                                "{}: {}",
                                f.name.value.as_ref().unwrap(),
                                f.field_type.to_string()
                            )
                        })
                        .collect();

                    symbols.push(Symbol {
                        name: name.clone(),
                        kind: SymbolKind::STRUCT,
                        range,
                        location: Location {
                            uri: Url::parse(uri).unwrap(),
                            range,
                        },
                        detail: Some(format!("{{ {} }}", fields.join(", "))),
                    });
                }
            }
            _ => {}
        }
    }

    fn find_symbol_at_position(&self, doc: &Document, position: &XenithPosition) -> Option<Symbol> {
        let line = position.line;
        let col = position.column;

        doc.symbols
            .iter()
            .find(|sym| {
                let start_line = sym.range.start.line as usize;
                let end_line = sym.range.end.line as usize;
                let start_col = sym.range.start.character as usize;
                let end_col = sym.range.end.character as usize;

                if line >= start_line && line <= end_line {
                    if line == start_line && col < start_col {
                        return false;
                    }
                    if line == end_line && col > end_col {
                        return false;
                    }
                    true
                } else {
                    false
                }
            })
            .cloned()
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::build(|client| XenithLanguageServer {
        client,
        documents: Arc::new(DashMap::new()),
    })
    .finish();

    Server::new(stdin, stdout, socket).serve(service).await;
}
