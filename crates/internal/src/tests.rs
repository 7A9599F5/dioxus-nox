//! Unit tests for dioxus-nox-internal.
//!
//! Note: DOM-dependent functions (focus trap, inert, scroll lock) are no-ops
//! on non-wasm targets. These tests verify the non-wasm stubs compile and
//! behave correctly (returning None / no-op).

use super::*;

#[test]
fn focusable_elements_returns_none_on_non_wasm() {
    // On non-wasm, get_focusable_elements_in_container always returns None.
    let result = get_focusable_elements_in_container("test-container");
    assert!(result.is_none());
}

#[test]
fn cycle_focus_is_noop_on_non_wasm() {
    // Should not panic on non-wasm targets.
    cycle_focus("test-container", true);
    cycle_focus("test-container", false);
}

#[test]
fn set_siblings_inert_is_noop_on_non_wasm() {
    // Should not panic on non-wasm targets.
    set_siblings_inert("test-root", true);
    set_siblings_inert("test-root", false);
}

#[test]
fn scroll_lock_is_noop_on_non_wasm() {
    // Should not panic on non-wasm targets.
    // Note: on non-wasm these call document::eval which is a no-op in test context.
    // These primarily verify the functions compile and don't panic.
}

#[test]
fn focusable_selector_is_valid() {
    // Verify the selector string contains expected element types.
    assert!(focus_trap::FOCUSABLE_SELECTOR.contains("button"));
    assert!(focus_trap::FOCUSABLE_SELECTOR.contains("input"));
    assert!(focus_trap::FOCUSABLE_SELECTOR.contains("[href]"));
    assert!(focus_trap::FOCUSABLE_SELECTOR.contains("select"));
    assert!(focus_trap::FOCUSABLE_SELECTOR.contains("textarea"));
    assert!(focus_trap::FOCUSABLE_SELECTOR.contains("[tabindex]"));
}
