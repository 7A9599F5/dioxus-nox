use comrak::nodes::{ListType, NodeValue};
use comrak::{Anchorizer, Arena, Options};
use dioxus::prelude::*;

use crate::types::{HeadingEntry, HtmlRenderPolicy, ParsedDoc};

/// Build comrak options with GFM extensions enabled.
pub(crate) fn build_comrak_options() -> Options<'static> {
    let mut opts = Options::default();
    opts.extension.strikethrough = true;
    opts.extension.table = true;
    opts.extension.autolink = true;
    opts.extension.tasklist = true;
    opts.extension.footnotes = true;
    opts.extension.front_matter_delimiter = Some("---".to_owned());
    opts
}

/// Parse a markdown string into a `ParsedDoc`.
///
/// Extracts headings, front matter, and renders the AST to a Dioxus Element
/// with `data-source-line` attributes on block-level elements for scroll sync.
pub fn parse_document(input: &str) -> ParsedDoc {
    let arena = Arena::new();
    let opts = build_comrak_options();
    let root = comrak::parse_document(&arena, input, &opts);

    let mut anchorizer = Anchorizer::new();
    let headings = extract_headings(root, &mut anchorizer);
    let front_matter = extract_front_matter(root);
    let element = render_ast_to_element(root);

    ParsedDoc {
        element,
        headings,
        front_matter,
    }
}

/// Walk the AST and collect all headings with their metadata.
fn extract_headings<'a>(
    root: &'a comrak::nodes::AstNode<'a>,
    anchorizer: &mut Anchorizer,
) -> Vec<HeadingEntry> {
    let mut headings = Vec::new();

    for node in root.descendants() {
        let data = node.data.borrow();
        if let NodeValue::Heading(heading) = &data.value {
            let text = collect_text(node);
            let anchor = anchorizer.anchorize(&text);
            // comrak sourcepos is 1-based; HeadingEntry.line is 0-based
            let line = data.sourcepos.start.line.saturating_sub(1);

            headings.push(HeadingEntry {
                level: heading.level,
                text,
                anchor,
                line,
            });
        }
    }

    headings
}

/// Recursively collect all text content from a node's children.
/// Handles Text, Code (inline), SoftBreak (→ space), and LineBreak (→ space).
fn collect_text<'a>(node: &'a comrak::nodes::AstNode<'a>) -> String {
    let mut text = String::new();
    collect_text_inner(node, &mut text);
    text
}

fn collect_text_inner<'a>(node: &'a comrak::nodes::AstNode<'a>, buf: &mut String) {
    for child in node.children() {
        let data = child.data.borrow();
        match &data.value {
            NodeValue::Text(t) => buf.push_str(t),
            NodeValue::Code(code) => buf.push_str(&code.literal),
            NodeValue::SoftBreak | NodeValue::LineBreak => buf.push(' '),
            // For other inline nodes (Emph, Strong, Link, etc.), recurse into children
            _ => {
                drop(data); // release borrow before recursing
                collect_text_inner(child, buf);
            }
        }
    }
}

/// Extract front matter from the AST root's direct children.
fn extract_front_matter<'a>(root: &'a comrak::nodes::AstNode<'a>) -> Option<String> {
    for child in root.children() {
        let data = child.data.borrow();
        if let NodeValue::FrontMatter(fm) = &data.value {
            // comrak includes the delimiters in the FrontMatter string.
            // Strip leading/trailing delimiter lines ("---\n" and "---\n").
            let trimmed = fm
                .strip_prefix("---\n")
                .unwrap_or(fm)
                .strip_suffix("---\n")
                .or_else(|| fm.strip_suffix("---"))
                .unwrap_or(fm);
            return Some(trimmed.to_string());
        }
    }
    None
}

/// Render a raw HTML fragment according to the given policy.
///
/// - `Escape`: render the raw HTML as visible text (safe default).
/// - `Trusted`: render via `dangerous_inner_html` (opt-in for trusted input).
fn render_html_fragment(raw: &str, policy: HtmlRenderPolicy) -> Element {
    match policy {
        HtmlRenderPolicy::Escape => rsx! { span { "{raw}" } },
        HtmlRenderPolicy::Trusted => rsx! { span { dangerous_inner_html: "{raw}" } },
    }
}

/// Sanitize an href value, blocking dangerous URI schemes.
///
/// Returns `Some(trimmed_url)` if the scheme is safe, `None` if blocked.
///
/// Blocked schemes (case-insensitive): `javascript`, `data`, `vbscript`.
/// Allowed: `http(s)`, `mailto`, `tel`, `ftp`, absolute paths (`/`),
/// anchors (`#`), and relative paths (no colon).
pub(crate) fn sanitize_href(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Some(String::new());
    }

    // Anchors and absolute paths are always safe
    if trimmed.starts_with('#') || trimmed.starts_with('/') {
        return Some(trimmed.to_string());
    }

    let lower = trimmed.to_lowercase();

    // Extract scheme (everything before the first ':')
    if let Some(colon_pos) = lower.find(':') {
        let scheme = &lower[..colon_pos];
        match scheme {
            "javascript" | "data" | "vbscript" => return None,
            _ => return Some(trimmed.to_string()),
        }
    }

    // No colon at all — relative path, safe
    Some(trimmed.to_string())
}

