//! Calls, recursion and captures, end to end.
//!
//! The chunks here are built by hand rather than compiled. That keeps the
//! interpreter loop testable apart from the compiler, which is what you want
//! on the day a call returns the wrong number and you need to know which half
//! is wrong.

use std::rc::Rc;

use xenith::types::Type;
use xenith::values::Value;
use xenith::vm::chunk::{Chunk, Instr, UpvalDesc};

/// `method double(n: int) -> int => n + n`, called with 21.
fn double_proto() -> Chunk {
    let mut proto = Chunk::new();
    proto.name = Some("double".to_string());
    proto.arity = 1;
    proto.param_types = Rc::new(vec![Type::Int]);
    // The parameter is register 0 of the callee's frame, which is the
    // register the caller put the argument in. Nothing was moved to get it
    // there.
    proto.push(Instr::Add { dst: 1, a: 0, b: 0 });
    proto.push(Instr::Ret { src: 1 });
    proto.registers = 2;
    proto
}

#[test]
fn a_call_returns_its_value() {
    let mut chunk = Chunk::new();
    chunk.protos.push(Rc::new(double_proto()));
    let k = chunk.add_constant(Value::int(21));
    chunk.push(Instr::Closure { dst: 0, proto: 0 });
    chunk.push(Instr::LoadConst { dst: 1, k });
    chunk.push(Instr::Call { dst: 0, callee: 0, argc: 1 });
    chunk.push(Instr::Halt { src: 0 });
    chunk.registers = 2;

    let value = xenith::vm::run::execute(Rc::new(chunk)).expect("should run");
    assert!(matches!(value, Value::Int(42)), "got {:?}", value);
}

#[test]
fn too_many_arguments_is_xen015() {
    let mut chunk = Chunk::new();
    chunk.protos.push(Rc::new(double_proto()));
    let k = chunk.add_constant(Value::int(1));
    chunk.push(Instr::Closure { dst: 0, proto: 0 });
    chunk.push(Instr::LoadConst { dst: 1, k });
    chunk.push(Instr::LoadConst { dst: 2, k });
    chunk.push(Instr::Call { dst: 0, callee: 0, argc: 2 });
    chunk.push(Instr::Halt { src: 0 });
    chunk.registers = 3;

    let error = xenith::vm::run::execute(Rc::new(chunk)).expect_err("should fail");
    assert!(error.as_string().contains("XEN015"), "{}", error.as_string());
}

#[test]
fn too_few_arguments_is_xen016() {
    let mut chunk = Chunk::new();
    chunk.protos.push(Rc::new(double_proto()));
    chunk.push(Instr::Closure { dst: 0, proto: 0 });
    chunk.push(Instr::Call { dst: 0, callee: 0, argc: 0 });
    chunk.push(Instr::Halt { src: 0 });
    chunk.registers = 1;

    let error = xenith::vm::run::execute(Rc::new(chunk)).expect_err("should fail");
    assert!(error.as_string().contains("XEN016"), "{}", error.as_string());
}

#[test]
fn an_argument_of_the_wrong_type_is_xen001() {
    let mut chunk = Chunk::new();
    chunk.protos.push(Rc::new(double_proto()));
    let k = chunk.add_constant(Value::string_of(xenith::values::XenithString::new(
        "no".to_string(),
    )));
    chunk.push(Instr::Closure { dst: 0, proto: 0 });
    chunk.push(Instr::LoadConst { dst: 1, k });
    chunk.push(Instr::Call { dst: 0, callee: 0, argc: 1 });
    chunk.push(Instr::Halt { src: 0 });
    chunk.registers = 2;

    let error = xenith::vm::run::execute(Rc::new(chunk)).expect_err("should fail");
    assert_eq!(error.code, "XEN001");
    // The words matter as much as the code: these are the bytes the tree
    // walker builds, and the differential harness compares bytes. Asserted on
    // the fields rather than on `as_string`, which renders the note and the
    // help but not the message -- only `as_string_colored` prints that.
    assert_eq!(error.details, "expected `int`, found `string`");
    assert_eq!(
        error.note.as_deref(),
        Some("cannot assign `string` to variable of type `int`")
    );
}

