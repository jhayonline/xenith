//! Compiler tests, asserted against disassembly text.
//!
//! Text rather than an `Instr` vector on purpose: when one of these fails, the
//! message shows the code that was actually emitted, which is the thing you
//! need in order to see a register allocation bug.

use xenith::values::Value;
use xenith::vm::chunk::{Chunk, Instr};

#[test]
fn a_chunk_disassembles() {
    let mut chunk = Chunk::new();
    let k = chunk.add_constant(Value::int(7));
    chunk.push(Instr::LoadConst { dst: 0, k });
    chunk.push(Instr::LoadConst { dst: 1, k });
    chunk.push(Instr::Add { dst: 2, a: 0, b: 1 });
    chunk.push(Instr::Halt { src: 2 });
    chunk.registers = 3;

    assert_eq!(
        chunk.disassemble(),
        "\
constants:
  k0  int 7
registers: 3
code:
  0000  LOAD_CONST   r0, k0
  0001  LOAD_CONST   r1, k0
  0002  ADD          r2, r0, r1
  0003  HALT         r2
"
    );
}

#[test]
fn an_identical_constant_is_interned_once() {
    let mut chunk = Chunk::new();
    let a = chunk.add_constant(Value::int(7));
    let b = chunk.add_constant(Value::int(7));
    assert_eq!(a, b);
    assert_eq!(chunk.constants.len(), 1);
}

#[test]
fn an_int_and_a_float_are_not_the_same_constant() {
    // `1` and `1.0` are different values in Xenith -- mixing them is an error,
    // not a promotion -- so they must not share a constant slot.
    let mut chunk = Chunk::new();
    let i = chunk.add_constant(Value::int(1));
    let f = chunk.add_constant(Value::float(1.0));
    assert_ne!(i, f);
}

#[test]
fn a_jump_target_is_shown_as_an_address() {
    let mut chunk = Chunk::new();
    chunk.push(Instr::LoadBool { dst: 0, value: true });
    chunk.push(Instr::JumpIfFalse { cond: 0, to: 3 });
    chunk.push(Instr::Jump { to: 0 });
    chunk.push(Instr::Halt { src: 0 });
    chunk.registers = 1;

    assert_eq!(
        chunk.disassemble(),
        "\
constants:
  (none)
registers: 1
code:
  0000  LOAD_BOOL    r0, true
  0001  JUMP_IF_FALSE r0, @0003
  0002  JUMP         @0000
  0003  HALT         r0
"
    );
}

use xenith::lexer::Lexer;
use xenith::nodes::Node;
use xenith::parser::Parser;
use xenith::vm::compile::{compile, Unsupported};

fn parse(source: &str) -> Node {
    let mut lexer = Lexer::new("<test>".to_string(), source.to_string());
    let tokens = lexer.make_tokens().expect("should lex");
    let mut parser = Parser::new(tokens);
    let result = parser.parse();
    assert!(result.error.is_none(), "should parse: {:?}", result.error);
    result.node.expect("should produce a node")
}

/// Compiles, or fails the test with what stopped it.
fn dis(source: &str) -> String {
    match compile(&parse(source)) {
        Ok(chunk) => chunk.disassemble(),
        Err(Unsupported { what }) => panic!("should have compiled, but: {what}"),
    }
}

/// What stopped the compiler, for the cases that are meant to bail out.
fn refuses(source: &str) -> String {
    match compile(&parse(source)) {
        Ok(_) => panic!("should not have compiled"),
        Err(Unsupported { what }) => what,
    }
}

#[test]
fn an_int_literal_loads_a_constant() {
    assert_eq!(
        dis("42\n"),
        "\
constants:
  k0  int 42
registers: 1
code:
  0000  LOAD_CONST   r0, k0
  0001  HALT         r0
"
    );
}

#[test]
fn a_float_literal_is_a_separate_constant() {
    assert_eq!(
        dis("1.5\n"),
        "\
constants:
  k0  float 1.5
registers: 1
code:
  0000  LOAD_CONST   r0, k0
  0001  HALT         r0
"
    );
}

#[test]
fn booleans_and_null_need_no_constant_slot() {
    assert_eq!(
        dis("true\n"),
        "\
constants:
  (none)
registers: 1
code:
  0000  LOAD_BOOL    r0, true
  0001  HALT         r0
"
    );
    assert_eq!(
        dis("null\n"),
        "\
constants:
  (none)
registers: 1
code:
  0000  LOAD_NULL    r0
  0001  HALT         r0
"
    );
}

#[test]
fn an_empty_program_halts_on_null() {
    assert_eq!(
        dis("\n"),
        "\
constants:
  (none)
registers: 1
code:
  0000  LOAD_NULL    r0
  0001  HALT         r0
"
    );
}

#[test]
fn a_method_declaration_is_not_supported_yet() {
    assert_eq!(
        refuses("method f() -> int {\n  release 1\n}\n"),
        "a method declaration"
    );
}

#[test]
fn a_list_literal_is_not_supported_yet() {
    // A bare list, not `let xs = [...]`: assignment is its own unsupported
    // reason until task 6, and this test is about the list.
    assert_eq!(refuses("[1, 2]\n"), "a list literal");
}

#[test]
fn addition_is_three_address() {
    // Two operands into two registers, the result into a third. No shuffling,
    // which is the whole point of a register machine over a stack one.
    assert_eq!(
        dis("1 + 2\n"),
        "\
constants:
  k0  int 1
  k1  int 2
registers: 2
code:
  0000  LOAD_CONST   r0, k0
  0001  LOAD_CONST   r1, k1
  0002  ADD          r0, r0, r1
  0003  HALT         r0
"
    );
}

#[test]
fn temporaries_are_reused_across_statements() {
    // Two statements, each using two registers, must not need four. The frame
    // stays at 2 because the first statement's temporaries are released.
    let text = dis("1 + 2\n3 + 4\n");
    assert!(
        text.contains("registers: 2"),
        "temporaries were not reused:\n{text}"
    );
}

#[test]
fn nesting_deepens_the_frame() {
    let text = dis("1 + 2 * 3\n");
    assert!(
        text.contains("registers: 3"),
        "expected three registers for a nested expression:\n{text}"
    );
}

#[test]
fn a_negative_literal_is_folded_by_the_lexer() {
    // `-5` never reaches the compiler as a unary operator: the lexer produces
    // a single negative number token. Asserted rather than assumed, because
    // it is the reason the NEG test below needs a non-literal operand.
    assert_eq!(
        dis("-5\n"),
        "\
constants:
  k0  int -5
registers: 1
code:
  0000  LOAD_CONST   r0, k0
  0001  HALT         r0
"
    );
}

#[test]
fn unary_minus_compiles() {
    assert_eq!(
        dis("-(1 + 2)\n"),
        "\
constants:
  k0  int 1
  k1  int 2
registers: 2
code:
  0000  LOAD_CONST   r0, k0
  0001  LOAD_CONST   r1, k1
  0002  ADD          r0, r0, r1
  0003  NEG          r0, r0
  0004  HALT         r0
"
    );
}

#[test]
fn unary_not_compiles() {
    assert_eq!(
        dis("!true\n"),
        "\
constants:
  (none)
registers: 1
code:
  0000  LOAD_BOOL    r0, true
  0001  NOT          r0, r0
  0002  HALT         r0
"
    );
}
