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

#[test]
fn a_program_reports_an_undefined_name_statically() {
    let (errors, _, _) = xenith::check_source_typed(
        "<test>",
        "method main() -> int {\n    echo(\"{nowhere}\")\n    release 0\n}\n",
    )
    .expect("should parse");

    assert!(
        errors.iter().any(|e| e.as_string().contains("XEN002")),
        "expected a static XEN002, got {errors:?}"
    );
}

#[test]
fn a_script_stays_quiet_about_an_undefined_name() {
    // A script's method may legitimately read a name declared later, so the
    // checker cannot prove anything. The interpreter still reports it.
    let (errors, _, _) = xenith::check_source_typed(
        "<test>",
        "method report() -> null {\n    echo(\"later is {later}\")\n    release null\n}\n\nlet later: int = 7\nreport()\n",
    )
    .expect("should parse");

    assert!(errors.is_empty(), "expected silence, got {errors:?}");
}

#[test]
fn try_catches_panic_and_exposes_error_fields() {
    let source = "method main() -> int {\n\
        try {\n\
            panic(\"boom\")\n\
        } catch (error) {\n\
            when error.code == \"XEN300\" {\n\
                release 0\n\
            } otherwise {\n\
                release 1\n\
            }\n\
        }\n\
    }\n";
    assert!(matches!(xenith::run("<test>", source).unwrap(), xenith::values::Value::Int(0)));
}

#[test]
fn try_returns_without_running_catch() {
    let source = "method main() -> int {\n\
        try { release 42 } catch (error) { release 1 }\n\
    }\n";
    assert!(matches!(xenith::run("<test>", source).unwrap(), xenith::values::Value::Int(42)));
}

#[test]
fn a_failure_inside_catch_is_not_swallowed() {
    let source = "method main() -> int {\n\
        try { panic(\"outer\") } catch (error) { panic(\"inner\") }\n\
    }\n";
    let error = xenith::run("<test>", source).expect_err("catch failure should escape");
    assert_eq!(error.code, "XEN300");
    assert_eq!(error.details, "inner");
}

#[test]
fn try_catches_division_by_zero() {
    let source = "method main() -> int {\n\
        let denominator: int = 0\n\
        try {\n\
            let value: int = 10 / denominator\n\
            release value\n\
        } catch (error) {\n\
            when error.code == \"XEN003\" { release 7 } otherwise { release 1 }\n\
        }\n\
    }\n";
    let value = xenith::run("<test>", source).unwrap();
    assert!(matches!(value, xenith::values::Value::Int(7)), "got {value:?}");
}
