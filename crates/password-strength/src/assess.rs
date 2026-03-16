//! Pure-function password strength assessment.
//!
//! No Dioxus dependency — usable in CLI tools, server-side, etc.

use crate::types::{StrengthCheck, StrengthLevel, StrengthResult};

/// Type alias for a password check function.
pub type CheckFn = Box<dyn Fn(&str) -> StrengthCheck>;

/// Assess password strength using custom check functions.
///
/// Score = min(number of passing checks, 4).
///
/// # Parameters
///
/// - `password`: The password to assess.
/// - `checks`: Custom check functions. Each returns a [`StrengthCheck`].
pub fn assess_password_strength(password: &str, checks: &[CheckFn]) -> StrengthResult {
    if password.is_empty() {
        return StrengthResult {
            level: StrengthLevel::None,
            score: 0,
            label: "None",
            checks: checks.iter().map(|check| check(password)).collect(),
        };
    }

    let results: Vec<StrengthCheck> = checks.iter().map(|check| check(password)).collect();
    let passed = results.iter().filter(|c| c.passed).count() as u8;
    let score = passed.min(4);
    let level = StrengthLevel::from_score(score);

    StrengthResult {
        level,
        score,
        label: level.label(),
        checks: results,
    }
}

/// Assess password strength using the default checks.
///
/// Convenience wrapper around [`assess_password_strength`] with [`default_checks`].
pub fn assess_password_strength_default(password: &str) -> StrengthResult {
    let checks = default_checks();
    assess_password_strength(password, &checks)
}

/// Default password strength checks:
///
/// 1. At least 8 characters
/// 2. At least 12 characters
/// 3. Contains an uppercase letter
/// 4. Contains a number
/// 5. Contains a special character
pub fn default_checks() -> Vec<CheckFn> {
    vec![
        Box::new(|pw: &str| StrengthCheck {
            label: "At least 8 characters",
            passed: pw.len() >= 8,
        }),
        Box::new(|pw: &str| StrengthCheck {
            label: "At least 12 characters",
            passed: pw.len() >= 12,
        }),
        Box::new(|pw: &str| StrengthCheck {
            label: "Contains an uppercase letter",
            passed: pw.chars().any(|c| c.is_uppercase()),
        }),
        Box::new(|pw: &str| StrengthCheck {
            label: "Contains a number",
            passed: pw.chars().any(|c| c.is_ascii_digit()),
        }),
        Box::new(|pw: &str| StrengthCheck {
            label: "Contains a special character",
            passed: pw.chars().any(|c| !c.is_alphanumeric()),
        }),
    ]
}
