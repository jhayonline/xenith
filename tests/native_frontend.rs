use std::path::Path;
use std::process::Command;
use xenith::native::{
    check::check,
    syntax::{parse, Span},
};

fn assert_error(source: &str, fragment: &str) {
    let program = parse(Path::new("src/main.xen"), source).expect("source should parse");
    let errors = check(&program).expect_err("source should not type-check");
    assert!(
        errors.iter().any(|error| error.message.contains(fragment)),
        "{errors:?}"
    );
}

fn check_source(
    source: &str,
) -> Result<xenith::native::check::TypedProgram, Vec<xenith::native::syntax::Diagnostic>> {
    let program = parse(Path::new("src/main.xen"), source).expect("source should parse");
    check(&program)
}

struct BuiltSource {
    _directory: tempfile::TempDir,
    executable: std::path::PathBuf,
}

fn build_source(source: &str) -> BuiltSource {
    let program = parse(Path::new("src/main.xen"), source).unwrap();
    let typed = check(&program).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    xenith::native::codegen::emit_object(&typed, &object).unwrap();
    xenith::native::codegen::link_host(&object, &executable).unwrap();
    BuiltSource {
        _directory: directory,
        executable,
    }
}

#[test]
fn accepts_typed_main_with_integer_expression() {
    let source = "fn main() -> i32 { let value: i32 = 40 + 2 return value }";
    let program = parse(Path::new("src/main.xen"), source).unwrap();
    assert!(check(&program).is_ok());
}

#[test]
fn accepts_function_returning_directly_from_infinite_loop() {
    assert!(check_source("fn main() -> i32 { while true { return 42 } }").is_ok());
}

#[test]
fn native_program_returns_directly_from_infinite_loop() {
    let artifact = build_source("fn main() -> i32 { while true { return 42 } }");
    assert_eq!(
        Command::new(artifact.executable).status().unwrap().code(),
        Some(42)
    );
}

#[test]
fn native_program_returns_from_if_inside_infinite_loop() {
    let artifact =
        build_source("fn main() -> i32 { while true { if true { return 42 } else { return 0 } } }");
    assert_eq!(
        Command::new(artifact.executable).status().unwrap().code(),
        Some(42)
    );
}

#[test]
fn native_program_returns_directly_from_if() {
    let artifact = build_source("fn main() -> i32 { if true { return 42 } else { return 0 } }");
    assert_eq!(
        Command::new(artifact.executable).status().unwrap().code(),
        Some(42)
    );
}

#[test]
fn parser_diagnostic_points_to_the_unexpected_token() {
    let source = "fn main() -> i32 { return ) }";
    let errors = parse(Path::new("main.xen"), source).expect_err("source should not parse");

    assert_eq!(errors[0].message, "expected expression");
    assert_eq!(errors[0].span, Some(Span::new(26, 27)));
}

#[test]
fn parser_recovers_to_report_multiple_bad_statements() {
    let errors = parse(
        Path::new("main.xen"),
        "fn main() -> i32 { let = 1 return ) }",
    )
    .expect_err("source should not parse");

    assert_eq!(errors.len(), 2);
}

#[test]
fn parser_recovery_terminates_at_end_of_unterminated_block() {
    let errors = parse(Path::new("main.xen"), "fn main() -> i32 { let = 1")
        .expect_err("source should not parse");

    assert_eq!(errors.len(), 2);
}
#[test]
fn parser_recovers_from_bad_struct_field_before_next_function() {
    let errors = parse(
        Path::new("main.xen"),
        "struct Pair { left i32 } fn main() -> i32 { return ) }",
    )
    .expect_err("source should not parse");

    assert_eq!(errors.len(), 2);
}

#[test]
fn parser_propagates_bad_statement_inside_if_block() {
    let errors = parse(
        Path::new("main.xen"),
        "fn main() -> i32 { if true { let = 1 let = 2 } else { return 0 } }",
    )
    .expect_err("source should not parse");

    assert_eq!(errors.len(), 1);
}

#[test]
fn parser_reports_truncated_let_statement_without_panicking() {
    let errors = parse(Path::new("main.xen"), "fn main() -> i32 { let")
        .expect_err("source should not parse");

    assert!(errors
        .iter()
        .any(|error| error.message.contains("binding name")));
}

