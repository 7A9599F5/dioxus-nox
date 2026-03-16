//! Tests for dioxus-nox-password-strength.

use crate::assess::{assess_password_strength_default, default_checks};
use crate::types::StrengthLevel;

#[test]
fn empty_password_is_none() {
    let result = assess_password_strength_default("");
    assert_eq!(result.level, StrengthLevel::None);
    assert_eq!(result.score, 0);
    assert_eq!(result.label, "None");
}

#[test]
fn short_weak_password() {
    let result = assess_password_strength_default("abc");
    assert_eq!(result.level, StrengthLevel::None);
    assert_eq!(result.score, 0);
}

#[test]
fn eight_chars_lowercase() {
    let result = assess_password_strength_default("abcdefgh");
    assert_eq!(result.score, 1); // Only passes length >= 8
    assert_eq!(result.level, StrengthLevel::Weak);
}

#[test]
fn twelve_chars_lowercase() {
    let result = assess_password_strength_default("abcdefghijkl");
    assert_eq!(result.score, 2); // length >= 8 + length >= 12
    assert_eq!(result.level, StrengthLevel::Fair);
}

#[test]
fn strong_password() {
    let result = assess_password_strength_default("Abcdefghijkl1!");
    // length >= 8 ✓, length >= 12 ✓, uppercase ✓, number ✓, special ✓ → 5, clamped to 4
    assert_eq!(result.score, 4);
    assert_eq!(result.level, StrengthLevel::Strong);
    assert_eq!(result.label, "Strong");
}

#[test]
fn good_password() {
    let result = assess_password_strength_default("Abcdefg1!");
    // length >= 8 ✓, length >= 12 ✗, uppercase ✓, number ✓, special ✓ → 4, clamped to 4
    assert_eq!(result.score, 4);
    assert_eq!(result.level, StrengthLevel::Strong);
}

#[test]
fn medium_password() {
    let result = assess_password_strength_default("Abcdefgh");
    // length >= 8 ✓, length >= 12 ✗, uppercase ✓, number ✗, special ✗ → 2
    assert_eq!(result.score, 2);
    assert_eq!(result.level, StrengthLevel::Fair);
}

#[test]
fn checks_are_individually_correct() {
    let result = assess_password_strength_default("Abcdefgh1!");
    assert_eq!(result.checks.len(), 5);

    // length >= 8: true
    assert!(result.checks[0].passed);
    // length >= 12: false
    assert!(!result.checks[1].passed);
    // uppercase: true
    assert!(result.checks[2].passed);
    // number: true
    assert!(result.checks[3].passed);
    // special: true
    assert!(result.checks[4].passed);
}

#[test]
fn default_checks_count() {
    let checks = default_checks();
    assert_eq!(checks.len(), 5);
}

#[test]
fn strength_level_ordering() {
    assert!(StrengthLevel::None < StrengthLevel::Weak);
    assert!(StrengthLevel::Weak < StrengthLevel::Fair);
    assert!(StrengthLevel::Fair < StrengthLevel::Good);
    assert!(StrengthLevel::Good < StrengthLevel::Strong);
}

#[test]
fn strength_level_from_score() {
    assert_eq!(StrengthLevel::from_score(0), StrengthLevel::None);
    assert_eq!(StrengthLevel::from_score(1), StrengthLevel::Weak);
    assert_eq!(StrengthLevel::from_score(2), StrengthLevel::Fair);
    assert_eq!(StrengthLevel::from_score(3), StrengthLevel::Good);
    assert_eq!(StrengthLevel::from_score(4), StrengthLevel::Strong);
    assert_eq!(StrengthLevel::from_score(5), StrengthLevel::Strong); // clamped
}

#[test]
fn strength_level_labels() {
    assert_eq!(StrengthLevel::None.label(), "None");
    assert_eq!(StrengthLevel::Weak.label(), "Weak");
    assert_eq!(StrengthLevel::Fair.label(), "Fair");
    assert_eq!(StrengthLevel::Good.label(), "Good");
    assert_eq!(StrengthLevel::Strong.label(), "Strong");
}
