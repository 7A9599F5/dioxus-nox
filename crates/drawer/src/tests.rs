//! Tests for dioxus-nox-drawer.

use crate::types::DrawerSide;

#[test]
fn drawer_side_default_is_right() {
    assert_eq!(DrawerSide::default(), DrawerSide::Right);
}

#[test]
fn drawer_side_as_str() {
    assert_eq!(DrawerSide::Left.as_str(), "left");
    assert_eq!(DrawerSide::Right.as_str(), "right");
    assert_eq!(DrawerSide::Bottom.as_str(), "bottom");
    assert_eq!(DrawerSide::Top.as_str(), "top");
}

#[test]
fn drawer_side_equality() {
    assert_eq!(DrawerSide::Left, DrawerSide::Left);
    assert_ne!(DrawerSide::Left, DrawerSide::Right);
}

#[test]
fn drawer_side_debug() {
    let s = format!("{:?}", DrawerSide::Bottom);
    assert_eq!(s, "Bottom");
}
