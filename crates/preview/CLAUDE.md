# dioxus-nox-preview

Debounced preview hook and LRU cache for navigable Dioxus lists.
See workspace `CLAUDE.md` for shared conventions.

## Crate Purpose

Prevents preview flicker during rapid arrow-key navigation by debouncing active item ID,
and caches previously rendered preview content in an LRU cache.
Standalone — zero dependency on dioxus-cmdk.

## Public API Surface

- `use_debounced_active(active_id: ReadSignal<Option<String>>, debounce_ms: u32) -> ReadSignal<Option<String>>`
- `use_preview_cache(capacity: usize) -> PreviewCacheHandle`
- `PreviewCacheHandle::get(&self, id: &str) -> Option<Rc<dyn Fn() -> Element>>`
- `PreviewCacheHandle::insert(&self, id: impl Into<String>, render: Rc<dyn Fn() -> Element>)`
- `PreviewCacheHandle::invalidate(&self, id: &str)`
- `PreviewCacheHandle::clear(&self)`, `len(&self) -> usize`, `is_empty(&self) -> bool`
- `PreviewPosition` enum: `None | Right | Bottom`
- `PreviewPosition::as_data_attr(&self) -> Option<&'static str>`

## Module Structure

- `lib.rs` — re-exports only
- `position.rs` — PreviewPosition enum + as_data_attr()
- `cache.rs` — PreviewCache (VecDeque-based LRU), PreviewCacheHandle, use_preview_cache
- `debounce.rs` — use_debounced_active hook + task lifecycle
- `tests.rs` — pure unit tests (no Dioxus runtime required)

## Key Design Decisions

- **OQ-1 (LRU impl):** `VecDeque` (zero-dep). O(n) ops acceptable at ≤20 entries.
- **OQ-2 (native debounce):** Immediate fire on non-wasm; no tokio dependency.
- **OQ-3 (cache reactivity):** Non-reactive `Rc<RefCell<>>`. Cache reads driven by debounced signal.
- **OQ-4 (cache key):** `String` for v0.1.
- **OQ-5 (hook location):** `use_debounced_active` included in this crate.
- **OQ-6 (closure cache):** Cache stores `Rc<dyn Fn() -> Element>` (render closures), not
  `Element` directly, because `Element = Result<VNode, RenderError>` is not `Clone`.
  Callers pass a closure that re-renders the preview on demand.

## Data Attributes

- `data-preview-position="right"` / `"bottom"` on container
- `data-preview-loading="true"` during debounce window
- `data-preview="true"` on preview container

## Crate-Specific Conventions

- ZERO web-sys/js-sys calls. Debounce timer via `gloo-timers` (wasm32 only), immediate on native.
- `Rc<RefCell<PreviewCache>>` for hot-path state (non-reactive, matches DragState pattern)

## CI Commands

```bash
cargo test
cargo clippy -- -D warnings
cargo clippy --target wasm32-unknown-unknown -- -D warnings
```