#[test]
fn native_program_respects_arithmetic_precedence() {
    let artifact = build_source("fn main() -> i32 { return 2 + 20 * 2 }");
    assert_eq!(
        Command::new(artifact.executable).status().unwrap().code(),
        Some(42)
    );
}

#[test]
fn native_program_preserves_i64_arithmetic_type_through_if() {
    let artifact = build_source(
        "fn main() -> i32 { let mut value = 40i64 + 1i64 if true { value = value + 1i64 } else { value = 0i64 } if value == 42i64 { return 42 } else { return 0 } }",
    );
    assert_eq!(
        Command::new(artifact.executable).status().unwrap().code(),
        Some(42)
    );
}

#[test]
fn parses_struct_literal_and_field_access() {
    let program = parse(
        Path::new("main.xen"),
        "struct Point { x: i32, y: i32 } fn main() -> i32 { let point = Point { x: 20, y: 22 } return point.x }",
    )
    .unwrap();
    assert_eq!(program.structs[0].fields[1].name, "y");
}

#[test]
fn parses_if_statement() {
    let program = parse(
        Path::new("main.xen"),
        "fn choose(value: i32) -> i32 { if value < 0 { return 1 } else { return 2 } } fn main() -> i32 { return choose(0) }",
    )
    .unwrap();
    assert_eq!(program.functions[0].body.len(), 1);
}

#[test]
fn checks_if_conditions_and_comparisons() {
    assert!(check_source("fn choose(value: i32) -> i32 { if value < 0 { return 1 } else { return 2 } } fn main() -> i32 { return choose(0) }").is_ok());
    assert_error(
        "fn main() -> i32 { if 1 { return 1 } else { return 2 } }",
        "bool",
    );
    assert_error(
        "fn main() -> i32 { if 1 < 2i64 { return 1 } else { return 2 } }",
        "same type",
    );
    assert_error(
        "fn main() -> i32 { if 1 && true { return 1 } else { return 2 } }",
        "`&&` requires bool",
    );
    assert_error(
        "fn main() -> i32 { let value = 0 if true { value = 1 } else { value = 2 } return value }",
        "immutable binding",
    );
    assert_error(
        "fn main() -> i32 { let value = 0 if true { if true { value = 1 } else { return 0 } } else { return 0 } return value }",
        "immutable binding",
    );
    assert_error(
        "fn main() -> i32 { let mut value = 0 while value < 1 { if true { missing = 1 } else { value = 1 } } return value }",
        "unknown name",
    );
    assert_error(
        "fn main() -> i32 { let mut value = 0 if true { if true { value = false } else { value = 1 } } else { value = 2 } return value }",
        "type mismatch",
    );
    assert_error(
        "fn main() -> i32 { if true { let temporary = 42 } else { return 0 } return temporary }",
        "unknown name",
    );
    assert_error(
        "fn main() -> i32 { if true { return 42 } else { let value = 0 } }",
        "main must return",
    );
    assert_error(
        "struct Pair { first: i32, second: i32 } fn main() -> i32 { let left = Pair { first: 1, second: 2 } let right = Pair { first: 1, second: 3 } if (left < right) { return 1 } else { return 0 } }",
        "ordering comparisons require integer",
    );
    assert_error(
        "fn main() -> i32 { if true < false { return 1 } else { return 0 } }",
        "ordering comparisons require integer",
    );
    assert_error(
        "fn main() -> i32 { let value = true + false return 0 }",
        "arithmetic operands require integers",
    );
}

#[test]
fn accepts_extended_comparison_operators() {
    assert!(
        check_source("fn main() -> i32 { if 42 >= 42 { return 42 } else { return 0 } }").is_ok()
    );
    assert!(
        check_source("fn main() -> i32 { if 42 != 0 { return 42 } else { return 0 } }").is_ok()
    );
}

#[test]
fn checks_mutable_while_counter() {
    assert!(check_source("fn main() -> i32 { let mut value = 0 while value < 42 { value = value + 1 } return value }").is_ok());
}

