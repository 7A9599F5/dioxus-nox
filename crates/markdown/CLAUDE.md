# dioxus-nox-markdown

Headless markdown editor, previewer, and display component library for Dioxus 0.7.
See workspace `CLAUDE.md` for Dioxus 0.7 gotchas, Radix pattern, web_sys policy, and shared conventions.

## Architecture

Three switchable modes in a single compound component:
- **Read** — semantic HTML rendering, read-only display
- **Source** — raw markdown textarea editor
- **LivePreview** — split pane: editor (left/top) + rendered preview (right/bottom)

### File Layout

```
src/
├── lib.rs        # Module re-exports + prelude
├── context.rs    # MarkdownContext, CursorContext, use_markdown_context
├── types.rs      # Mode enum, CursorPosition, Selection, ParsedDoc, HeadingEntry
├── parser.rs     # parse_document(), use_debounced_parse hook, render_ast()
├── components.rs # Root, Editor, Preview, Content, Toolbar, ToolbarButton,
│                 #   ToolbarSeparator, ModeBar, ModeTab, Divider
├── hooks.rs      # use_heading_index, use_viewport_height
└── tests.rs      # Unit tests (pure logic, no component rendering tests)
```

### Compound Component Parts

| Part | Usage |
|---|---|
| `markdown::Root` | Provides `MarkdownContext` + `CursorContext`; controlled/uncontrolled for mode and value |
| `markdown::Editor` | `div > textarea` (always mounted); `data-state="active/inactive"` |
| `markdown::Preview` | Rendered preview (always mounted); `data-state="active/inactive"` |
| `markdown::Content` | Read-mode display; may be conditionally rendered |
| `markdown::Toolbar` | Consumer-composed toolbar container |
| `markdown::ToolbarButton` | Individual toolbar button |
| `markdown::ToolbarSeparator` | Visual separator in toolbar |
| `markdown::ModeBar` | Mode tab strip container (`role="tablist"`) |
| `markdown::ModeTab` | Individual mode tab (`role="tab"`) |
| `markdown::Divider` | Visual split between editor and preview panes |

### Context Design

Two separate contexts to prevent cursor-movement re-renders on Preview:
- **`MarkdownContext`**: mode signal, value signal, `parsed_doc` memo, `trigger_parse` callback, disabled state
- **`CursorContext`**: `cursor_position` signal, `selection` signal — only `Editor` writes these

### Parser Integration

- **Parser**: `comrak 0.50` (`default-features = false`) — full GFM AST
- **Parse cycle**: uncontrolled `Rc<RefCell<String>>` hot-path → debounced 300ms → `comrak::parse_document()` → `ParsedDoc`
- **No incremental parsing in v1** — full re-parse < 2ms for typical docs
- **Heading index**: `use_heading_index()` returns `Vec<HeadingEntry>` from the AST
- **Front matter**: comrak `front_matter_delimiter` extension; exposed as raw `String`

## Data Attributes (`data-md-*` namespace)

| State | Attribute |
|---|---|
| Current mode | `data-md-mode="read\|source\|live-preview"` |
| Active/inactive pane | `data-state="active\|inactive"` |
| Dirty (unsaved changes) | `data-md-dirty="true\|false"` |
| Parse state | `data-md-parse-state="idle\|parsing\|done\|error"` |
| Disabled | `data-disabled` |
| Readonly | `data-md-readonly="true\|false"` |
| Layout orientation | `data-md-layout="horizontal\|vertical"` |

## Markdown Editor Specific Gotchas

- **Controlled textarea cursor reset**: Setting `value: "{signal}"` on textarea resets cursor to end on every re-render. ALWAYS use uncontrolled textarea. Store content in `Rc<RefCell<String>>` for hot-path.
- **oninput vs onchange**: Use `oninput` (fires per keystroke). `onchange` fires on blur only.
- **eval() must run after mount**: Never call `document::eval()` in component body. Only inside `use_effect`, event handlers, or `onmounted`.
- **prevent_default must be synchronous**: In `onkeydown`, call `evt.prevent_default()` synchronously — cannot `.await` before deciding.
- **async in event handlers**: Use `spawn(async move { ... })` inside event handlers for eval() calls.
- **IME composition**: Track composition state in `Rc<RefCell<bool>>`. Skip debounce during `oncompositionstart..oncompositionend` to avoid corrupting CJK input.
- **write_unchecked borrow panics**: Do not hold a `signal.read_unchecked()` guard while calling `signal.write_unchecked()`.
- **use_effect infinite loops**: If an effect reads and writes the same signal, it loops. Use `use_memo` for derived state.
- **Textarea programmatic set**: Use `document::eval()` to set `el.value = newText` directly, then dispatch a synthetic `input` event to sync Rust state.
- **comrak sourcepos**: `opts.parse.sourcepos` does NOT exist in comrak 0.50 — sourcepos is always tracked. Only `opts.render.sourcepos` exists (HTML output only, irrelevant here).

## Crate-Specific Conventions

- Non-reactive hot-path state: `Rc<RefCell<T>>` (raw editor content)
- Reactive derived state: `Memo<T>` (e.g., `parsed_doc`)
- Debounced parse trigger: `gloo_timers::callback::Timeout` (WebView targets)
- No syntax highlighting shipped — consumers add highlight.js/Prism.js
- Optional: `syntax-highlighting` feature flag gates syntect + two-face

## Search & Indexing Boundary Rule

Library provides markdown content and its AST. Search, indexing, and TOC are application-layer.

**In scope:** `use_heading_index()` returning `Vec<HeadingEntry>` (level, text, anchor, line).
**Out of scope:** full-text search, cross-document search, Find & Replace UI, TOC component.

## Adding a New Feature Checklist

- New component: `components.rs` → `lib.rs` re-export → `tests.rs`
- New hook: `hooks.rs` → `lib.rs` re-export → `tests.rs`
- New type: `types.rs` → `lib.rs` re-export (if public) → `tests.rs`
- New data attribute: update the data attributes table above → add to component's render

## CI Commands

```bash
cargo test -p dioxus-nox-markdown
cargo clippy -p dioxus-nox-markdown -- -D warnings
cargo clippy -p dioxus-nox-markdown --target wasm32-unknown-unknown -- -D warnings
cargo check -p dioxus-nox-markdown --features desktop --no-default-features
cargo check -p dioxus-nox-markdown --features mobile --no-default-features
```
