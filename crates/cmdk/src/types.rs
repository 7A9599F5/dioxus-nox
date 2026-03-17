use std::collections::VecDeque;
use std::rc::Rc;

use dioxus::prelude::{EventHandler, Signal};
use dioxus_nox_collection::ListItem;

use crate::shortcut::Hotkey;

/// Controls how `CommandPalette` resolves its rendering mode.
///
/// - `Auto` — detects mobile via media queries and renders `CommandSheet` or `CommandDialog`
/// - `Dialog` — always renders `CommandDialog`
/// - `Sheet` — always renders `CommandSheet`
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum PaletteMode {
    #[default]
    Auto,
    Dialog,
    Sheet,
}

/// Preferred placement of the floating `CommandList` relative to its anchor.
///
/// Auto-flips to the opposite side on wasm32 when the preferred side has less
/// available viewport space. On Desktop/Mobile, `preferred_side` is always used.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Side {
    /// List opens below the anchor (default).
    #[default]
    Bottom,
    /// List opens above the anchor.
    Top,
}

// ---------------------------------------------------------------------------
// P-015: Animation lifecycle state
// ---------------------------------------------------------------------------

/// Animation state for transition lifecycle management.
///
/// Used with the `on_mount` / `on_unmount` callbacks and `data-entering` /
/// `data-leaving` CSS attributes on `CommandItem`, `CommandDialog`, and
/// `CommandSheet`.
///
/// Note: `Leaving` / deferred unmount is prepared here but the actual DOM
/// retention is deferred to Wave 6 (too complex for this release).
#[derive(Clone, Debug, PartialEq)]
pub enum AnimationState {
    /// The element has just mounted and is in its enter animation.
    Entering,
    /// The element is fully visible (enter animation complete).
    Visible,
    /// The element is leaving (exit animation in progress).
    /// Note: deferred unmount (keeping element in DOM) is implemented in Wave 6.
    Leaving,
}

/// Registration data for a command item.
#[derive(Clone, Debug, PartialEq)]
pub struct ItemRegistration {
    pub id: String,
    pub label: String,
    pub keywords: Vec<String>,
    /// Pre-computed `keywords.join(" ")` — avoids allocation per search.
    pub keywords_cached: String,
    pub group_id: Option<String>,
    pub disabled: bool,
    pub force_mount: bool,
    /// Semantic value sent to `on_select`. Falls back to `id` when `None`.
    pub value: Option<String>,
    /// Keyboard shortcut that triggers this item when pressed while the palette is open.
    pub shortcut: Option<Hotkey>,
    /// Page this item belongs to. `None` means root (visible when no page is active).
    pub page_id: Option<String>,
    /// When `true`, the item is excluded from scoring entirely.
    /// Unlike `disabled` (which keeps the item visible but non-interactive),
    /// `hidden` removes the item from results. Default: `false`.
    pub hidden: bool,
    /// Additive score modifier. Applied after nucleo scoring.
    /// Positive values boost the item, negative values dampen.
    /// Only applied when the item has a non-None score (matched the query).
    /// Default: `0`.
    pub boost: i32,
    /// Mode this item belongs to. `None` means the item appears in all modes.
    /// When a mode is active, only items with matching `mode_id` or `None` are scored.
    pub mode_id: Option<String>,
    /// Item-level select callback. Takes precedence over root `on_select`.
    pub on_select: Option<ItemSelectCallback>,
}

impl ListItem for ItemRegistration {
    fn value(&self) -> &str {
        &self.id
    }
    fn label(&self) -> &str {
        &self.label
    }
    fn keywords(&self) -> &str {
        &self.keywords_cached
    }
    fn disabled(&self) -> bool {
        self.disabled
    }
    fn group_id(&self) -> Option<&str> {
        self.group_id.as_deref()
    }
}

/// Registration data for a command group.
#[derive(Clone, Debug, PartialEq)]
pub struct GroupRegistration {
    pub id: String,
    pub heading: Option<String>,
    /// When `true`, the group is always visible regardless of whether it has
    /// any matching items. Useful for groups that contain non-filterable content.
    pub force_mount: bool,
}

/// An item with its computed match score.
#[derive(Clone, Debug, PartialEq)]
pub struct ScoredItem {
    pub id: String,
    pub score: Option<u32>,
    /// Character positions in the label that matched the query.
    /// `None` when query is empty or item matched via keywords only.
    pub match_indices: Option<Vec<u32>>,
}