#[test]
fn accepts_struct_fields_and_rejects_invalid_construction() {
    assert!(check_source("struct Point { x: i32, y: i32 } fn main() -> i32 { let point = Point { x: 40, y: 2 } return point.x + point.y }").is_ok());
    assert_error(
        "struct Point { x: i32, y: i32 } fn main() -> i32 { return Point { x: 1 }.x }",
        "missing field",
    );
    assert_error(
        "struct Point { x: i32 } fn main() -> i32 { return Point { x: 1 }.y }",
        "unknown field",
    );
    assert_error(
        "struct Point { x: i32, y: i32 } fn main() -> i32 { let point = Point { x: 1, x: 2, y: 3 } return point.x }",
        "duplicate field `x`",
    );
    assert_error(
        "struct Outer { inner: Missing } fn main() -> i32 { return 0 }",
        "unknown struct `Missing`",
    );
    assert_error(
        "struct A { b: B } struct B { a: A } fn main() -> i32 { return 0 }",
        "recursive struct layout",
    );
}

#[test]
fn parses_typed_function_and_call() {
    let program = parse(
        Path::new("main.xen"),
        "fn add(a: i32, b: i32) -> i32 { return a + b } fn main() -> i32 { return add(20, 22) }",
    )
    .unwrap();

    assert_eq!(program.functions[0].params.len(), 2);
}

#[test]
fn rejects_out_of_range_unsuffixed_integer_literal() {
    let errors = parse(
        Path::new("main.xen"),
        "fn main() -> i32 { return 2147483648 }",
    )
    .expect_err("source should not parse");

    assert!(errors[0].message.contains("out of range"));
}

#[test]
fn rejects_out_of_range_i64_literal() {
    let errors = parse(
        Path::new("main.xen"),
        "fn main() -> i32 { let value = 9223372036854775808i64 return 0 }",
    )
    .expect_err("source should not parse");

    assert!(errors[0]
        .message
        .contains("i64 integer literal out of range"));
}

#[test]
fn rejects_malformed_numeric_literal_suffix() {
    let errors = parse(Path::new("main.xen"), "fn main() -> i32 { return 42i65 }")
        .expect_err("source should not parse");

    assert!(errors[0].message.contains("invalid numeric literal"));
}

#[test]
fn rejects_invalid_declaration_identifier() {
    let errors = parse(Path::new("main.xen"), "fn 1main() -> i32 { return 0 }")
        .expect_err("source should not parse");

    assert!(errors[0].message.contains("invalid function name"));
}

#[test]
fn accepts_forward_calls_and_rejects_bad_calls() {
    assert!(check_source(
        "fn main() -> i32 { return add(1, 2) } fn add(a: i32, b: i32) -> i32 { return a + b }",
    )
    .is_ok());
    assert_error("fn main() -> i32 { return missing(1) }", "unknown function");
    assert_error(
        "fn add(a: i32) -> i32 { return a } fn main() -> i32 { return add(1, 2) }",
        "arguments",
    );
    assert_error(
        "fn add(a: i32) -> i32 { return a } fn main() -> i32 { return add(1i64) }",
        "argument type",
    );
}

#[test]
fn accepts_recursive_calls() {
    assert!(check_source(
        "fn recurse(value: i32) -> i32 { return recurse(value) } fn main() -> i32 { return 0 }",
    )
    .is_ok());
}

#[test]
fn rejects_duplicate_parameter_names() {
    assert_error(
        "fn pick(x: i32, x: i32) -> i32 { return x } fn main() -> i32 { return pick(1, 2) }",
        "duplicate parameter",
    );
    assert_error(
        "fn pick(x: i32, x: i64) -> i32 { return 0 } fn main() -> i32 { return pick(1, 2i64) }",
        "duplicate parameter",
    );
}

#[test]
fn rejects_missing_or_invalid_main() {
    assert_error("fn nope() -> i32 { return 0 }", "main");
    assert_error("fn main() -> i64 { return 0 }", "-> i32");
    assert_error(
        "fn main(value: i32) -> i32 { return value }",
        "takes no parameters",
    );
    assert_error(
        "fn helper(value: Missing) -> i32 { return 0 } fn main() -> i32 { return 0 }",
        "unknown type `Missing`",
    );
    assert_error(
        "fn main() -> i32 { let value: Missing = 0 return 0 }",
        "unknown type `Missing`",
    );
}

