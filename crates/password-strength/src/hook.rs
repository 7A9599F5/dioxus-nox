//! Dioxus hooks for reactive password strength assessment.

use std::sync::Arc;

use dioxus::prelude::*;

use crate::assess::{assess_password_strength, assess_password_strength_default};
use crate::types::{StrengthCheck, StrengthResult};

/// Reactive password strength hook using default checks.
///
/// Returns a memo signal that updates whenever the password signal changes.
pub fn use_password_strength(password: Signal<String>) -> Memo<StrengthResult> {
    use_memo(move || assess_password_strength_default(&password.read()))
}

/// Type alias for a shareable password check function.
pub type SharedCheckFn = Arc<dyn Fn(&str) -> StrengthCheck + Send + Sync>;

/// Reactive password strength hook with custom checks.
///
/// # Parameters
///
/// - `password`: Reactive password signal.
/// - `checks`: Custom check functions wrapped in Arc for shareability.
pub fn use_password_strength_with(
    password: Signal<String>,
    checks: Vec<SharedCheckFn>,
) -> Memo<StrengthResult> {
    let stored_checks = use_signal(|| checks);
    use_memo(move || {
        let checks_ref = stored_checks.read();
        let boxed: Vec<crate::assess::CheckFn> = checks_ref
            .iter()
            .map(|f| {
                let f = Arc::clone(f);
                Box::new(move |pw: &str| f(pw)) as crate::assess::CheckFn
            })
            .collect();
        assess_password_strength(&password.read(), &boxed)
    })
}