/// Group ID provided via context so child items can associate.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct GroupId(pub String);

/// Context type for threading item ID from `CommandItem` to `CommandHighlight`.
///
/// Provided by `CommandItem` via `use_context_provider` so that child
/// `CommandHighlight` components can fall back to the parent's ID when no
/// explicit `id` prop override is needed.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ItemId(pub String);

/// Page ID provided via context so child items can associate with a page.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PageId(pub String);

/// Registration data for a command page.
#[derive(Clone, Debug, PartialEq)]
pub struct PageRegistration {
    pub id: String,
    pub title: Option<String>,
}

/// Wrapper for a custom filter function. Accepts closures via `Rc<dyn Fn>`.
/// Always compares as not-equal to trigger re-renders.
#[derive(Clone)]
#[allow(clippy::type_complexity)]
pub struct CustomFilter(pub Rc<dyn Fn(&str, &str, &str) -> Option<u32>>);

impl CustomFilter {
    /// Convenience constructor that wraps a function or closure.
    pub fn new(f: impl Fn(&str, &str, &str) -> Option<u32> + 'static) -> Self {
        Self(Rc::new(f))
    }
}

impl PartialEq for CustomFilter {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl std::fmt::Debug for CustomFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CustomFilter(..)")
    }
}

/// Wrapper for an item-level select callback. Always compares as not-equal
/// to avoid the function pointer comparison warning from `#[component]`.
#[derive(Clone)]
pub struct ItemSelectCallback(pub EventHandler<String>);

impl PartialEq for ItemSelectCallback {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl std::fmt::Debug for ItemSelectCallback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ItemSelectCallback(..)")
    }
}

// ---------------------------------------------------------------------------
// Sheet types
// ---------------------------------------------------------------------------

/// Mutable drag state held in `Rc<RefCell<_>>` — mutated on every pointermove
/// without triggering Dioxus re-renders. Only the `is_dragging` Signal is
/// reactive so RSX can conditionally skip its own transform style.
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct DragState {
    /// Cached sheet element for imperative style updates during drag.
    #[cfg(target_arch = "wasm32")]
    pub sheet_element: Option<web_sys::HtmlElement>,
    /// Whether a drag gesture is in progress.
    pub is_dragging: bool,
    /// ID of the pointer that initiated the drag (multi-touch guard).
    pub pointer_id: i32,
    /// Y coordinate at pointerdown.
    pub start_y: f64,
    /// Current translate-Y applied imperatively.
    pub current_translate_y: f64,
    /// Translate-Y when the drag started (the snap position baseline).
    pub base_translate_y: f64,
    /// Sheet element height in px (cached on mount / resize).
    pub sheet_height: f64,
    /// Recent (delta_px, timestamp_ms) samples for velocity calculation.
    pub velocity_buffer: VecDeque<(f64, f64)>,
    /// Timestamp (ms) after which drag is re-allowed post-scroll.
    pub scroll_locked_until: f64,
}

impl Default for DragState {
    fn default() -> Self {
        Self {
            #[cfg(target_arch = "wasm32")]
            sheet_element: None,
            is_dragging: false,
            pointer_id: -1,
            start_y: 0.0,
            current_translate_y: 0.0,
            base_translate_y: 0.0,
            sheet_height: 0.0,
            velocity_buffer: VecDeque::with_capacity(8),
            scroll_locked_until: 0.0,
        }
    }
}

/// Pure snap-point math extracted for testability.
#[allow(dead_code)]
pub(crate) mod sheet_math {
    /// Given a set of snap point ratios (0.0–1.0) and the sheet height,
    /// compute the translate-Y offset for each snap point.
    /// A ratio of 1.0 means fully open (translate-Y = 0), 0.5 means half open, etc.
    pub fn snap_offsets(snap_points: &[f32], sheet_height: f64) -> Vec<f64> {
        snap_points
            .iter()
            .map(|&ratio| (1.0 - ratio as f64) * sheet_height)
            .collect()
    }