#[test]
fn rejects_reassignment_without_mut_and_mixed_numbers() {
    assert_error("fn main() -> i32 { let x = 1 x = 2 return x }", "immutable");
    assert_error("fn main() -> i32 { return 1 + 2i64 }", "same type");
    assert_error("fn main() -> i32 { return 1 / 0 }", "division by zero");
    assert_error(
        "fn main() -> i32 { return 2147483647 + 1 }",
        "integer overflow",
    );
    assert_error(
        "fn main() -> i32 { if false && missing { return 0 } else { return 0 } }",
        "unknown name `missing`",
    );
    assert_error(
        "fn main() -> i32 { let value = 1 let value = 2 return value }",
        "duplicate binding `value`",
    );
    assert_error(
        "fn main() -> i32 { if true { let value = 1 let value = 2 return value } else { return 0 } }",
        "duplicate binding `value`",
    );
}

#[test]
fn generated_native_program_returns_main_exit_code() {
    let source = "fn main() -> i32 { return 40 + 2 }";
    let program = parse(Path::new("src/main.xen"), source).unwrap();
    let typed = check(&program).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let object = directory.path().join("main.o");
    let executable = directory.path().join("main");
    xenith::native::codegen::emit_object(&typed, &object).unwrap();
    xenith::native::codegen::link_host(&object, &executable).unwrap();
    let status = Command::new(executable).status().unwrap();
    assert_eq!(status.code(), Some(42));
}

#[test]
fn native_program_accepts_hexadecimal_and_binary_literals() {
    let artifact = build_source("fn main() -> i32 { return 0x20 + 0b1010 }");
    assert_eq!(
        Command::new(artifact.executable).status().unwrap().code(),
        Some(42)
    );
}

#[test]
fn native_program_traps_on_runtime_integer_overflow() {
    let artifact = build_source(
        "fn main() -> i32 { let maximum = 2147483647 let one = 1 return maximum + one }",
    );
    assert_ne!(
        Command::new(artifact.executable).status().unwrap().code(),
        Some(0)
    );
}

#[test]
fn native_program_ignores_line_comments() {
    let artifact = build_source("# entry point\nfn main() -> i32 {\n  return 42 # success\n}");
    assert_eq!(
        Command::new(artifact.executable).status().unwrap().code(),
        Some(42)
    );
}

#[test]
fn native_program_calls_typed_function() {
    let artifact = build_source(
        "fn twice(value: i32) -> i32 { return value * 2 } fn main() -> i32 { return twice(21) }",
    );
    assert_eq!(
        Command::new(artifact.executable).status().unwrap().code(),
        Some(42)
    );
}

#[test]
fn native_calls_names_ending_in_numeric_type_suffixes() {
    let artifact = build_source(
        "fn answer_i32() -> i32 { return 42 } fn main() -> i32 { let answer_i32 = 0 return answer_i32() }",
    );
    assert_eq!(
        Command::new(artifact.executable).status().unwrap().code(),
        Some(42)
    );
}

#[test]
fn native_program_calls_forward_function() {
    let artifact = build_source(
        "fn main() -> i32 { return twice(21) } fn twice(value: i32) -> i32 { return value * 2 }",
    );
    assert_eq!(
        Command::new(artifact.executable).status().unwrap().code(),
        Some(42)
    );
}

#[test]
fn native_program_passes_returns_and_reads_structs() {
    let artifact = build_source("struct Pair { first: i32, second: i32 } fn swap(pair: Pair) -> Pair { return Pair { first: pair.second, second: pair.first } } fn main() -> i32 { let pair = swap(Pair { first: 2, second: 40 }) return pair.second + pair.first }");
    assert_eq!(
        Command::new(artifact.executable).status().unwrap().code(),
        Some(42)
    );
}

