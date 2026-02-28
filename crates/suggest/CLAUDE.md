# dioxus-nox-suggest — Headless inline-trigger suggestion primitive

> See workspace `CLAUDE.md` for Dioxus 0.7 gotchas, Radix conventions, web_sys policy.

## Purpose
Headless inline-trigger suggestion primitive for Dioxus 0.7. Covers the "type a
special char, pick from a floating list" pattern: slash commands (`/`), @mentions,
`#`hashtags, and any custom trigger char. Standalone — zero dependency on cmdk or
markdown. `dioxus = "=0.7.3"` only.

## Module Structure
- `lib.rs` — re-exports; public API surface
- `types.rs` — `TriggerConfig`, `TriggerSelectEvent`, `TriggerContext` (context + impl)
- `hook.rs` — `use_suggestion()` → `SuggestionHandle`
- `trigger.rs` — `detect_trigger()`, `extract_filter()`, `utf16_to_byte_index()` (pure fns)
- `components.rs` — `Root`, `Trigger`, `List`, `Item`, `Group`, `Empty`
- `placement.rs` — `compute_float_style()` (pure, FUNCTIONAL inline style only)
- `tests.rs` — unit tests (trigger detection, filter extraction, placement math)

## Key Design Decisions
1. Single `Root` supports multiple trigger chars; `on_select` dispatches via `TriggerSelectEvent::trigger_char`
2. `Trigger` wraps any `<input>` / `<textarea>` — captures bubbled `oninput` + `onkeydown`
3. Cursor position via `document::eval("dioxus.send(document.activeElement?.selectionStart ?? 0)")` — wasm32 only; non-WASM stays inactive (v0.1 acceptable)
4. `Item` self-registers on mount / unregisters on drop — `highlighted_index` indexes into the ordered items Vec
5. `compute_float_style`: no auto-flip in v0.1 — always opens below anchor

## Public API

### Types
- `TriggerConfig { char, line_start_only, max_filter_len, allow_spaces }` + convenience ctors `::slash()`, `::mention()`, `::hashtag()`
- `TriggerSelectEvent { trigger_char, value, filter, trigger_offset }` — replace range: `text[trigger_offset..trigger_offset + filter.len() + trigger_char.len_utf8()]`
- `TriggerContext` — `Copy` context (provided by `Root`, accessed via `use_suggestion()`)
- `SuggestionHandle` — `active_char()`, `filter()`, `trigger_offset()`, `is_open()`, `close()`

### Pure functions (also public)
- `detect_trigger(text, cursor_utf16, trigger_char, line_start_only) -> Option<usize>`
- `extract_filter(text, cursor_utf16, trigger_char, line_start_only, allow_spaces, max_filter_len) -> Option<String>`
- `compute_float_style(anchor_left, anchor_bottom, anchor_width, side_offset, viewport_height) -> String`

### Data Attributes
| Attribute | Element | Values |
|---|---|---|
| `data-state` | `List` | `"open"` / `"closed"` |
| `data-trigger` | `List` + `Trigger` wrapper | Active char e.g. `"/"`, `"@"` |
| `data-highlighted` | `Item` | Present on keyboard-focused item |
| `data-slot="trigger-input"` | `Trigger` wrapper | Always present |
| `data-slot="trigger-list"` | `List` | Always present |

## Composing with dioxus-nox-cmdk (consumer example, docs only)
```rust
// No dependency on cmdk in this crate — pure consumer wiring
let sg = use_suggestion();
use_effect(move || { cmd_ctx.search.set(sg.filter()); });
```

## v0.1 Limitations
- `highlighted_index` uses mount-order Vec indexing; items with duplicate values behave unexpectedly
- No auto-flip for `List` placement (always opens below)
- Cursor detection inactive on non-WASM (no JS eval available)

## CI
```bash
cargo check -p dioxus-nox-suggest
cargo check -p dioxus-nox-suggest --target wasm32-unknown-unknown
cargo test -p dioxus-nox-suggest
cargo clippy -p dioxus-nox-suggest --target wasm32-unknown-unknown -- -D warnings
cargo check -p dioxus-nox-suggest --features desktop --no-default-features
cargo check -p dioxus-nox-suggest --features mobile --no-default-features
```
