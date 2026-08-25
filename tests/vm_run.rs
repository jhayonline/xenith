//! End-to-end: source in, value out, through the compiler and the VM.

use xenith::lexer::Lexer;
use xenith::parser::Parser;
use xenith::values::Value;

/// Runs through the VM, or fails the test saying why it could not.
fn vm(source: &str) -> Value {
    let mut lexer = Lexer::new("<test>".to_string(), source.to_string());
    let tokens = lexer.make_tokens().expect("should lex");
    let mut parser = Parser::new(tokens);
    let parsed = parser.parse();
    assert!(parsed.error.is_none(), "should parse: {:?}", parsed.error);
    let ast = parsed.node.expect("should produce a node");

    match xenith::vm::compile_and_run(&ast) {
        Some(Ok(value)) => value,
        Some(Err(error)) => panic!("should not have failed: {}", error.as_string()),
        None => panic!("should have compiled"),
    }
}

#[test]
fn an_int_literal_evaluates_to_itself() {
    assert!(matches!(vm("42\n"), Value::Int(42)));
}

#[test]
fn a_float_literal_evaluates_to_itself() {
    let Value::Float(f) = vm("1.5\n") else {
        panic!("expected a float");
    };
    assert_eq!(f, 1.5);
}

#[test]
fn a_bool_literal_evaluates_to_itself() {
    assert!(matches!(vm("true\n"), Value::Bool(true)));
}

#[test]
fn the_last_statement_is_the_value() {
    assert!(matches!(vm("1\n2\n3\n"), Value::Int(3)));
}

#[test]
fn arithmetic_agrees_with_the_language() {
    assert!(matches!(vm("1 + 2\n"), Value::Int(3)));
    assert!(matches!(vm("7 - 2\n"), Value::Int(5)));
    assert!(matches!(vm("6 * 7\n"), Value::Int(42)));
    assert!(matches!(vm("7 / 2\n"), Value::Int(3)));
    assert!(matches!(vm("7 % 2\n"), Value::Int(1)));
    assert!(matches!(vm("2 ^ 10\n"), Value::Int(1024)));
    assert!(matches!(vm("-5\n"), Value::Int(-5)));
}

#[test]
fn comparisons_produce_bools() {
    assert!(matches!(vm("1 < 2\n"), Value::Bool(true)));
    assert!(matches!(vm("1 > 2\n"), Value::Bool(false)));
    assert!(matches!(vm("2 <= 2\n"), Value::Bool(true)));
    assert!(matches!(vm("2 >= 3\n"), Value::Bool(false)));
    assert!(matches!(vm("1 == 1\n"), Value::Bool(true)));
    assert!(matches!(vm("1 != 1\n"), Value::Bool(false)));
}

#[test]
fn precedence_is_the_parsers_business_and_survives_compilation() {
    assert!(matches!(vm("1 + 2 * 3\n"), Value::Int(7)));
    assert!(matches!(vm("(1 + 2) * 3\n"), Value::Int(9)));
}

#[test]
fn a_string_concatenation_works_because_value_add_does_it() {
    let Value::String(s) = vm("\"a\" + \"b\"\n") else {
        panic!("expected a string");
    };
    assert_eq!(s.value, "ab");
}