#[test]
fn native_program_reads_mixed_scalar_struct_fields() {
    let artifact = build_source(
        "struct Status { enabled: bool, limit: i64 } fn main() -> i32 { let status = Status { enabled: true, limit: 42i64 } if status.enabled && status.limit == 42i64 { return 42 } else { return 0 } }",
    );
    assert_eq!(
        Command::new(artifact.executable).status().unwrap().code(),
        Some(42)
    );
}

#[test]
fn native_program_reads_three_field_structs() {
    let artifact = build_source(
        "struct Triple { first: i32, second: i32, third: i32 } fn main() -> i32 { let triple = Triple { first: 10, second: 20, third: 12 } return triple.first + triple.second + triple.third }",
    );
    assert_eq!(
        Command::new(artifact.executable).status().unwrap().code(),
        Some(42)
    );
}

#[test]
fn native_program_reads_nested_struct_fields() {
    let artifact = build_source(
        "struct Position { x: i32, y: i32 } struct Player { position: Position, active: bool } fn main() -> i32 { let player = Player { position: Position { x: 20, y: 22 }, active: true } if player.active { return player.position.x + player.position.y } else { return 0 } }",
    );
    assert_eq!(
        Command::new(artifact.executable).status().unwrap().code(),
        Some(42)
    );
}

#[test]
fn native_program_passes_nested_structs_through_calls() {
    let artifact = build_source(
        "struct Position { x: i32, y: i32 } struct Player { position: Position, active: bool } fn keep(player: Player) -> Player { return player } fn main() -> i32 { let player = keep(Player { position: Position { x: 20, y: 22 }, active: true }) if player.active { return player.position.x + player.position.y } else { return 0 } }",
    );
    assert_eq!(
        Command::new(artifact.executable).status().unwrap().code(),
        Some(42)
    );
}

#[test]
fn native_program_returns_wide_struct_from_if() {
    let artifact = build_source(
        "struct Position { x: i32, y: i32 } struct Player { position: Position, active: bool } fn choose(flag: bool) -> Player { if flag { return Player { position: Position { x: 20, y: 22 }, active: true } } else { return Player { position: Position { x: 0, y: 0 }, active: false } } } fn main() -> i32 { let player = choose(true) if player.active { return player.position.x + player.position.y } else { return 0 } }",
    );
    assert_eq!(
        Command::new(artifact.executable).status().unwrap().code(),
        Some(42)
    );
}

#[test]
fn native_program_returns_wide_struct_from_while() {
    let artifact = build_source(
        "struct Position { x: i32, y: i32 } struct Player { position: Position, active: bool } fn choose() -> Player { while true { return Player { position: Position { x: 20, y: 22 }, active: true } } return Player { position: Position { x: 0, y: 0 }, active: false } } fn main() -> i32 { let player = choose() if player.active { return player.position.x + player.position.y } else { return 0 } }",
    );
    assert_eq!(
        Command::new(artifact.executable).status().unwrap().code(),
        Some(42)
    );
}

#[test]
fn native_program_branches_on_comparison() {
    let true_path = build_source("fn main() -> i32 { if 1 < 2 { return 42 } else { return 7 } }");
    let false_path = build_source("fn main() -> i32 { if 2 < 1 { return 7 } else { return 42 } }");
    assert_eq!(
        Command::new(true_path.executable).status().unwrap().code(),
        Some(42)
    );
    assert_eq!(
        Command::new(false_path.executable).status().unwrap().code(),
        Some(42)
    );
}

#[test]
fn native_program_uses_struct_literal_in_if_condition() {
    let artifact = build_source(
        "struct Pair { first: i32, second: i32 } fn main() -> i32 { if Pair { first: 42, second: 0 }.first == 42 { return 42 } else { return 0 } }",
    );
    assert_eq!(
        Command::new(artifact.executable).status().unwrap().code(),
        Some(42)
    );
}

#[test]
fn native_program_compares_every_struct_field_for_equality() {
    let artifact = build_source(
        "struct Pair { first: i32, second: i32 } fn main() -> i32 { let left = Pair { first: 20, second: 22 } let right = Pair { first: 20, second: 0 } if left == right { return 0 } else { return 42 } }",
    );
    assert_eq!(
        Command::new(artifact.executable).status().unwrap().code(),
        Some(42)
    );
}

