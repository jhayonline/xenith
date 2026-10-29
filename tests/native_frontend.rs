use std::path::Path;
use xenith::native::{check::check, syntax::parse};

fn assert_error(source: &str, fragment: &str) {
    let program = parse(Path::new("src/main.xen"), source).expect("source should parse");
    let errors = check(&program).expect_err("source should not type-check");
    assert!(
        errors.iter().any(|error| error.message.contains(fragment)),
        "{errors:?}"
    );
}

#[test]
fn accepts_typed_main_with_integer_expression() {
    let source = "fn main() -> i32 { let value: i32 = 40 + 2 return value }";
    let program = parse(Path::new("src/main.xen"), source).unwrap();
    assert!(check(&program).is_ok());
}

#[test]
fn rejects_missing_or_invalid_main() {
    assert_error("fn nope() -> i32 { return 0 }", "main");
    assert_error("fn main() -> i64 { return 0 }", "-> i32");
    assert_error(
        "fn main(value: i32) -> i32 { return value }",
        "takes no parameters",
    );
}

#[test]
fn rejects_reassignment_without_mut_and_mixed_numbers() {
    assert_error("fn main() -> i32 { let x = 1 x = 2 return x }", "immutable");
    assert_error("fn main() -> i32 { return 1 + 2i64 }", "same type");
}