#[test]
fn runaway_recursion_is_xen019_and_not_a_stack_overflow() {
    // `method loop_forever(n: int) -> int { release loop_forever(n) }`.
    // The VM's frames are a heap vector, so the depth limit is a counter and
    // not a race with the host stack.
    let mut proto = Chunk::new();
    proto.name = Some("loop_forever".to_string());
    proto.arity = 1;
    proto.param_types = Rc::new(vec![Type::Int]);
    // A method reaches itself through a capture of the register it was
    // stored in, which is upvalue 0 here.
    proto.upvalues.push(xenith::vm::chunk::UpvalDesc {
        in_parent_locals: true,
        index: 0,
    });
    proto.push(Instr::GetUpval { dst: 1, idx: 0 });
    proto.push(Instr::Move { dst: 2, src: 0 });
    proto.push(Instr::Call { dst: 1, callee: 1, argc: 1 });
    proto.push(Instr::Ret { src: 1 });
    proto.registers = 3;

    let mut chunk = Chunk::new();
    chunk.protos.push(Rc::new(proto));
    let k = chunk.add_constant(Value::int(0));
    chunk.push(Instr::Closure { dst: 0, proto: 0 });
    chunk.push(Instr::LoadConst { dst: 1, k });
    chunk.push(Instr::Call { dst: 0, callee: 0, argc: 1 });
    chunk.push(Instr::Halt { src: 0 });
    chunk.registers = 2;

    let error = xenith::vm::run::execute(Rc::new(chunk)).expect_err("should fail");
    assert_eq!(error.code, "XEN019");
    // On `details` rather than `as_string`, for the same reason the XEN001
    // test is: the plain formatter renders the note and the help but not the
    // message, and the method's name is in the message.
    assert_eq!(
        error.details,
        "call depth exceeded 10000 while calling `loop_forever`"
    );
}

/// `method read() -> int => captured`, where `captured` is the caller's r0.
fn reader_proto() -> Chunk {
    let mut proto = Chunk::new();
    proto.name = Some("read".to_string());
    proto.arity = 0;
    proto.upvalues.push(UpvalDesc {
        in_parent_locals: true,
        index: 0,
    });
    proto.push(Instr::GetUpval { dst: 0, idx: 0 });
    proto.push(Instr::Ret { src: 0 });
    proto.registers = 1;
    proto
}

#[test]
fn an_open_capture_sees_a_later_write() {
    // let x = 1; method read() => x; x = 50; read()  ->  50
    //
    // This is `closures.xen`'s `step`, which is the case that rules out
    // capturing by copy.
    let mut chunk = Chunk::new();
    chunk.protos.push(Rc::new(reader_proto()));
    let one = chunk.add_constant(Value::int(1));
    let fifty = chunk.add_constant(Value::int(50));
    chunk.push(Instr::LoadConst { dst: 0, k: one });
    chunk.push(Instr::Closure { dst: 1, proto: 0 });
    chunk.push(Instr::LoadConst { dst: 0, k: fifty });
    chunk.push(Instr::Call { dst: 1, callee: 1, argc: 0 });
    chunk.push(Instr::Halt { src: 1 });
    chunk.registers = 2;

    let value = xenith::vm::run::execute(Rc::new(chunk)).expect("should run");
    assert!(matches!(value, Value::Int(50)), "got {:?}", value);
}

#[test]
fn a_closed_capture_keeps_its_own_copy() {
    // `make_adder`: a method that makes a closure over its parameter and
    // returns it. When the frame goes, the capture must not follow the
    // register into the next call's frame.
    let mut inner = Chunk::new();
    inner.name = Some("adder".to_string());
    inner.arity = 1;
    inner.param_types = Rc::new(vec![Type::Int]);
    inner.upvalues.push(UpvalDesc {
        in_parent_locals: true,
        index: 0,
    });
    inner.push(Instr::GetUpval { dst: 1, idx: 0 });
    inner.push(Instr::Add { dst: 1, a: 0, b: 1 });
    inner.push(Instr::Ret { src: 1 });
    inner.registers = 2;

    let mut maker = Chunk::new();
    maker.name = Some("make_adder".to_string());
    maker.arity = 1;
    maker.param_types = Rc::new(vec![Type::Int]);
    maker.protos.push(Rc::new(inner));
    maker.push(Instr::Closure { dst: 1, proto: 0 });
    // The scope holding `n` ends with the method, and the return closes it.
    maker.push(Instr::Ret { src: 1 });
    maker.registers = 2;

    let mut chunk = Chunk::new();
    chunk.protos.push(Rc::new(maker));
    let ten = chunk.add_constant(Value::int(10));
    let one = chunk.add_constant(Value::int(1));
    let five = chunk.add_constant(Value::int(5));

    // add_ten = make_adder(10)
    chunk.push(Instr::Closure { dst: 0, proto: 0 });
    chunk.push(Instr::LoadConst { dst: 1, k: ten });
    chunk.push(Instr::Call { dst: 1, callee: 0, argc: 1 });
    // Wrong answer if the capture stayed open: this call reuses the register
    // `n` lived in.
    chunk.push(Instr::Move { dst: 2, src: 0 });
    chunk.push(Instr::LoadConst { dst: 3, k: one });
    chunk.push(Instr::Call { dst: 2, callee: 2, argc: 1 });
    // add_ten(5)
    chunk.push(Instr::Move { dst: 4, src: 1 });
    chunk.push(Instr::LoadConst { dst: 5, k: five });
    chunk.push(Instr::Call { dst: 4, callee: 4, argc: 1 });
    chunk.push(Instr::Halt { src: 4 });
    chunk.registers = 6;

    let value = xenith::vm::run::execute(Rc::new(chunk)).expect("should run");
    assert!(matches!(value, Value::Int(15)), "got {:?}", value);
}