#[test]
fn native_program_compares_every_struct_field_for_inequality() {
    let artifact = build_source(
        "struct Pair { first: i32, second: i32 } fn main() -> i32 { let left = Pair { first: 20, second: 22 } let right = Pair { first: 20, second: 0 } if left != right { return 42 } else { return 0 } }",
    );
    assert_eq!(
        Command::new(artifact.executable).status().unwrap().code(),
        Some(42)
    );
}

#[test]
fn native_program_branches_on_extended_comparison() {
    let artifact = build_source("fn main() -> i32 { if 42 >= 42 { return 42 } else { return 0 } }");
    assert_eq!(
        Command::new(artifact.executable).status().unwrap().code(),
        Some(42)
    );
}

#[test]
fn native_program_branches_on_boolean_literal() {
    let artifact = build_source("fn main() -> i32 { if true { return 42 } else { return 0 } }");
    assert_eq!(
        Command::new(artifact.executable).status().unwrap().code(),
        Some(42)
    );
}

#[test]
fn native_program_branches_on_boolean_parameter() {
    let artifact = build_source("fn choose(flag: bool) -> i32 { if flag { return 42 } else { return 0 } } fn main() -> i32 { return choose(true) }");
    assert_eq!(
        Command::new(artifact.executable).status().unwrap().code(),
        Some(42)
    );
}

#[test]
fn native_program_evaluates_boolean_operators() {
    let artifact = build_source(
        "fn main() -> i32 { if !false && true || false { return 42 } else { return 0 } }",
    );
    assert_eq!(
        Command::new(artifact.executable).status().unwrap().code(),
        Some(42)
    );
}

#[test]
fn native_program_evaluates_parenthesized_conditions() {
    let artifact =
        build_source("fn main() -> i32 { if !(false || (1 < 0)) { return 42 } else { return 0 } }");
    assert_eq!(
        Command::new(artifact.executable).status().unwrap().code(),
        Some(42)
    );
}

#[test]
fn native_boolean_operators_short_circuit() {
    let and_artifact =
        build_source("fn main() -> i32 { if false && 1 / 0 < 1 { return 0 } else { return 42 } }");
    let or_artifact =
        build_source("fn main() -> i32 { if true || 1 / 0 < 1 { return 42 } else { return 0 } }");
    assert_eq!(
        Command::new(and_artifact.executable)
            .status()
            .unwrap()
            .code(),
        Some(42)
    );
    assert_eq!(
        Command::new(or_artifact.executable)
            .status()
            .unwrap()
            .code(),
        Some(42)
    );
}

#[test]
fn native_program_preserves_assignments_after_if() {
    let true_artifact = build_source(
        "fn main() -> i32 { let mut value = 0 if true { value = 42 } else { value = 7 } return value }",
    );
    let false_artifact = build_source(
        "fn main() -> i32 { let mut value = 0 if false { value = 7 } else { value = 42 } return value }",
    );
    assert_eq!(
        Command::new(true_artifact.executable)
            .status()
            .unwrap()
            .code(),
        Some(42)
    );
    assert_eq!(
        Command::new(false_artifact.executable)
            .status()
            .unwrap()
            .code(),
        Some(42)
    );
}

#[test]
fn native_program_keeps_outer_binding_when_nested_binding_is_shadowed() {
    let artifact = build_source(
        "fn main() -> i32 { let mut value = 0 if true { let mut value = 40 value = value + 2 } else { } return value }",
    );
    assert_eq!(
        Command::new(artifact.executable).status().unwrap().code(),
        Some(0)
    );
}

#[test]
fn native_program_preserves_struct_assignments_after_if() {
    let artifact = build_source(
        "struct Pair { first: i32, second: i32 } fn main() -> i32 { let mut pair = Pair { first: 0, second: 0 } if true { pair = Pair { first: 20, second: 22 } } else { pair = Pair { first: 1, second: 1 } } return pair.first + pair.second }",
    );
    assert_eq!(
        Command::new(artifact.executable).status().unwrap().code(),
        Some(42)
    );
}

