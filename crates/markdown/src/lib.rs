//! # Security
//!
//! Raw HTML in markdown is a potential XSS vector. This crate provides three
//! rendering policies via [`HtmlRenderPolicy`](types::HtmlRenderPolicy):
//!
//! - **`Escape`** (default) — HTML tags are rendered as visible text. Safe for
//!   all inputs including untrusted user content.
//! - **`Sanitized`** — HTML is cleaned with the [`ammonia`](https://docs.rs/ammonia)
//!   crate, stripping `<script>`, `<iframe>`, event handlers, etc. while keeping
//!   safe formatting tags. Requires the `sanitize` Cargo feature.
//! - **`Trusted`** — HTML is injected directly into the DOM with **no sanitization**.
//!   Only use when you fully control the markdown source. **This is an XSS vector
//!   if used with user-generated content.**
//!
//! When in doubt, use the default `Escape` policy. If you need HTML rendering
//! with user content, enable the `sanitize` feature and use `Sanitized`.

pub mod components;
pub mod context;
pub mod highlight;
pub mod hooks;
pub mod ime_proxy;
pub mod inline_editor;
pub mod inline_tokens;
pub mod interop;
pub mod parser;
pub mod reveal_engine;
pub mod types;
pub mod viewport;

#[cfg(test)]
mod tests;

/// Prelude — import everything for typical consumer usage.
pub mod prelude {
    pub use crate::components::*;
    pub use crate::context::{
        CursorContext, MarkdownContext, MarkdownHandle, use_cursor_context, use_markdown_context,
        use_markdown_handle,
    };
    pub use crate::highlight::{
        HighlightResult, generate_theme_css, highlight_code, supported_languages,
        wrap_with_line_numbers,
    };
    pub use crate::hooks::{use_debounced_parse, use_heading_index, use_viewport_height};
    pub use crate::ime_proxy::ImeProxy;
    pub use crate::inline_editor::InlineEditor;
    pub use crate::parser::{index_to_line_col, parse_document, parse_document_with_policy};
    pub use crate::types::{
        ActiveBlockInputEvent, BlockEntry, CursorPosition, HeadingEntry, HtmlRenderPolicy, Layout,
        LivePreviewVariant, Mode, Orientation, ParseOptions, ParseState, ParsedDoc, Selection,
        SourceMap, SourceMapEntry, VimAction, VimMode, VimState,
    };
    pub use crate::viewport::EditorViewport;
}

/// Compound component namespace — `use dioxus_nox_markdown::markdown;` then `markdown::Root { ... }`.
pub mod markdown {
    pub use crate::components::{
        Content, Divider, Editor, ModeBar, ModeTab, Preview, Root, Toolbar, ToolbarButton,
        ToolbarSeparator,
    };
    pub use crate::ime_proxy::ImeProxy;
    pub use crate::inline_editor::InlineEditor;
    pub use crate::viewport::EditorViewport;
}
