//! The checker records the type it inferred for each expression. A later
//! compiler reads this to choose a type-specialised opcode, so what matters is
//! that a recorded type is never wrong -- an absent one costs speed, a wrong one
//! costs correctness.

use xenith::check_source_typed;
use xenith::nodes::{Node, NodeId};
use xenith::types::Type;

/// The type recorded for the first node matching a predicate, in pre-order.
fn find(ast: &Node, table: &xenith::type_table::TypeTable, want: &dyn Fn(&Node) -> bool) -> Type {
    fn walk<'a>(node: &'a Node, want: &dyn Fn(&Node) -> bool) -> Option<&'a Node> {
        if want(node) {
            return Some(node);
        }
        node.children().into_iter().find_map(|c| walk(c, want))
    }

    let found = walk(ast, want).expect("no matching node");
    table.get(found.id()).clone()
}

#[test]
fn an_int_addition_is_recorded_as_int() {
    let (errors, table, ast) =
        check_source_typed("<test>", "let x: int = 1 + 2\n").expect("should parse");
    assert!(errors.is_empty(), "{errors:?}");

    let ty = find(&ast, &table, &|n| matches!(n, Node::BinaryOperator(_)));
    assert_eq!(ty, Type::Int);
}

#[test]
fn a_float_addition_is_recorded_as_float() {
    let (errors, table, ast) =
        check_source_typed("<test>", "let x: float = 1.0 + 2.0\n").expect("should parse");
    assert!(errors.is_empty(), "{errors:?}");

    let ty = find(&ast, &table, &|n| matches!(n, Node::BinaryOperator(_)));
    assert_eq!(ty, Type::Float);
}

#[test]
fn a_builtin_call_stays_unknown() {
    // `len` accepts several argument types in ways `Type` cannot describe, so
    // the checker leaves it `Unknown` and the compiler must emit a generic
    // opcode. This is the degradation path and it must keep working.
    let (_, table, ast) =
        check_source_typed("<test>", "let n = len([1, 2, 3])\n").expect("should parse");

    let ty = find(&ast, &table, &|n| matches!(n, Node::Call(_)));
    assert_eq!(ty, Type::Unknown);
}

#[test]
fn an_unnumbered_node_reads_as_unknown() {
    let (_, table, _) = check_source_typed("<test>", "let x: int = 1\n").expect("should parse");
    assert_eq!(*table.get(NodeId::UNSET), Type::Unknown);
}
