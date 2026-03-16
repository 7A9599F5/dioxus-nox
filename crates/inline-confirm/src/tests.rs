//! Tests for dioxus-nox-inline-confirm.

use crate::types::ConfirmState;

#[test]
fn confirm_state_default_is_idle() {
    assert_eq!(ConfirmState::default(), ConfirmState::Idle);
}

#[test]
fn confirm_state_equality() {
    assert_eq!(ConfirmState::Idle, ConfirmState::Idle);
    assert_eq!(ConfirmState::Confirming, ConfirmState::Confirming);
    assert_ne!(ConfirmState::Idle, ConfirmState::Confirming);
}

#[test]
fn confirm_state_debug() {
    assert_eq!(format!("{:?}", ConfirmState::Idle), "Idle");
    assert_eq!(format!("{:?}", ConfirmState::Confirming), "Confirming");
}

#[test]
fn confirm_state_copy() {
    let state = ConfirmState::Confirming;
    let copied = state;
    assert_eq!(state, copied);
}