    /// Find the nearest snap point index by position only.
    pub fn nearest_snap_by_position(current_translate_y: f64, offsets: &[f64]) -> usize {
        offsets
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                let da = (current_translate_y - **a).abs();
                let db = (current_translate_y - **b).abs();
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    /// Find the target snap point index, considering velocity.
    /// If velocity is strong enough (> threshold px/ms), bias toward the
    /// snap point in the flick direction. Otherwise fall back to nearest.
    pub fn snap_with_velocity(
        current_translate_y: f64,
        velocity_px_per_ms: f64,
        offsets: &[f64],
        velocity_threshold: f64,
    ) -> usize {
        if offsets.is_empty() {
            return 0;
        }

        let nearest = nearest_snap_by_position(current_translate_y, offsets);

        if velocity_px_per_ms.abs() < velocity_threshold {
            return nearest;
        }

        // Positive velocity = dragging down = increasing translate-Y = closing
        if velocity_px_per_ms > 0.0 {
            // Find next higher offset (more closed) than current position
            offsets
                .iter()
                .enumerate()
                .filter(|&(_, off)| *off > current_translate_y)
                .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(i, _)| i)
                .unwrap_or(nearest)
        } else {
            // Dragging up = decreasing translate-Y = opening
            offsets
                .iter()
                .enumerate()
                .filter(|&(_, off)| *off < current_translate_y)
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(i, _)| i)
                .unwrap_or(nearest)
        }
    }

    /// Calculate whether the sheet should be dismissed based on the
    /// close threshold ratio.
    pub fn should_dismiss(
        current_translate_y: f64,
        sheet_height: f64,
        close_threshold: f32,
    ) -> bool {
        if sheet_height <= 0.0 {
            return false;
        }
        // How much of the sheet is hidden (ratio 0.0 = fully visible, 1.0 = fully hidden)
        let hidden_ratio = current_translate_y / sheet_height;
        hidden_ratio >= close_threshold as f64
    }

    /// Compute velocity from a buffer of (delta_px, timestamp_ms) samples.
    /// Returns px/ms.
    pub fn compute_velocity(samples: &[(f64, f64)]) -> f64 {
        if samples.len() < 2 {
            return 0.0;
        }
        let total_delta: f64 = samples.iter().map(|(d, _)| d).sum();
        let time_span = samples.last().unwrap().1 - samples.first().unwrap().1;
        if time_span <= 0.0 {
            return 0.0;
        }
        total_delta / time_span
    }
}

// ---------------------------------------------------------------------------
// Scoring strategy (re-exported from collection)
// ---------------------------------------------------------------------------

pub use dioxus_nox_collection::ScoringStrategy;

/// Wrapper for `Rc<dyn ScoringStrategy>` that implements `PartialEq` + `Clone`
/// for use as a Dioxus component prop. Always compares as not-equal to
/// trigger re-renders (same pattern as `CustomFilter`).
#[derive(Clone)]
pub struct ScoringStrategyProp(pub Rc<dyn ScoringStrategy>);

impl std::fmt::Debug for ScoringStrategyProp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ScoringStrategyProp(..)")
    }
}

impl PartialEq for ScoringStrategyProp {
    fn eq(&self, _other: &Self) -> bool {
        false // Always re-render when strategy changes
    }
}

// Debug for `dyn ScoringStrategy` is implemented in dioxus-nox-collection.

// ---------------------------------------------------------------------------
// Frecency scoring strategy
// ---------------------------------------------------------------------------

/// Helper that implements [`ScoringStrategy`] using a caller-provided frecency
/// weight lookup. The closure returns `Some(weight)` for known items (e.g.
/// 0.0–1.0 based on frequency + recency) or `None` to leave the score unchanged.
///
/// Final score: `raw_score * (1.0 + weight)`, clamped to `u32::MAX`.
///
/// # Example
/// ```rust,ignore
/// let frecency = FrecencyStrategy::new(|id: &str| store.get(id).map(|e| e.weight()));
/// CommandRoot {
///     scoring_strategy: ScoringStrategyProp(Rc::new(frecency)),
/// }
/// ```
pub struct FrecencyStrategy<F>
where
    F: Fn(&str) -> Option<f32> + 'static,
{
    lookup: F,
}

impl<F> FrecencyStrategy<F>
where
    F: Fn(&str) -> Option<f32> + 'static,
{
    /// Create a new [`FrecencyStrategy`] with the given weight lookup closure.
    pub fn new(lookup: F) -> Self {
        Self { lookup }
    }
}

