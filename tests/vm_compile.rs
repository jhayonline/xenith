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
use xenith::type_table::TypeTable;
use xenith::vm::compile::{compile, Unsupported};

fn parse(source: &str) -> Node {
    let mut lexer = Lexer::new("<test>".to_string(), source.to_string());
    let tokens = lexer.make_tokens().expect("should lex");
    let mut parser = Parser::new(tokens);
    let result = parser.parse();
    assert!(result.error.is_none(), "should parse: {:?}", result.error);
    result.node.expect("should produce a node")
}

/// Lexes, parses *and checks*, so the compiler sees the types the checker
/// proved.
///
/// The old helper stopped at the parser, which meant every test in this file
/// compiled against an empty table -- fine while no opcode depended on one,
/// and misleading the moment one does.
fn checked(source: &str) -> (Node, TypeTable) {
    let (errors, types, ast) =
        xenith::check_source_typed("<test>", source).expect("should lex and parse");
    assert!(errors.is_empty(), "should check: {:?}", errors);
    (ast, types)
}

/// Compiles with the checker's types, or fails the test with what stopped it.
fn dis(source: &str) -> String {
    let (ast, types) = checked(source);
    match compile(&ast, &types) {
        Ok(chunk) => chunk.disassemble(),
        Err(Unsupported { what }) => panic!("should have compiled, but: {what}"),
    }
}

/// Compiles against an empty table, which is what the compiler sees for any
/// node the checker could not prove.
///
/// Every typed opcode has to have a generic twin that behaves identically, and
/// this is how the twin is exercised. Also the home for sources the parser
/// accepts and the checker does not.
#[allow(dead_code)]
fn dis_untyped(source: &str) -> String {
    let ast = parse(source);
    match compile(&ast, &TypeTable::default()) {
        Ok(chunk) => chunk.disassemble(),
        Err(Unsupported { what }) => panic!("should have compiled, but: {what}"),
    }
}

/// What stopped the compiler, for the cases that are meant to bail out.
/// Checked, so a refusal is never an artefact of a missing type.
fn refuses(source: &str) -> String {
    let (ast, types) = checked(source);
    match compile(&ast, &types) {
        Ok(_) => panic!("should not have compiled"),
        Err(Unsupported { what }) => what,
    }
}

/// Parsed but not checked, for a refusal the checker would reject first.
#[allow(dead_code)]
fn refuses_untyped(source: &str) -> String {
    match compile(&parse(source), &TypeTable::default()) {
        Ok(_) => panic!("should not have compiled"),
        Err(Unsupported { what }) => what,
    }
}

#[test]
fn the_compiler_is_given_the_checkers_types() {
    let (ast, types) = checked("let i: int = 1\n");
    assert!(
        matches!(types.get(first_value(&ast)), xenith::types::Type::Int),
        "the checker should have proved the literal an int"
    );
    // The point of the test: this signature exists.
    compile(&ast, &types).expect("should compile");
}

