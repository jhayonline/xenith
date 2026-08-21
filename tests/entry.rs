use xenith::entry::{shape_of, ProgramShape};
use xenith::lexer::Lexer;
use xenith::nodes::Node;
use xenith::parser::Parser;

fn parse(source: &str) -> Node {
    let mut lexer = Lexer::new("<test>".to_string(), source.to_string());
    let tokens = lexer.make_tokens().expect("should lex");
    let mut parser = Parser::new(tokens);
    let result = parser.parse();
    assert!(result.error.is_none(), "should parse: {:?}", result.error);
    result.node.expect("should produce a node")
}

#[test]
fn a_file_with_main_is_a_program() {
    let ast = parse("method main() -> int {\n    release 0\n}\n");
    assert_eq!(shape_of(&ast), ProgramShape::Program);
}

#[test]
fn a_file_without_main_is_a_script() {
    let ast = parse("echo(\"hello\")\n");
    assert_eq!(shape_of(&ast), ProgramShape::Script);
}

#[test]
fn main_must_be_at_the_top_level() {
    // A nested `main` is an ordinary method that happens to share the name.
    let ast = parse(
        "method outer() -> int {\n\
        \x20   method main() -> int => 0\n\
        \x20   release 0\n\
        }\n",
    );
    assert_eq!(shape_of(&ast), ProgramShape::Script);
}

#[test]
fn a_main_binding_is_not_a_main_method() {
    let ast = parse("let main: int = 3\n");
    assert_eq!(shape_of(&ast), ProgramShape::Script);
}
