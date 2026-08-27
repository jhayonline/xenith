//! Size guards for the values on the interpreter's hot path.
//!
//! `docs/internals/10-performance.md` states the rule in prose: "before adding
//! a variant, check whether it is larger than the current biggest. If it is,
//! box it." This is that rule, enforced.
//!
//! A failure here is not necessarily a bug. It means the size changed, which is
//! a decision, so make it deliberately and update the number.

use std::mem::size_of;

use xenith::values::{BuiltInFunction, Bytes, Number, Value};

#[test]
fn value_stays_small() {
    // 16, down from 32. Boxing `Bytes` and `Tuple` and shrinking
    // `BuiltInFunction` left `Number` as the widest payload, and `Number`'s own
    // tag byte has 254 unused values -- a niche roomy enough for `Value`'s
    // discriminant, so the enum costs nothing beyond its largest member.
    assert_eq!(size_of::<Value>(), 16, "Value");
}

#[test]
fn payloads_stay_small() {
    assert_eq!(size_of::<Number>(), 16, "Number");
    // Boxed. `Value::Bytes` holds an `Rc<Bytes>`, so what the enum sees is a
    // pointer; the struct itself is still a Vec.
    assert_eq!(size_of::<std::rc::Rc<Bytes>>(), 8, "the Bytes payload");
    assert_eq!(
        size_of::<std::rc::Rc<Vec<Value>>>(),
        8,
        "the Tuple payload"
    );
}

#[test]
fn builtin_is_an_index() {
    assert_eq!(size_of::<BuiltInFunction>(), 2, "BuiltInFunction");
}

#[test]
fn runtime_result_stays_small() {
    // Returned by value from every `visit`, so its size is multiplied by every
    // node evaluated.
    assert_eq!(size_of::<xenith::runtime_result::RuntimeResult>(), 48);
}

#[test]
fn an_instruction_fits_in_a_word() {
    // The spec's ceiling. A wider instruction multiplies by every instruction
    // in every chunk, and the VM loop reads one per dispatch.
    assert!(
        size_of::<xenith::vm::chunk::Instr>() <= 8,
        "Instr is {} bytes, over the 8-byte ceiling",
        size_of::<xenith::vm::chunk::Instr>()
    );
}

#[test]
fn a_closure_is_a_pointer_in_a_value() {
    // `Value` is 16 bytes and stays 16 bytes. A closure is a proto plus a
    // vector of cells, which is 32 bytes of struct -- so what the enum holds
    // is a pointer to it, as `List`, `String` and `Bytes` already do.
    assert_eq!(
        size_of::<std::rc::Rc<xenith::vm::closure::Closure>>(),
        8,
        "the Closure payload"
    );
    assert_eq!(size_of::<Value>(), 16, "Value");
}
