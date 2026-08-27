//! Calls, recursion and captures, end to end.
//!
//! The chunks here are built by hand rather than compiled. That keeps the
//! interpreter loop testable apart from the compiler, which is what you want
//! on the day a call returns the wrong number and you need to know which half
//! is wrong.

use std::rc::Rc;

use xenith::types::Type;
use xenith::values::Value;
use xenith::vm::chunk::{Chunk, Instr};

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
#[ignore = "needs upvalues, task 6"]
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
    let text = error.as_string();
    assert!(text.contains("XEN019"), "{}", text);
    assert!(text.contains("loop_forever"), "{}", text);
}
