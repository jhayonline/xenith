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
    assert_eq!(size_of::<Value>(), 32, "Value");
}

#[test]
fn payloads_stay_small() {
    assert_eq!(size_of::<Number>(), 16, "Number");
    // Boxed. `Value::Bytes` holds an `Rc<Bytes>`, so what the enum sees is a
    // pointer; the struct itself is still a Vec.
    assert_eq!(size_of::<std::rc::Rc<Bytes>>(), 8, "the Bytes payload");
    assert_eq!(size_of::<BuiltInFunction>(), 24, "BuiltInFunction");
    assert_eq!(size_of::<Vec<Value>>(), 24, "the Tuple payload");
}

#[test]
fn runtime_result_stays_small() {
    // Returned by value from every `visit`, so its size is multiplied by every
    // node evaluated.
    assert_eq!(size_of::<xenith::runtime_result::RuntimeResult>(), 80);
}
