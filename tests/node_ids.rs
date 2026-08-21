//! Every expression node the parser produces must carry a distinct id, because
//! the checker records the type it inferred against that id and the compiler
//! looks it up. A duplicate id is a wrong type, which is a wrong opcode.
//!
//! Ids may have gaps. A speculative parse that backtracks -- `try_parse_field_access`
//! is one -- has already taken an id for a node it then throws away. That costs
//! an unused slot in a table sized by `node_count`, and nothing else; what must
//! hold is that no two live nodes share an id.

use std::collections::HashSet;

use xenith::lexer::Lexer;
use xenith::nodes::{Node, NodeId};
use xenith::parser::Parser;

fn parse(source: &str) -> (Node, u32) {
    let mut lexer = Lexer::new("<test>".to_string(), source.to_string());
    let tokens = lexer.make_tokens().expect("should lex");
    let mut parser = Parser::new(tokens);
    let result = parser.parse();
    assert!(result.error.is_none(), "should parse: {:?}", result.error);
    (result.node.expect("should produce a node"), parser.node_count())
}

fn collect(node: &Node, seen: &mut Vec<NodeId>) {
    let id = node.id();
    if id != NodeId::UNSET {
        seen.push(id);
    }
    for child in node.children() {
        collect(child, seen);
    }
}

#[test]
fn ids_are_distinct() {
    let (ast, count) = parse(
        "let x: int = 1 + 2\n\
         let y: string = \"a{x}b\"\n\
         method f(n: int) -> int => n * n\n\
         echo(\"{f(x)} {y}\")\n",
    );

    let mut seen = Vec::new();
    collect(&ast, &mut seen);

    assert!(!seen.is_empty(), "some node should have been numbered");

    let unique: HashSet<_> = seen.iter().copied().collect();
    assert_eq!(unique.len(), seen.len(), "ids must be distinct");

    for id in &seen {
        assert!(id.0 < count, "{id:?} is outside the reported node count");
    }
}

#[test]
fn an_empty_program_is_one_empty_block() {
    // `statements()` builds a `ListNode` whether or not it found any, and a
    // list is a numbered node, so even an empty file has exactly one id: the
    // block itself, with no children under it.
    let (ast, count) = parse("\n");
    let mut seen = Vec::new();
    collect(&ast, &mut seen);

    assert_eq!(seen.len(), 1, "only the block itself");
    assert!(seen[0].0 < count);
    assert!(ast.children().is_empty());
}
