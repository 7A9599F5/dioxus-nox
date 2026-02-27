# dioxus-nox-tag-input — Headless tag/multi-select input

> See workspace `CLAUDE.md` for Dioxus 0.7 gotchas, Radix conventions, web_sys policy.

## Purpose
Headless tag/multi-select input. Hook-first design: `use_tag_input` / `use_tag_input_grouped` returns `TagInputState<T>` (47 signals/memos, `Copy`). No built-in renders — consumers wire signals into their own RSX. Radix-style compound components also provided.

## Module Structure
- `tag.rs` — `TagLike` trait, default `Tag` struct
- `hook.rs` — `TagInputState<T>` (47 fields), `use_tag_input`, `use_tag_input_with`, `use_tag_input_grouped`
- `components/` — Radix-style compound components (`Root`, `Control`, `Input`, `Tag`, `TagRemove`, `TagPopover`, `TagList`, `Dropdown`, `DropdownGroup`, `Option`, `AutoComplete`, `Count`, `FormValue`, `LiveRegion`)
- `breakpoint.rs` — `use_breakpoint()`, resize listener on WASM, falls back to `Desktop` on native

## Key Design Decisions
1. `TagInputState<T>` is `Copy` (all fields are `Signal<T>` / `Memo<T>`) but bindings must be `let mut` to call setters
2. Popover and dropdown are mutually exclusive — closing one doesn't affect the other
3. Readonly mode: only ArrowLeft and Escape work; all mutations blocked

## Further Reading
Detailed context in `.context/` — read on demand:
- `state.md` — all 47 TagInputState signals/memos documented
- `keyboard.md` — 3-layer keyboard model (Input mode / Pill mode / Escape behavior)
- `architecture.md` — hook-first design rationale, TagLike trait, compound component wiring

## CI
```bash
cargo check -p dioxus-nox-tag-input
cargo test -p dioxus-nox-tag-input
cargo clippy -p dioxus-nox-tag-input --target wasm32-unknown-unknown -- -D warnings
```
