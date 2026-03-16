//! Core types for password strength assessment.
//!
//! These types have **no Dioxus dependency** and can be used anywhere.

/// Strength level (0-4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StrengthLevel {
    /// No password entered.
    None = 0,
    /// Weak (score 1).
    Weak = 1,
    /// Fair (score 2).
    Fair = 2,
    /// Good (score 3).
    Good = 3,
    /// Strong (score 4).
    Strong = 4,
}

impl StrengthLevel {
    /// Convert a numeric score (0-4) to a strength level.
    pub fn from_score(score: u8) -> Self {
        match score {
            0 => Self::None,
            1 => Self::Weak,
            2 => Self::Fair,
            3 => Self::Good,
            _ => Self::Strong,
        }
    }

    /// Human-readable label for this strength level.
    pub fn label(&self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Weak => "Weak",
            Self::Fair => "Fair",
            Self::Good => "Good",
            Self::Strong => "Strong",
        }
    }
}

impl Default for StrengthLevel {
    fn default() -> Self {
        Self::None
    }
}

/// Individual check result.
#[derive(Debug, Clone, PartialEq)]
pub struct StrengthCheck {
    /// Human-readable label (e.g., "At least 8 characters").
    pub label: &'static str,
    /// Whether this check passes.
    pub passed: bool,
}

/// Full strength assessment result.
#[derive(Debug, Clone, PartialEq)]
pub struct StrengthResult {
    /// Overall strength level.
    pub level: StrengthLevel,
    /// Numeric score (0-4).
    pub score: u8,
    /// Descriptive label ("None", "Weak", "Fair", "Good", "Strong").
    pub label: &'static str,
    /// Individual check results.
    pub checks: Vec<StrengthCheck>,
}