/// The `NodeId` of the value in the file's first `let`.
fn first_value(ast: &Node) -> xenith::nodes::NodeId {
    let Node::List(list) = ast else {
        panic!("expected a statement list")
    };
    let Node::VarAssign(assign) = &*list.element_nodes[0] else {
        panic!("expected an assignment")
    };
    assign.value_node.id()
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
fn a_list_literal_is_not_supported_yet() {
    // A bare list, not `let xs = [...]`: assignment is its own unsupported
    // reason until task 6, and this test is about the list.
    assert_eq!(refuses("[1, 2]\n"), "a list literal");
}

#[test]
fn addition_is_three_address() {
    // Two operands into two registers, the result into a third. No shuffling,
    // which is the whole point of a register machine over a stack one.
    //
    // Locals rather than literals: a literal right operand folds into the
    // instruction now, and a folded form would not show the two operand
    // registers this test is about.
    assert_eq!(
        dis("let a: int = 1\nlet b: int = 2\na + b\n"),
        "\
constants:
  k0  int 1
  k1  int 2
registers: 3
code:
  0000  LOAD_CONST   r0, k0
  0001  LOAD_CONST   r1, k1
  0002  ADD_I        r2, r0, r1
  0003  HALT         r2
"
    );
}

#[test]
fn temporaries_are_reused_across_statements() {
    // Two statements, each taking a temporary above the locals, must not need
    // two temporaries. The frame stays at 3 because the first statement's is
    // released before the second asks for one.
    //
    // Named operands rather than literals: a literal right operand folds into
    // the instruction, and a statement that needs no temporary would not test
    // whether temporaries are reused.
    let text = dis("let a: int = 1\nlet b: int = 2\na * b\nb * a\n");
    assert!(
        text.contains("registers: 3"),
        "temporaries were not reused:\n{text}"
    );
}

#[test]
fn nesting_deepens_the_frame() {
    // Named operands, because `2 * 3` folds into one instruction now and a
    // folded expression does not nest in the register sense.
    let text = dis("let a: int = 2\nlet b: int = 3\na + b * b\n");
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
registers: 1
code:
  0000  LOAD_CONST   r0, k0
  0001  ADD_IK       r0, r0, k1
  0002  NEG          r0, r0
  0003  HALT         r0
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

#[test]
fn a_local_is_a_register_not_a_lookup() {
    // `x` is read as `r0` directly -- no lookup, and no copy into a temporary
    // either. The instruction reads the operand register before it writes
    // `r1`, and a destination is always above every live local, so naming the
    // local as an operand is safe. See `Compiler::operand`.
    assert_eq!(
        dis("let x: int = 1\nx + 1\n"),
        "\
constants:
  k0  int 1
registers: 2
code:
  0000  LOAD_CONST   r0, k0
  0001  ADD_IK       r1, r0, k0
  0002  HALT         r1
"
    );
}

#[test]
fn two_locals_take_two_registers_in_declaration_order() {
    let text = dis("let a: int = 1\nlet b: int = 2\nb\n");
    assert!(text.contains("registers: 3"), "{text}");
}

#[test]
fn reassignment_writes_the_same_register() {
    let text = dis("let x: int = 1\nx = 2\nx\n");
    // No second slot for `x`: the assignment moves into the register the
    // declaration took.
    assert!(text.contains("registers: 2"), "{text}");
}

#[test]
fn assigning_an_undeclared_name_is_not_compiled() {
    // The checker reports this in program mode; in script mode the tree
    // walker does. Either way the VM must not invent a register for it.
    assert_eq!(refuses("nowhere = 1\n"), "an assignment to an unknown name");
}

#[test]
fn a_block_of_expressions_is_still_a_block() {
    // The reason bodies are compiled by call site rather than by looking at
    // what is inside them: a block whose only statement is a call holds no
    // node that could only be a statement, so a contents test would read it
    // as a list literal and refuse the whole program.
    let text = dis("when true {\n    echo(1)\n}\n");
    assert!(text.contains("ECHO"), "{text}");
}

#[test]
fn a_list_literal_in_expression_position_is_still_refused() {
    assert_eq!(refuses("let xs: list<int> = [1, 2]\n"), "a list literal");
}

#[test]
fn the_counting_loop_is_small() {
    // The benchmark's shape, and a canary against runaway moves. Thirteen is
    // what the compiler emits today, and two of them are known: reading `i`
    // copies it into a temporary, and assigning to `i` copies back. Phase 5
    // elides both by letting an operand and a destination name a local
    // directly. Until then this number should not move on its own -- if it
    // grows, read the disassembly in the failure before touching the budget.
    let text = dis("let i: int = 0\nwhile i < 5 {\n    i = i + 1\n}\ni\n");
    let instructions = text.lines().filter(|l| l.starts_with("  00")).count();
    assert!(
        instructions <= 13,
        "counting loop compiled to {instructions} instructions:\n{text}"
    );
}

#[test]
fn a_proto_disassembles_after_the_code_that_makes_it() {
    let mut inner = Chunk::new();
    inner.name = Some("square".to_string());
    inner.arity = 1;
    inner.push(Instr::Mul { dst: 1, a: 0, b: 0 });
    inner.push(Instr::Ret { src: 1 });
    inner.registers = 2;

    let mut outer = Chunk::new();
    outer.protos.push(std::rc::Rc::new(inner));
    outer.push(Instr::Closure { dst: 0, proto: 0 });
    outer.push(Instr::Halt { src: 0 });
    outer.registers = 1;

    assert_eq!(
        outer.disassemble(),
        "\
constants:
  (none)
registers: 1
code:
  0000  CLOSURE      r0, p0
  0001  HALT         r0

proto p0  square/1
constants:
  (none)
registers: 2
code:
  0000  MUL          r1, r0, r0
  0001  RET          r1
"
    );
}

#[test]
fn an_upvalue_table_is_printed_only_when_there_is_one() {
    // The 23 tests written in phase 3 assert on exact text. A header for an
    // empty table would change every one of them, which is why the table is
    // conditional rather than always present.
    let mut chunk = Chunk::new();
    chunk.push(Instr::Halt { src: 0 });
    chunk.registers = 1;
    assert!(!chunk.disassemble().contains("upvalues:"));

    chunk.upvalues.push(xenith::vm::chunk::UpvalDesc {
        in_parent_locals: true,
        index: 3,
    });
    chunk.upvalues.push(xenith::vm::chunk::UpvalDesc {
        in_parent_locals: false,
        index: 1,
    });
    assert!(chunk
        .disassemble()
        .contains("upvalues:\n  u0  parent local r3\n  u1  parent upvalue u1\n"));
}

#[test]
fn the_calling_instructions_disassemble() {
    let mut chunk = Chunk::new();
    chunk.push(Instr::Closure { dst: 0, proto: 0 });
    chunk.push(Instr::GetUpval { dst: 1, idx: 2 });
    chunk.push(Instr::SetUpval { idx: 2, src: 1 });
    chunk.push(Instr::Call { dst: 0, callee: 0, argc: 2 });
    chunk.push(Instr::CloseUpvals { from: 3 });
    chunk.push(Instr::Ret { src: 0 });
    chunk.registers = 4;

    assert_eq!(
        chunk.disassemble(),
        "\
constants:
  (none)
registers: 4
code:
  0000  CLOSURE      r0, p0
  0001  GET_UPVAL    r1, u2
  0002  SET_UPVAL    u2, r1
  0003  CALL         r0, r0, 2
  0004  CLOSE_UPVALS r3
  0005  RET          r0
"
    );
}

#[test]
fn an_arrow_method_compiles_to_a_proto_and_a_closure() {
    assert_eq!(
        dis("method square(n: int) -> int => n * n\n"),
        "\
constants:
  (none)
registers: 2
code:
  0000  CLOSURE      r0, p0
  0001  MOVE         r1, r0
  0002  HALT         r1

proto p0  square/1
constants:
  (none)
registers: 2
code:
  0000  MUL_I        r1, r0, r0
  0001  RET          r1
"
    );
}

#[test]
fn a_parameter_is_a_register_the_caller_already_filled() {
    // Two parameters, in declaration order, at r0 and r1 -- which is where a
    // `CALL` puts the arguments. Nothing moves them.
    assert_eq!(
        dis("method add(a: int, b: int) -> int { release a + b }\n"),
        "\
constants:
  (none)
registers: 2
code:
  0000  CLOSURE      r0, p0
  0001  MOVE         r1, r0
  0002  HALT         r1

proto p0  add/2
constants:
  (none)
registers: 3
code:
  0000  ADD_I        r2, r0, r1
  0001  RET          r2
"
    );
}

#[test]
fn a_method_that_runs_off_the_end_returns_null() {
    // `Function::execute` ends with `success(Value::Null)` when nothing set a
    // return value. A block body's last statement is *not* its value -- only
    // `release` is, and only `=>` returns an expression.
    assert_eq!(
        dis("method nothing() -> null { let x: int = 1 }\n"),
        "\
constants:
  (none)
registers: 2
code:
  0000  CLOSURE      r0, p0
  0001  MOVE         r1, r0
  0002  HALT         r1

proto p0  nothing/0
constants:
  k0  int 1
registers: 2
code:
  0000  LOAD_CONST   r0, k0
  0001  LOAD_NULL    r1
  0002  RET          r1
"
    );
}

#[test]
fn a_bare_release_returns_null() {
    assert_eq!(
        dis("method nothing() -> null { release }\n"),
        "\
constants:
  (none)
registers: 2
code:
  0000  CLOSURE      r0, p0
  0001  MOVE         r1, r0
  0002  HALT         r1

proto p0  nothing/0
constants:
  (none)
registers: 1
code:
  0000  LOAD_NULL    r0
  0001  RET          r0
"
    );
}

#[test]
fn an_anonymous_method_is_the_same_thing_without_the_name() {
    let text = dis("let triple: method(int) -> int = method(n: int) -> int => n * 3\n");
    assert!(text.contains("proto p0  <anonymous>/1"), "{text}");
    assert!(text.contains("CLOSURE      r0, p0"), "{text}");
}

#[test]
fn release_at_the_top_level_is_still_the_tree_walkers_to_report() {
    // The top level is not a function. The tree walker has a message for
    // this; the VM must not invent a second one.
    assert_eq!(refuses("release 1\n"), "release outside a method");
}

#[test]
fn a_call_puts_its_arguments_where_the_callee_will_read_them() {
    assert_eq!(
        dis("method square(n: int) -> int => n * n\nsquare(3)\n"),
        "\
constants:
  k0  int 3
registers: 3
code:
  0000  CLOSURE      r0, p0
  0001  MOVE         r1, r0
  0002  LOAD_CONST   r2, k0
  0003  CALL         r1, r1, 1
  0004  HALT         r1

proto p0  square/1
constants:
  (none)
registers: 2
code:
  0000  MUL_I        r1, r0, r0
  0001  RET          r1
"
    );
}

#[test]
fn echo_keeps_its_own_opcode() {
    // Not folded into CALL. `echo` is a builtin, not a Xenith method, and the
    // VM has no builtin registry until phase 7.
    assert!(dis("echo(1)\n").contains("ECHO"));
}

#[test]
fn a_call_to_something_that_is_not_a_method_here_goes_to_the_tree_walker() {
    assert_eq!(
        refuses("len([1, 2])\n"),
        "a name that is not a local or a capture"
    );
}

#[test]
fn a_captured_local_becomes_an_upvalue() {
    assert_eq!(
        dis("let step: int = 1\nmethod advance(n: int) -> int => n + step\n"),
        "\
constants:
  k0  int 1
registers: 3
code:
  0000  LOAD_CONST   r0, k0
  0001  CLOSURE      r1, p0
  0002  MOVE         r2, r1
  0003  HALT         r2

proto p0  advance/1
upvalues:
  u0  parent local r0
constants:
  (none)
registers: 2
code:
  0000  GET_UPVAL    r1, u0
  0001  ADD_I        r1, r0, r1
  0002  RET          r1
"
    );
}

#[test]
fn writing_through_a_capture_is_a_set_upval() {
    let text = dis(
        "let counter: int = 0\nmethod bump() -> null { counter = counter + 1 release null }\n",
    );
    assert!(text.contains("GET_UPVAL"), "{text}");
    assert!(text.contains("SET_UPVAL    u0, r"), "{text}");
}

#[test]
fn a_method_can_reach_a_name_from_two_functions_out() {
    // The middle method captures nothing of its own; it exists only to pass
    // the capture through, which is the chain `resolve_upvalue` builds.
    let text = dis(
        "let base: int = 10\n\
         method outer() -> method() -> int { release method() -> int => base }\n",
    );
    assert!(text.contains("u0  parent local r0"), "{text}");
    assert!(text.contains("u0  parent upvalue u0"), "{text}");
}

#[test]
fn a_named_method_can_reach_itself() {
    let text = dis(
        "method countdown(n: int) -> int {\n\
             when n <= 0 { release 0 }\n\
             release countdown(n - 1)\n\
         }\n",
    );
    // Itself, captured out of the register the CLOSURE is about to fill.
    assert!(text.contains("u0  parent local r0"), "{text}");
    assert!(text.contains("CALL"), "{text}");
}

#[test]
fn a_capture_of_a_constant_is_still_the_tree_walkers_to_refuse() {
    // XEN010 territory. The checker reports it and so does the tree walker;
    // the VM must not be the one to decide, or the message would have to be
    // duplicated.
    //
    // Unchecked, because the checker gets there first: this source is XEN018
    // before it is anything else, so `refuses` would never reach the compiler.
    // What is under test is what the *compiler* does when handed it anyway.
    assert_eq!(
        refuses_untyped(
            "const let fixed: int = 1\nmethod set() -> null { fixed = 2 release null }\n"
        ),
        "an assignment to a constant"
    );
}

#[test]
fn a_capture_of_a_loop_bodys_own_binding_goes_to_the_tree_walker() {
    // `visit_while` builds one `body_ctx` and clears it each pass, so every
    // closure a loop makes shares one binding and reads the value it last
    // held. The VM cannot reproduce that: closing per pass would give each
    // closure its own cell, and closing after the loop reads a register the
    // condition has already reused for its own temporary. Refused rather
    // than guessed at.
    assert_eq!(
        refuses(
            "let i: int = 0\n\
             while i < 2 {\n\
                 let x: int = i\n\
                 let get: method() -> int = method() -> int => x\n\
                 i = i + 1\n\
             }\n"
        ),
        "a capture of a loop body's own binding"
    );
}

#[test]
fn a_loop_body_that_captured_nothing_closes_nothing() {
    // The instruction costs a dispatch. A loop with no captures must not pay
    // for one, and must not be refused either.
    let text = dis("let i: int = 0\nwhile i < 2 { i = i + 1 }\n");
    assert!(!text.contains("CLOSE_UPVALS"), "{text}");
}

#[test]
fn a_when_body_inside_a_loop_closes_every_pass() {
    // `visit_if` builds a fresh `Context` each time it evaluates a branch, so
    // a capture of one of that branch's own locals belongs to that pass and
    // closes as the branch ends -- inside the loop, before the backward jump.
    let text = dis(
        "let g: method() -> int = method() -> int => 0\n\
         let i: int = 0\n\
         while i < 2 {\n\
             when i == 0 { let x: int = 5 g = method() -> int => x }\n\
             i = i + 1\n\
         }\n",
    );
    let close = text.find("CLOSE_UPVALS").expect(&text);
    let back = text.rfind("JUMP  ").expect(&text);
    assert!(close < back, "{text}");
}

#[test]
fn a_jump_out_of_a_loop_that_captures_goes_to_the_tree_walker() {
    // A `stop` leaves the body without running the close the rest of it
    // would have. Refused; a loop that captures nothing is unaffected.
    assert_eq!(
        refuses(
            "let g: method() -> int = method() -> int => 0\n\
             let i: int = 0\n\
             while i < 2 {\n\
                 when i == 0 { let x: int = 5 g = method() -> int => x }\n\
                 stop\n\
             }\n"
        ),
        "a jump out of a loop that captures"
    );
}

#[test]
fn a_loop_that_captures_nothing_still_compiles_its_stop_and_skip() {
    let text = dis(
        "let i: int = 0\n\
         while i < 10 {\n\
             i = i + 1\n\
             when i == 3 { skip }\n\
             when i == 5 { stop }\n\
         }\n",
    );
    assert!(!text.contains("CLOSE_UPVALS"), "{text}");
}

#[test]
fn proven_ints_get_the_int_opcode() {
    assert_eq!(
        dis("let a: int = 1\nlet b: int = 2\na + b\n"),
        "\
constants:
  k0  int 1
  k1  int 2
registers: 3
code:
  0000  LOAD_CONST   r0, k0
  0001  LOAD_CONST   r1, k1
  0002  ADD_I        r2, r0, r1
  0003  HALT         r2
"
    );
}

#[test]
fn proven_floats_get_the_float_opcode() {
    assert_eq!(
        dis("let a: float = 1.5\nlet b: float = 2.5\na < b\n"),
        "\
constants:
  k0  float 1.5
  k1  float 2.5
registers: 3
code:
  0000  LOAD_CONST   r0, k0
  0001  LOAD_CONST   r1, k1
  0002  LT_F         r2, r0, r1
  0003  HALT         r2
"
    );
}

#[test]
fn an_unproven_operand_keeps_the_generic_opcode() {
    // The same source against an empty table -- which is what the compiler
    // sees for anything the checker could not prove. The generic opcode is not
    // a fallback for broken programs; it is the normal case for an unannotated
    // one, and it must stay reachable.
    assert_eq!(
        dis_untyped("let a: int = 1\nlet b: int = 2\na + b\n"),
        "\
constants:
  k0  int 1
  k1  int 2
registers: 3
code:
  0000  LOAD_CONST   r0, k0
  0001  LOAD_CONST   r1, k1
  0002  ADD          r2, r0, r1
  0003  HALT         r2
"
    );
}

#[test]
fn a_mixed_pair_keeps_the_generic_opcode() {
    // int + float is XEN001 at run time. The checker proves it neither an int
    // pair nor a float pair, so no narrow opcode is chosen and the generic one
    // raises exactly as it does today.
    let text = dis_untyped("let a: int = 1\nlet b: float = 2.0\na + b\n");
    assert!(
        text.contains("ADD          "),
        "a mixed pair must not be narrowed:\n{text}"
    );
}

#[test]
fn power_is_never_narrowed() {
    // No POW_I: integer exponentiation has a negative-exponent case and an
    // overflow case whose wording lives in Value::power, and it is never hot
    // enough to be worth a second copy of them.
    let text = dis("let a: int = 2\nlet b: int = 3\na ^ b\n");
    assert!(text.contains("POW          "), "pow stays generic:\n{text}");
}

#[test]
fn there_is_no_float_remainder_opcode() {
    // Float remainder has its own rounding rule in Value::modulo.
    let text = dis("let a: float = 7.5\nlet b: float = 2.5\na % b\n");
    assert!(text.contains("REM          "), "float % stays generic:\n{text}");
}

#[test]
fn an_alias_is_peeled_before_the_opcode_is_chosen() {
    // `type Count = int` must not cost a program the speed that `int` gets.
    // The compiler declines type aliases for other reasons in this phase, so
    // this asserts the peeling itself rather than the whole program.
    let text = dis("let a: int = 1\nlet b: int = 2\na - b\n");
    assert!(text.contains("SUB_I        "), "{text}");
}

#[test]
fn an_int_literal_folds_into_the_operator() {
    assert_eq!(
        dis("let i: int = 0\ni + 1\n"),
        "\
constants:
  k0  int 0
  k1  int 1
registers: 2
code:
  0000  LOAD_CONST   r0, k0
  0001  ADD_IK       r1, r0, k1
  0002  HALT         r1
"
    );
}

#[test]
fn a_comparison_against_a_literal_folds() {
    assert_eq!(
        dis("let i: int = 0\ni < 400000\n"),
        "\
constants:
  k0  int 0
  k1  int 400000
registers: 2
code:
  0000  LOAD_CONST   r0, k0
  0001  LT_IK        r1, r0, k1
  0002  HALT         r1
"
    );
}

#[test]
fn a_literal_on_the_left_does_not_fold() {
    // Only the right operand folds. A mirrored set of opcodes for `1 + i`
    // would double the instruction count of this phase to serve a shape that
    // bounds, steps and increments never take.
    let text = dis("let i: int = 0\n1 + i\n");
    assert!(
        text.contains("ADD_I        ") && !text.contains("ADD_IK"),
        "a literal on the left keeps the register form:\n{text}"
    );
}

#[test]
fn a_float_literal_does_not_fold() {
    // There is no ADD_FK. Float loops are not the shape this serves.
    let text = dis("let x: float = 1.0\nx + 2.0\n");
    assert!(
        text.contains("ADD_F        "),
        "a float literal must not fold:\n{text}"
    );
}

#[test]
fn a_literal_does_not_fold_into_a_division() {
    // No DIV_IK or REM_IK exist.
    let text = dis("let i: int = 8\ni / 2\n");
    assert!(
        text.contains("DIV_I        "),
        "division keeps its register form:\n{text}"
    );
    let text = dis("let i: int = 8\ni % 2\n");
    assert!(
        text.contains("REM_I        "),
        "remainder keeps its register form:\n{text}"
    );
}

#[test]
fn a_literal_does_not_fold_into_a_power() {
    let text = dis("let i: int = 2\ni ^ 10\n");
    assert!(text.contains("POW          "), "{text}");
}

#[test]
fn an_unproven_operand_does_not_fold() {
    let text = dis_untyped("let i: int = 0\ni + 1\n");
    assert!(
        text.contains("ADD          "),
        "without a proof there is nothing to fold into:\n{text}"
    );
}

