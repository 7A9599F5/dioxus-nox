# dioxus-nox-markdown — Headless markdown editor/previewer/display

> See workspace `CLAUDE.md` for Dioxus 0.7 gotchas, Radix conventions, web_sys policy.

## Purpose
Three switchable modes (Read / Source / LivePreview) in a single compound component. Split-context design prevents cursor-movement re-renders on the preview pane. Parser: comrak 0.50 (full GFM AST), 300ms debounced, ~2ms full re-parse.

## Module Structure
- `lib.rs` — module re-exports + prelude
- `context.rs` — `MarkdownContext`, `CursorContext`, `use_markdown_context`
- `types.rs` — `Mode` enum, `CursorPosition`, `Selection`, `ParsedDoc`, `HeadingEntry`
- `parser.rs` — `parse_document()`, `use_debounced_parse`, `render_ast()`
- `components.rs` — `Root`, `Editor`, `Preview`, `Content`, `Toolbar`, `ToolbarButton`, `ToolbarSeparator`, `ModeBar`, `ModeTab`, `Divider`
- `hooks.rs` — `use_heading_index`, `use_viewport_height`
- `tests.rs` — unit tests (pure logic only)

## Key Design Decisions
1. Two separate contexts (`MarkdownContext` + `CursorContext`) — cursor signal updates never re-render `Preview`
2. Uncontrolled textarea for editor: `Rc<RefCell<String>>` hot-path → debounced 300ms → `comrak::parse_document()`
3. Library boundary: `use_heading_index()` is in-scope; full-text search and TOC are application-layer

## Further Reading
Detailed context in `.context/` — read on demand:
- `architecture.md` — split-context design, compound component parts, data attributes
- `editor.md` — uncontrolled textarea pattern, IME handling, oninput vs onchange, eval() timing
- `parser.md` — comrak 0.50 integration, 300ms debounce, AST rendering, parse states

## CI
```bash
cargo check -p dioxus-nox-markdown
cargo test -p dioxus-nox-markdown
cargo clippy -p dioxus-nox-markdown --target wasm32-unknown-unknown -- -D warnings
cargo check -p dioxus-nox-markdown --features desktop --no-default-features
cargo check -p dioxus-nox-markdown --features mobile --no-default-features
```
