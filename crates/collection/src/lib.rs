// AI-generated with human review — see repository root for full notice.

//! Shared collection primitives for scoring, navigation, and filtering.
//!
//! This crate is **pure Rust** with zero Dioxus dependency. It provides:
//!
//! - [`ListItem`] trait — implemented by select's `ItemEntry`, cmdk's `ItemRegistration`, etc.
//! - [`score_items`] — nucleo-based fuzzy scoring with optional config (hidden, force_mount, boost, strategy)
//! - Navigation functions — [`navigate`], [`navigate_by`], [`first`], [`last`], [`type_ahead`]
//! - Utility — [`visible_values`], [`visible_values_set`]

mod navigation;
mod scoring;
#[cfg(test)]
mod tests;
mod types;

pub use navigation::{first, last, navigate, navigate_by, type_ahead};
pub use scoring::{score_items, visible_values, visible_values_set};
pub use types::{CustomFilter, Direction, ListItem, ScoredItem, ScoringConfig, ScoringStrategy};
