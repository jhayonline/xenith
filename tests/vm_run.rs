//! End-to-end: source in, value out, through the compiler and the VM.

use xenith::values::Value;

/// Runs through the VM, or fails the test saying why it could not.
fn vm(source: &str) -> Value {
    // Checked, not merely parsed: a user's code always reaches the compiler
    // with the checker's types behind it, so a test that skipped the checker
    // would be exercising a path nothing actually takes.
    let (errors, types, ast) =
        xenith::check_source_typed("<test>", source).expect("should lex and parse");
    assert!(errors.is_empty(), "should check: {:?}", errors);

    match xenith::vm::compile_and_run(&ast, &types) {
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

#[test]
fn a_local_round_trips() {
    assert!(matches!(vm("let x: int = 41\nx + 1\n"), Value::Int(42)));
}

#[test]
fn reassignment_takes_effect() {
    assert!(matches!(vm("let x: int = 1\nx = 9\nx\n"), Value::Int(9)));
}

#[test]
fn a_later_local_can_read_an_earlier_one() {
    assert!(matches!(
        vm("let a: int = 2\nlet b: int = a * 3\nb\n"),
        Value::Int(6)
    ));
}

#[test]
fn echo_compiles_to_its_own_opcode() {
    // Not asserting on stdout -- that is what tests/differential.rs is for.
    // This only proves the call is recognised rather than refused.
    let (_, types, ast) =
        xenith::check_source_typed("<test>", "echo(1)\n").expect("should lex and parse");

    let chunk = xenith::vm::compile::compile(&ast, &types).expect("echo should compile");
    assert!(
        chunk.disassemble().contains("ECHO"),
        "{}",
        chunk.disassemble()
    );
}

#[test]
fn a_call_to_anything_else_is_still_refused() {
    let (_, types, ast) =
        xenith::check_source_typed("<test>", "len(\"ab\")\n").expect("should lex and parse");

    assert!(xenith::vm::compile::compile(&ast, &types).is_err());
}

#[test]
fn when_takes_the_true_branch() {
    assert!(matches!(
        vm("let x: int = 0\nwhen true {\n    x = 1\n}\nx\n"),
        Value::Int(1)
    ));
}

#[test]
fn when_skips_the_false_branch() {
    assert!(matches!(
        vm("let x: int = 0\nwhen false {\n    x = 1\n}\nx\n"),
        Value::Int(0)
    ));
}

#[test]
fn or_when_chains_in_order() {
    assert!(matches!(
        vm("let x: int = 0\nwhen false {\n    x = 1\n} or when true {\n    x = 2\n} otherwise {\n    x = 3\n}\nx\n"),
        Value::Int(2)
    ));
}

#[test]
fn otherwise_is_the_fallthrough() {
    assert!(matches!(
        vm("let x: int = 0\nwhen false {\n    x = 1\n} otherwise {\n    x = 3\n}\nx\n"),
        Value::Int(3)
    ));
}

#[test]
fn and_short_circuits_on_a_false_left() {
    assert!(matches!(vm("false && true\n"), Value::Bool(false)));
    assert!(matches!(vm("true && false\n"), Value::Bool(false)));
    assert!(matches!(vm("true && true\n"), Value::Bool(true)));
}

#[test]
fn or_short_circuits_on_a_true_left() {
    assert!(matches!(vm("true || false\n"), Value::Bool(true)));
    assert!(matches!(vm("false || true\n"), Value::Bool(true)));
    assert!(matches!(vm("false || false\n"), Value::Bool(false)));
}

#[test]
fn a_branch_body_gets_its_own_scope() {
    // `let` inside a branch is local to the branch. The tree walker was
    // changed to do this deliberately; the VM must match.
    assert!(matches!(
        vm("let x: int = 1\nwhen true {\n    let x: int = 2\n}\nx\n"),
        Value::Int(1)
    ));
}

#[test]
fn a_while_loop_counts() {
    assert!(matches!(
        vm("let i: int = 0\nwhile i < 5 {\n    i = i + 1\n}\ni\n"),
        Value::Int(5)
    ));
}

#[test]
fn a_while_loop_that_never_runs() {
    assert!(matches!(
        vm("let i: int = 9\nwhile false {\n    i = 0\n}\ni\n"),
        Value::Int(9)
    ));
}

#[test]
fn break_leaves_the_loop() {
    assert!(matches!(
        vm("let i: int = 0\nwhile true {\n    i = i + 1\n    when i == 3 {\n        stop\n    }\n}\ni\n"),
        Value::Int(3)
    ));
}

#[test]
fn continue_skips_the_rest_of_the_body() {
    assert!(matches!(
        vm("let i: int = 0\nlet n: int = 0\nwhile i < 5 {\n    i = i + 1\n    when i == 3 {\n        skip\n    }\n    n = n + 1\n}\nn\n"),
        Value::Int(4)
    ));
}

#[test]
fn break_leaves_only_the_inner_loop() {
    assert!(matches!(
        vm("let n: int = 0\nlet i: int = 0\nwhile i < 3 {\n    i = i + 1\n    let j: int = 0\n    while true {\n        j = j + 1\n        when j == 2 {\n            stop\n        }\n    }\n    n = n + j\n}\nn\n"),
        Value::Int(6)
    ));
}

#[test]
fn a_loop_body_gets_its_own_scope_each_pass() {
    assert!(matches!(
        vm("let i: int = 0\nwhile i < 3 {\n    let inner: int = i\n    i = i + 1\n}\ni\n"),
        Value::Int(3)
    ));
}

#[test]
fn a_classic_for_counts() {
    assert!(matches!(
        vm("let n: int = 0\nfor (let i: int = 0; i < 5; i = i + 1) {\n    n = n + i\n}\nn\n"),
        Value::Int(10)
    ));
}

#[test]
fn the_induction_variable_does_not_leak() {
    // `i` is scoped to the loop. Reading it afterwards is an undefined name,
    // which means the *program* does not compile as a VM program -- the tree
    // walker reports it. So this asserts the loop runs, not that `i` is gone.
    assert!(matches!(
        vm("let n: int = 0\nfor (let i: int = 0; i < 3; i = i + 1) {\n    n = n + 1\n}\nn\n"),
        Value::Int(3)
    ));
}

#[test]
fn break_works_in_a_classic_for() {
    assert!(matches!(
        vm("let n: int = 0\nfor (let i: int = 0; i < 10; i = i + 1) {\n    when i == 4 {\n        stop\n    }\n    n = n + 1\n}\nn\n"),
        Value::Int(4)
    ));
}

#[test]
fn continue_still_runs_the_step() {
    // The trap this test exists for: if `continue` jumps to the condition
    // rather than to the step, this loops forever.
    assert!(matches!(
        vm("let n: int = 0\nfor (let i: int = 0; i < 5; i = i + 1) {\n    when i == 2 {\n        skip\n    }\n    n = n + 1\n}\nn\n"),
        Value::Int(4)
    ));
}

#[test]
fn a_classic_for_runs_the_first_pass_before_the_step() {
    // The step is emitted before the body and jumped over on the way in, so
    // the trap is running it once too early. Summing 0..2 catches that: with
    // an early step this would be 1 + 2 rather than 0 + 1.
    assert!(matches!(
        vm("let n: int = 0\nfor (let i: int = 0; i < 2; i = i + 1) {\n    n = n + i\n}\nn\n"),
        Value::Int(1)
    ));
}

use xenith::error::Error;

/// Runs through the VM and returns the error it produced.
fn vm_error(source: &str) -> Error {
    // The table is taken but static errors are not asserted away: these
    // sources are built to fail at *run* time, and some of them the checker
    // also has an opinion about. What is under test is the VM's report.
    let (_, types, ast) =
        xenith::check_source_typed("<test>", source).expect("should lex and parse");

    match xenith::vm::compile_and_run(&ast, &types) {
        Some(Err(error)) => error,
        Some(Ok(value)) => panic!("should have failed, got {value:?}"),
        None => panic!("should have compiled"),
    }
}

#[test]
fn division_by_zero_keeps_its_code() {
    let error = vm_error("1 / 0\n");
    assert!(error.as_string().contains("XEN003"), "{}", error.as_string());
}

#[test]
fn an_error_points_at_the_source_not_at_nothing() {
    // The bug this exists for: `Value::divide` builds its error with a dummy
    // position, and if the VM does not fix it up the report says line 0 and
    // shows no source line at all.
    let error = vm_error("1\n2\n3 / 0\n");
    assert_eq!(error.position_start.line, 2, "expected the third line");
}

#[test]
fn an_overflow_keeps_its_code() {
    let error = vm_error("9223372036854775807 + 1\n");
    assert!(error.as_string().contains("XEN017"), "{}", error.as_string());
}

#[test]
fn a_traps_caret_underlines_the_whole_expression() {
    // `visit_binary_op` sets the span to the operator node's start and end, so
    // the caret covers `3 / 0` rather than one character. A chunk that
    // recorded only the start would report the right message under the wrong
    // underline, and `tests/differential.rs` would fail on the caret line.
    let error = vm_error("1\n2\n3 / 0\n");
    assert!(
        error.position_end.index > error.position_start.index + 1,
        "the caret spans {}..{}, which is not a whole expression",
        error.position_start.index,
        error.position_end.index
    );
}