#[test]
fn native_program_preserves_struct_assignments_after_while() {
    let artifact = build_source(
        "struct Pair { first: i32, second: i32 } fn main() -> i32 { let mut pair = Pair { first: 0, second: 0 } let mut count = 0 while count < 1 { pair = Pair { first: 20, second: 22 } count = count + 1 } return pair.first + pair.second }",
    );
    assert_eq!(
        Command::new(artifact.executable).status().unwrap().code(),
        Some(42)
    );
}

#[test]
fn native_program_preserves_struct_assignments_from_while_inside_if() {
    let artifact = build_source(
        "struct Pair { first: i32, second: i32 } fn main() -> i32 { let mut pair = Pair { first: 0, second: 0 } if true { let mut count = 0 while count < 1 { pair = Pair { first: 20, second: 22 } count = count + 1 } } else { } return pair.first + pair.second }",
    );
    assert_eq!(
        Command::new(artifact.executable).status().unwrap().code(),
        Some(42)
    );
}

#[test]
fn native_program_preserves_struct_assignments_from_while_inside_while() {
    let artifact = build_source(
        "struct Pair { first: i32, second: i32 } fn main() -> i32 { let mut pair = Pair { first: 0, second: 0 } let mut outer = 0 while outer < 1 { let mut inner = 0 while inner < 1 { pair = Pair { first: 20, second: 22 } inner = inner + 1 } outer = outer + 1 } return pair.first + pair.second }",
    );
    assert_eq!(
        Command::new(artifact.executable).status().unwrap().code(),
        Some(42)
    );
}

#[test]
fn native_program_preserves_nested_if_assignments() {
    let artifact = build_source(
        "fn main() -> i32 { let mut value = 0 if true { if true { value = 42 } else { value = 1 } } else { value = 2 } return value }",
    );
    assert_eq!(
        Command::new(artifact.executable).status().unwrap().code(),
        Some(42)
    );
}

#[test]
fn native_program_preserves_assignments_from_if_inside_while() {
    let artifact = build_source(
        "fn main() -> i32 { let mut value = 0 let mut count = 0 while count < 6 { if count < 6 { value = value + 7 } else { value = 0 } count = count + 1 } return value }",
    );
    assert_eq!(
        Command::new(artifact.executable).status().unwrap().code(),
        Some(42)
    );
}

#[test]
fn native_program_runs_while_inside_if() {
    let artifact = build_source(
        "fn main() -> i32 { let mut value = 0 if true { let mut count = 0 while count < 6 { value = value + 7 count = count + 1 } } else { value = 0 } return value }",
    );
    assert_eq!(
        Command::new(artifact.executable).status().unwrap().code(),
        Some(42)
    );
}

#[test]
fn native_program_returns_from_if_inside_while() {
    let artifact = build_source(
        "fn main() -> i32 { let mut count = 0 while count < 6 { if count == 3 { return 42 } else { count = count + 1 } } return 0 }",
    );
    assert_eq!(
        Command::new(artifact.executable).status().unwrap().code(),
        Some(42)
    );
}

#[test]
fn native_program_returns_directly_from_while() {
    let artifact = build_source("fn main() -> i32 { while true { return 42 } return 0 }");
    assert_eq!(
        Command::new(artifact.executable).status().unwrap().code(),
        Some(42)
    );
}

#[test]
fn native_program_ignores_statements_after_top_level_return() {
    let artifact = build_source("fn main() -> i32 { return 42 let unused = 0 return unused }");
    assert_eq!(
        Command::new(artifact.executable).status().unwrap().code(),
        Some(42)
    );
}

#[test]
fn native_program_runs_while_loop() {
    let artifact = build_source("fn main() -> i32 { let mut value = 0 while value < 42 { value = value + 1 } return value }");
    assert_eq!(
        Command::new(artifact.executable).status().unwrap().code(),
        Some(42)
    );
}

#[test]
fn native_while_loop_runs_multiple_assignments() {
    let artifact = build_source("fn main() -> i32 { let mut value = 0 let mut total = 0 while value < 6 { total = total + 7 value = value + 1 } return total }");
    assert_eq!(
        Command::new(artifact.executable).status().unwrap().code(),
        Some(42)
    );
}
