# dioxus-nox-markdown — Headless markdown editor/previewer/display

> See workspace `CLAUDE.md` for Dioxus 0.7 gotchas, Radix conventions, web_sys policy.

## Purpose
Three switchable modes (Read / Source / LivePreview) in a single compound component. Split-context design prevents cursor-movement re-renders on the preview pane. Parser: pulldown-cmark 0.13 (GFM), 300ms debounced. Optional syntax highlighting via `syntax-highlighting` feature (syntect).

## Module Structure
- `lib.rs` — module re-exports + prelude
- `context.rs` — `MarkdownContext`, `CursorContext`, `use_markdown_context`
- `types.rs` — `Mode` enum, `CursorPosition`, `Selection`, `ParsedDoc`, `HeadingEntry`, `ActiveBlockInputEvent`
- `parser.rs` — `parse_document()`, `parse_document_full()`, `parse_document_full_with_config()`, `RenderConfig`, `render_ast_to_element()`
- `components.rs` — `Root`, `Editor`, `InlineEditor`, `Preview`, `Content`, `Toolbar`, `ToolbarButton`, `ToolbarSeparator`, `ModeBar`, `ModeTab`, `Divider`
- `highlight.rs` — `highlight_code()`, `wrap_with_line_numbers()`, `generate_theme_css()`, `supported_languages()` (dual cfg-gated: syntect when feature on, HTML-escape fallback when off)
- `viewport.rs` — `ViewportNode` component, virtual viewport rendering
- `hooks.rs` — `use_heading_index`, `use_viewport_height`, `sync_gutter_scroll`
- `tests.rs` — unit tests (pure logic only)

## Key Design Decisions
1. Two separate contexts (`MarkdownContext` + `CursorContext`) — cursor signal updates never re-render `Preview`
2. Uncontrolled textarea for editor: `Rc<RefCell<String>>` hot-path → debounced 300ms → `pulldown_cmark::Parser`
3. Library boundary: `use_heading_index()` is in-scope; full-text search and TOC are application-layer
4. Syntax highlighting: `highlight_code()` is a pure function (not a component); `dangerous_inner_html` on `<code>` for highlighted spans
5. `MarkdownContext` is `Copy` — all fields are signals. String data stored as `Signal<String>` (e.g., `highlight_class_prefix`)

## Code Block Display Features

Three opt-in/opt-out props on `Root` control code block rendering:

| Prop | Type | Default | Scope |
|---|---|---|---|
| `show_code_line_numbers` | `bool` | `false` | Rendered code blocks in Preview/Content/Viewport |
| `show_code_language` | `bool` | `true` | Language label on rendered fenced code blocks |
| `show_editor_line_numbers` | `bool` | `false` | Line number gutter on source editor textarea |

### Code Block Data Attributes

| Attribute | Element | Values |
|---|---|---|
| `data-md-line-numbers` | `<pre>` | Present when line numbers active |
| `data-line-number="N"` | `<span class="code-line">` | Line's number (1-based) |
| `data-md-line-gutter` | `<span>` in code block / `<div>` in editor | CSS targeting for gutter element |
| `data-md-code-header` | `<div>` inside `<pre>` | Language header container |
| `data-md-code-language` | `<span>` inside header | Language text element |
| `data-md-editor-gutter` | `<div>` | Editor line-number gutter container |
| `data-md-blank-line` | `<div>` (InlineEditor block wrapper) | Present on synthetic blank-line paragraphs (extra `\n\n\n`+ gaps) |

### Line Number Non-selectability
- Inline `style="user-select:none"` on gutter elements — FUNCTIONAL (copy-paste behavior)
- `aria-hidden="true"` on gutter spans — screen readers skip decorative numbers
- Consumer CSS targets `[data-md-line-gutter]` for visual styling

## Syntax Highlighting (`syntax-highlighting` feature)
- Engine: syntect v5.3 (`fancy-regex`, pure Rust, WASM-safe)
- `LazyLock<SyntaxSet>` — initialized once on first code block render
- `ClassedHTMLGenerator` emits `<span class="{prefix}...">` spans (consumer provides CSS)
- `highlight_class_prefix` prop on `Root` (default `"hl-"`) for namespace isolation
- `data-md-highlighted="true"` on `<pre>` when syntect matched the language
- When feature disabled: `highlight_code()` returns HTML-escaped plain text

## Further Reading
Detailed context in `.context/` — read on demand:
- `architecture.md` — split-context design, compound component parts, data attributes
- `editor.md` — uncontrolled textarea pattern, IME handling, oninput vs onchange, eval() timing
- `parser.md` — pulldown-cmark integration, 300ms debounce, AST rendering, parse states

## CI
```bash
cargo check -p dioxus-nox-markdown
cargo test -p dioxus-nox-markdown
cargo clippy -p dioxus-nox-markdown --target wasm32-unknown-unknown -- -D warnings
cargo check -p dioxus-nox-markdown --features desktop --no-default-features
cargo check -p dioxus-nox-markdown --features mobile --no-default-features

# With syntax highlighting
cargo check -p dioxus-nox-markdown --features syntax-highlighting
cargo test -p dioxus-nox-markdown --features syntax-highlighting
cargo clippy -p dioxus-nox-markdown --features syntax-highlighting --target wasm32-unknown-unknown -- -D warnings
```
