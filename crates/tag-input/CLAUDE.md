# dioxus-nox-tag-input

Headless tag/multi-select input library for Dioxus 0.7 (WASM/web).
See workspace `CLAUDE.md` for Dioxus 0.7 gotchas, Radix pattern, web_sys policy, and shared conventions.

## Architecture

Headless hook pattern: `use_tag_input` / `use_tag_input_grouped` returns `TagInputState<T>` containing reactive signals. No built-in renders — consumers wire signals into their own RSX. Styled with Tailwind CSS v4.

## Key Types

- **`TagLike` trait** (`tag.rs`): `id() -> &str`, `name() -> &str`, optional `group() -> Option<&str>`, `is_locked() -> bool`. Default `Tag` struct provided.
- **`TagInputState<T>`** (`hook.rs`): 47 signals/memos. `Copy` (all fields are signals). Manually implements `Clone`/`PartialEq`.
  - Core: `search_query`, `selected_tags`, `available_tags`, `is_dropdown_open`, `highlighted_index`, `active_pill`, `popover_pill`
  - Callbacks: `on_create`, `on_add`, `on_remove`, `on_duplicate`, `on_paste`, `on_edit`, `on_reorder`
  - Config: `is_disabled`, `is_readonly`, `allow_duplicates`, `enforce_allow_list`, `select_mode`, `max_tags`, `min_tags`, `max_suggestions`, `max_tag_length`, `max_visible_tags`, `validate`, `filter`, `sort_selected`, `async_suggestions`
  - State: `status_message`, `validation_error`, `editing_pill`
  - Memos: `filtered_suggestions`, `grouped_suggestions`, `is_at_limit`, `is_below_minimum`, `has_no_matches`, `overflow_count`, `visible_tags`, `auto_complete_suggestion`, `form_value`, `suggestion_count`
  - ARIA: `listbox_id()`, `suggestion_id(idx)`, `active_descendant()`, `aria_expanded()`, `pill_id(idx)`
  - Methods: `set_query`, `add_tag`, `remove_tag`, `handle_keydown`, `handle_paste`, `start_editing`, `commit_edit`, `cancel_edit`, `move_tag`, `clear_all`, `select_all`, `toggle_popover`, `close_dropdown`
- **`use_tag_input_with(config)`**: accepts `TagInputConfig<T>` with controlled mode signals (`value`, `query`, `open`)
- **`use_tag_input_grouped()`**: accepts `TagInputGroupConfig<T>`; uses `TagLike::group()` for labelled sections
- **`SuggestionGroup<T>`**: `label: String`, `items: Vec<T>`, `total_count: usize`
- **`find_match_ranges(text, query)`**: `Vec<(usize, usize)>` byte-offset ranges for match highlighting
- **`extract_clipboard_text(event)`**: WASM helper for paste handlers (returns `None` on non-WASM)
- **`use_breakpoint()`** (`breakpoint.rs`): resize listener on WASM, falls back to `Desktop` on native

## Compound Components (`src/components/`)

Radix-style. `Root` calls `use_tag_input`/`use_tag_input_grouped` and provides state via context. All parts accept `#[props(extends = GlobalAttributes)]` and emit `data-slot` attributes.

`Root`, `Control`, `Input`, `Tag`, `TagRemove`, `TagPopover`, `TagList`, `Dropdown`, `DropdownGroup`, `Option`, `AutoComplete`, `Count`, `FormValue`, `LiveRegion`

## Keyboard Model

**Input mode:**
- ArrowDown/Up → navigate suggestions (opens dropdown if closed)
- Enter → add highlighted suggestion, or call `on_create`, or open dropdown
- Tab → accept auto-complete suggestion
- ArrowLeft (empty query) → enter pill mode (last pill)
- Backspace (empty query) → enter pill mode (last pill, then delete)
- Escape → close dropdown
- Delimiter chars → commit current query (if `delimiters` set)

**Pill mode (a pill is focused):**
- ArrowLeft/Right → navigate pills; right past last returns to input
- Home/End → jump to first/last pill
- Enter → toggle popover (blocked in readonly)
- Backspace/Delete → layered: close popover → delete pill (blocked in readonly)
- Escape → layered: close popover → deselect → close dropdown
- Any typing key → exit pill mode, character enters input (blocked in readonly)

**Readonly mode:** only ArrowLeft (enter pill mode) and Escape work; all mutations blocked.
**Disabled mode:** all handling is no-op.

Popover and dropdown are mutually exclusive.

## Tailwind CSS v4 Setup

- `input.css` is the entry point (`@source` directives scan `.rs` files)
- Built CSS → `assets/tailwind.css` (loaded via `asset!("/assets/tailwind.css")`)
- Classes must be concrete string literals — dynamic interpolation won't be scanned
- `form.rs` pattern: `AccentTheme` structs with static class strings

## Crate-Specific Conventions

- `TagInputState` is `Copy` but bindings must be `let mut` to call setters
- Signals are `Copy` in Dioxus 0.7 — state structs hold `Signal<T>` and `Memo<T>` directly
- `EventHandler<T>` for callbacks (not closures in props)
- `asset!()` macro for static assets; `dioxus::document::Stylesheet` for CSS

## CI Commands

```bash
cargo check
cargo check --target wasm32-unknown-unknown
cargo test
```

## Sortable Example Note

`sortable.rs` uses `dioxus-nox-dnd` as a dev-dep. In the workspace, the path dep is resolved via workspace:
```toml
dioxus-nox-dnd = { workspace = true }
```
The `SortableItem` for locked tags uses `disabled: true`.
