pub mod components;
pub mod context;
pub mod hooks;
pub mod parser;
pub mod types;

#[cfg(test)]
mod tests;

/// Prelude — import everything for typical consumer usage.
pub mod prelude {
    pub use crate::components::*;
    pub use crate::context::{
        CursorContext, MarkdownContext, MarkdownHandle, use_cursor_context, use_markdown_context,
        use_markdown_handle,
    };
    pub use crate::hooks::{use_debounced_parse, use_heading_index, use_viewport_height};
    pub use crate::parser::{index_to_line_col, parse_document};
    pub use crate::types::{
        CursorPosition, HeadingEntry, HtmlRenderPolicy, Layout, Mode, Orientation, ParseOptions,
        ParseState, ParsedDoc, Selection, SourceMap, SourceMapEntry, VimAction, VimMode, VimState,
    };
}

/// Compound component namespace — `use dioxus_nox_markdown::markdown;` then `markdown::Root { ... }`.
pub mod markdown {
    pub use crate::components::{
        Content, Divider, Editor, ModeBar, ModeTab, Preview, Root, Toolbar, ToolbarButton,
        ToolbarSeparator,
    };
}