impl<F> ScoringStrategy for FrecencyStrategy<F>
where
    F: Fn(&str) -> Option<f32> + 'static,
{
    fn adjust_score(&self, item_id: &str, raw_score: u32, _query: &str) -> Option<u32> {
        match (self.lookup)(item_id) {
            Some(weight) => {
                let adjusted = (raw_score as f64) * (1.0 + weight as f64);
                Some(adjusted.min(u32::MAX as f64) as u32)
            }
            None => Some(raw_score),
        }
    }
}

// ---------------------------------------------------------------------------
// Mode registration
// ---------------------------------------------------------------------------

/// Registration for a command palette mode triggered by a prefix character.
#[derive(Clone, Debug, PartialEq)]
pub struct ModeRegistration {
    /// Unique mode identifier (e.g., "commands", "exercises").
    pub id: String,
    /// Prefix that activates this mode (e.g., ">", "/", "#", "@").
    /// Must be a single character at the start of the search query.
    pub prefix: String,
    /// Display label for the mode indicator (e.g., "Commands", "Exercises").
    pub label: String,
    /// Placeholder text when this mode is active.
    pub placeholder: Option<String>,
}

// ---------------------------------------------------------------------------
// Global shortcuts
// ---------------------------------------------------------------------------

/// A registered global keyboard shortcut.
#[derive(Clone, Debug)]
pub struct GlobalShortcut {
    pub id: String,
    pub hotkey: Hotkey,
    pub handler: dioxus::prelude::EventHandler<()>,
}

/// A registered two-key chord shortcut (e.g., G then W).
#[derive(Clone, Debug)]
pub struct ChordShortcut {
    pub id: String,
    pub first: Hotkey,
    pub second: Hotkey,
    pub handler: dioxus::prelude::EventHandler<()>,
    pub timeout_ms: u32,
}

/// State for the chord detection state machine.
#[derive(Clone, Debug, Default)]
pub struct ChordState {
    /// The pending first key of a chord, and when it was pressed.
    pub pending: Option<(Hotkey, f64)>,
}

// ---------------------------------------------------------------------------
// P-038: Async commands
// ---------------------------------------------------------------------------

/// An item registered via [`use_async_commands`](crate::hook::use_async_commands).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AsyncItem {
    pub id: String,
    pub label: String,
    pub keywords: Option<String>,
    pub value: Option<String>,
    pub group: Option<String>,
    pub disabled: bool,
}

/// Handle returned by [`use_async_commands`](crate::hook::use_async_commands)
/// for monitoring loading state and errors.
#[derive(Clone, Copy, PartialEq)]
pub struct AsyncCommandHandle {
    pub is_loading: Signal<bool>,
    pub error: Signal<Option<String>>,
    pub refresh_counter: Signal<u32>,
}

// ---------------------------------------------------------------------------
// P-039: Action panel
// ---------------------------------------------------------------------------

/// State for the action panel overlay on an active item.
#[derive(Clone, Debug, PartialEq)]
pub struct ActionPanelState {
    pub item_id: String,
    pub active_idx: usize,
}

/// A single action registered within a [`CommandActionPanel`](crate::CommandActionPanel).
#[derive(Clone, Debug)]
pub struct ActionRegistration {
    pub id: String,
    pub label: String,
    pub disabled: bool,
    pub on_action: Option<EventHandler<String>>,
}

impl PartialEq for ActionRegistration {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.label == other.label && self.disabled == other.disabled
    }
}

// ---------------------------------------------------------------------------
// P-040: Inline Forms
// ---------------------------------------------------------------------------

/// A select option for [`FormFieldType::Select`] fields.
#[derive(Clone, Debug, PartialEq)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

/// The type and constraints of a [`CommandFormField`](crate::CommandFormField).
#[derive(Clone, Debug, Default, PartialEq)]
pub enum FormFieldType {
    /// Single-line text input.
    #[default]
    Text,
    /// Numeric input with optional min/max bounds.
    Number { min: Option<f64>, max: Option<f64> },
    /// Boolean checkbox field.
    Bool,
    /// Select / dropdown field with a fixed list of options.
    Select { options: Vec<SelectOption> },
}

/// The current value of a form field.
#[derive(Clone, Debug, PartialEq)]
pub enum FormValue {
    Text(String),
    Number(f64),
    Bool(bool),
    Select(String),
}

impl Default for FormValue {
    fn default() -> Self {
        FormValue::Text(String::new())
    }
}