/// Render a comrak AST node tree into a Dioxus Element.
///
/// Block-level elements receive `data-source-line="{N}"` attributes from
/// comrak's sourcepos (1-indexed). This enables line-index scroll sync
/// between the editor and preview panes.
pub(crate) fn render_ast_to_element<'a>(root: &'a comrak::nodes::AstNode<'a>) -> Element {
    let children: Vec<Element> = root.children().map(|child| render_node(child)).collect();
    rsx! {
        for child in children {
            {child}
        }
    }
}

/// Render a single AST node (and its subtree) to an Element.
fn render_node<'a>(node: &'a comrak::nodes::AstNode<'a>) -> Element {
    let data = node.data.borrow();
    let sp = data.sourcepos;
    let source_line = sp.start.line.to_string();

    match &data.value {
        NodeValue::Document => {
            drop(data);
            render_ast_to_element(node)
        }
        NodeValue::FrontMatter(_) => {
            // Front matter is extracted separately; not rendered
            rsx! {}
        }
        NodeValue::Heading(h) => {
            let level = h.level;
            let level_str = level.to_string();
            drop(data);
            let children = render_children(node);
            match level {
                1 => {
                    rsx! { h1 { "data-md-heading": "{level_str}", "data-source-line": "{source_line}", {children} } }
                }
                2 => {
                    rsx! { h2 { "data-md-heading": "{level_str}", "data-source-line": "{source_line}", {children} } }
                }
                3 => {
                    rsx! { h3 { "data-md-heading": "{level_str}", "data-source-line": "{source_line}", {children} } }
                }
                4 => {
                    rsx! { h4 { "data-md-heading": "{level_str}", "data-source-line": "{source_line}", {children} } }
                }
                5 => {
                    rsx! { h5 { "data-md-heading": "{level_str}", "data-source-line": "{source_line}", {children} } }
                }
                _ => {
                    rsx! { h6 { "data-md-heading": "{level_str}", "data-source-line": "{source_line}", {children} } }
                }
            }
        }
        NodeValue::Paragraph => {
            drop(data);
            let children = render_children(node);
            rsx! { p { "data-source-line": "{source_line}", {children} } }
        }
        NodeValue::CodeBlock(cb) => {
            let lang = cb.info.split_whitespace().next().unwrap_or("").to_string();
            let code_text = cb.literal.clone();
            rsx! {
                pre {
                    "data-md-code-block": "",
                    "data-md-language": "{lang}",
                    "data-source-line": "{source_line}",
                    code {
                        class: "language-{lang}",
                        {code_text}
                    }
                }
            }
        }
        NodeValue::BlockQuote => {
            drop(data);
            let children = render_children(node);
            rsx! { blockquote { "data-source-line": "{source_line}", {children} } }
        }
        NodeValue::List(nl) => {
            let is_ordered = nl.list_type == ListType::Ordered;
            let start = nl.start;
            drop(data);
            let children = render_children(node);
            if is_ordered {
                rsx! { ol { start: "{start}", "data-source-line": "{source_line}", {children} } }
            } else {
                rsx! { ul { "data-source-line": "{source_line}", {children} } }
            }
        }
        NodeValue::Item(_) => {
            drop(data);
            let children = render_children(node);
            rsx! { li { "data-source-line": "{source_line}", {children} } }
        }
        NodeValue::TaskItem(ti) => {
            let checked = ti.symbol.is_some();
            let checked_str = if checked { "true" } else { "false" };
            drop(data);
            let children = render_children(node);
            rsx! {
                li {
                    "data-md-task-item": "",
                    "data-md-task-checked": "{checked_str}",
                    "data-source-line": "{source_line}",
                    input {
                        r#type: "checkbox",
                        checked: "{checked}",
                        disabled: true,
                    }
                    {children}
                }
            }
        }
        NodeValue::Table(_) => {
            drop(data);
            let children = render_children(node);
            rsx! {
                div {
                    "data-md-table-wrapper": "",
                    "data-source-line": "{source_line}",
                    table { {children} }
                }
            }
        }
        NodeValue::TableRow(is_header) => {
            let is_header = *is_header;
            drop(data);
            let children = render_children(node);
            if is_header {
                rsx! { thead { tr { {children} } } }
            } else {
                rsx! { tr { {children} } }
            }
        }
        NodeValue::TableCell => {
            let is_header = node
                .parent()
                .map(|p| matches!(p.data.borrow().value, NodeValue::TableRow(true)))
                .unwrap_or(false);
            drop(data);
            let children = render_children(node);
            if is_header {
                rsx! { th { scope: "col", {children} } }
            } else {
                rsx! { td { {children} } }
            }
        }
        NodeValue::ThematicBreak => {
            rsx! { hr { "data-source-line": "{source_line}" } }
        }
        NodeValue::HtmlBlock(hb) => {
            let fragment = render_html_fragment(&hb.literal, HtmlRenderPolicy::Escape);
            rsx! { div { "data-source-line": "{source_line}", {fragment} } }
        }
        NodeValue::FootnoteDefinition(fd) => {
            let name = fd.name.clone();
            drop(data);
            let children = render_children(node);
            rsx! {
                div {
                    "data-md-footnote-def": "{name}",
                    "data-source-line": "{source_line}",
                    {children}
                }
            }
        }
        // Inline nodes
        NodeValue::Text(t) => {
            let text = t.to_string();
            rsx! { "{text}" }
        }
        NodeValue::Emph => {
            drop(data);
            let children = render_children(node);
            rsx! { em { {children} } }
        }
        NodeValue::Strong => {
            drop(data);
            let children = render_children(node);
            rsx! { strong { {children} } }
        }
        NodeValue::Strikethrough => {
            drop(data);
            let children = render_children(node);
            rsx! { del { {children} } }
        }
        NodeValue::Code(c) => {
            let literal = c.literal.clone();
            rsx! { code { "{literal}" } }
        }
        NodeValue::Link(link) => {
            let safe_url = sanitize_href(&link.url).unwrap_or_default();
            let title = link.title.clone();
            let is_external = safe_url.starts_with("http://") || safe_url.starts_with("https://");
            let external_str = if is_external { "true" } else { "false" };
            drop(data);
            let children = render_children(node);
            rsx! {
                a {
                    href: "{safe_url}",
                    title: "{title}",
                    "data-md-link": "",
                    "data-md-link-external": "{external_str}",
                    {children}
                }
            }
        }
        NodeValue::Image(link) => {
            let url = sanitize_href(&link.url).unwrap_or_default();
            let title = link.title.clone();
            let alt = collect_text(node);
            rsx! { img { src: "{url}", alt: "{alt}", title: "{title}" } }
        }
        NodeValue::SoftBreak => rsx! { " " },
        NodeValue::LineBreak => rsx! { br {} },
        NodeValue::HtmlInline(html) => render_html_fragment(html, HtmlRenderPolicy::Escape),
        NodeValue::FootnoteReference(fr) => {
            let name = fr.name.clone();
            rsx! {
                sup {
                    a { href: "#fn-{name}", "data-md-footnote-ref": "{name}", "{name}" }
                }
            }
        }
        // Catch-all for node types we don't render (DescriptionList, etc.)
        _ => {
            drop(data);
            render_children(node)
        }
    }
}

