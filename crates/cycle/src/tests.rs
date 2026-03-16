//! Tests for dioxus-nox-cycle.
//!
//! Note: Hook tests require a Dioxus runtime. These tests verify
//! the pure logic aspects and type constraints.

#[test]
fn cycle_state_is_clone() {
    // CycleState<T> must be Clone.
    fn assert_clone<T: Clone>() {}
    assert_clone::<crate::CycleState<String>>();
}

#[test]
fn cycle_state_type_constraints() {
    // Verify that CycleState works with common types.
    fn assert_usable<T: Clone + PartialEq + 'static>() {}
    assert_usable::<String>();
    assert_usable::<i32>();
    assert_usable::<bool>();
}