#[test]
fn two_closures_over_one_variable_share_a_cell() {
    // `counter` in `tests/cases/methods.xen`: a method writes through a
    // capture and the variable it captured sees it.
    let mut bump = Chunk::new();
    bump.name = Some("bump".to_string());
    bump.arity = 0;
    bump.upvalues.push(UpvalDesc {
        in_parent_locals: true,
        index: 0,
    });
    bump.push(Instr::GetUpval { dst: 0, idx: 0 });
    bump.push(Instr::LoadConst { dst: 1, k: 0 });
    bump.push(Instr::Add { dst: 0, a: 0, b: 1 });
    bump.push(Instr::SetUpval { idx: 0, src: 0 });
    bump.push(Instr::Ret { src: 0 });
    bump.registers = 2;
    bump.constants.push(Value::int(1));

    let mut chunk = Chunk::new();
    chunk.protos.push(Rc::new(bump));
    let zero = chunk.add_constant(Value::int(0));
    chunk.push(Instr::LoadConst { dst: 0, k: zero });
    chunk.push(Instr::Closure { dst: 1, proto: 0 });
    chunk.push(Instr::Call { dst: 2, callee: 1, argc: 0 });
    chunk.push(Instr::Move { dst: 2, src: 1 });
    chunk.push(Instr::Call { dst: 2, callee: 2, argc: 0 });
    // The variable, not the return value: the write has to have landed in r0.
    chunk.push(Instr::Halt { src: 0 });
    chunk.registers = 4;

    let value = xenith::vm::run::execute(Rc::new(chunk)).expect("should run");
    assert!(matches!(value, Value::Int(2)), "got {:?}", value);
}

/// Compiles and runs a source string, or fails the test saying why not.
fn vm(source: &str) -> Value {
    let mut lexer = xenith::lexer::Lexer::new("<test>".to_string(), source.to_string());
    let tokens = lexer.make_tokens().expect("should lex");
    let mut parser = xenith::parser::Parser::new(tokens);
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
fn a_capture_in_a_when_inside_a_loop_keeps_that_passs_value() {
    // A `when` body is a fresh `Context` per evaluation, so this closure
    // keeps the 5 it was made with rather than following the register into
    // later passes. Verified against the tree walker, which answers 5.
    let value = vm(
        "let g: method() -> int = method() -> int => -1\n\
         let i: int = 0\n\
         while i < 2 {\n\
             when i == 0 { let x: int = 5 g = method() -> int => x }\n\
             i = i + 1\n\
         }\n\
         g()\n",
    );
    assert!(matches!(value, Value::Int(5)), "got {:?}", value);
}

#[test]
fn a_capture_of_a_for_induction_variable_closes_to_its_last_value() {
    // `visit_for_classic` keeps the induction variable in one `loop_ctx` for
    // the whole loop, so the closure sees the value the step left behind
    // after the condition failed. The tree walker answers 3.
    let value = vm(
        "let g: method() -> int = method() -> int => -1\n\
         for (let i: int = 0; i < 3; i = i + 1) {\n\
             g = method() -> int => i\n\
         }\n\
         g()\n",
    );
    assert!(matches!(value, Value::Int(3)), "got {:?}", value);
}

#[test]
fn a_method_passed_as_an_argument_keeps_its_capture() {
    let value = vm(
        "let n: int = 7\n\
         let get: method() -> int = method() -> int => n\n\
         method run(f: method() -> int) -> int { release f() }\n\
         run(get)\n",
    );
    assert!(matches!(value, Value::Int(7)), "got {:?}", value);
}