/// Render all children of a node into a single Element.
fn render_children<'a>(node: &'a comrak::nodes::AstNode<'a>) -> Element {
    let children: Vec<Element> = node.children().map(|child| render_node(child)).collect();
    rsx! {
        for child in children {
            {child}
        }
    }
}

/// Render a code block to a Dioxus Element.
///
/// Without the `syntax-highlighting` feature: emits a plain `<pre><code>` with
/// `data-md-code-block` and `data-md-language` attributes and a CSS language class.
///
/// With the `syntax-highlighting` feature: same structure (full syntect integration
/// is deferred to Phase 3 completion; the feature flag wires in the capability).
#[cfg(feature = "syntax-highlighting")]
#[allow(dead_code)] // MUST only receive syntect-escaped HTML; never raw user input.
pub(crate) fn highlight_code(code: &str, lang: &str) -> Element {
    rsx! {
        pre {
            "data-md-code-block": "",
            "data-md-language": "{lang}",
            code {
                class: "language-{lang}",
                dangerous_inner_html: "{code}",
            }
        }
    }
}

#[cfg(not(feature = "syntax-highlighting"))]
#[allow(dead_code)] // MUST only receive syntect-escaped HTML; never raw user input.
pub(crate) fn highlight_code(code: &str, lang: &str) -> Element {
    rsx! {
        pre {
            "data-md-code-block": "",
            "data-md-language": "{lang}",
            code {
                class: "language-{lang}",
                {code}
            }
        }
    }
}

/// Convert a byte offset into (line, column) for a given text.
/// Both line and column are 0-based.
pub fn index_to_line_col(text: &str, index: usize) -> (usize, usize) {
    let before = &text[..index];
    let line = before.bytes().filter(|&b| b == b'\n').count();
    let col = match before.rfind('\n') {
        Some(nl_pos) => index - nl_pos - 1,
        None => index,
    };
    (line, col)
}
