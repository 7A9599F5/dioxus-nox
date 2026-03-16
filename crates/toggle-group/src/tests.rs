//! Tests for dioxus-nox-toggle-group.

use crate::types::Orientation;

#[test]
fn orientation_default_is_horizontal() {
    assert_eq!(Orientation::default(), Orientation::Horizontal);
}

#[test]
fn orientation_as_str() {
    assert_eq!(Orientation::Horizontal.as_str(), "horizontal");
    assert_eq!(Orientation::Vertical.as_str(), "vertical");
}

#[test]
fn orientation_equality() {
    assert_eq!(Orientation::Horizontal, Orientation::Horizontal);
    assert_ne!(Orientation::Horizontal, Orientation::Vertical);
}
