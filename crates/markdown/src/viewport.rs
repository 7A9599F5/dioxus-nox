use std::rc::Rc;

use dioxus::prelude::*;

use crate::context::MarkdownContext;
use crate::types::{NodeType, OwnedAstNode};

/// A custom component override for a specific Markdown block.
#[derive(Clone)]
pub struct BlockOverride {
    pub matches: Rc<dyn Fn(&OwnedAstNode) -> bool>,
    // Store a callback that returns an Element
    pub component: Rc<dyn Fn(OwnedAstNode) -> Element>,
}

impl PartialEq for BlockOverride {
    fn eq(&self, _other: &Self) -> bool {
        // Functions cannot be easily compared; always trigger an update if overrides change.
        false
    }
}

/// The core headless virtual viewport renderer.
/// Iterates over the `ParsedDoc::ast` and renders each node.
#[derive(Props, Clone, PartialEq)]
pub struct EditorViewportProps {
    /// Optional overrides for specific AST nodes.
    #[props(default)]
    pub overrides: Vec<BlockOverride>,
}

#[component]
pub fn EditorViewport(props: EditorViewportProps) -> Element {
    let ctx = use_context::<MarkdownContext>();
    let parsed = (ctx.parsed_doc)();

    rsx! {
        div {
            class: "nox-md-viewport",
            "data-md-viewport": "true",
            // Render the root AST nodes
            for node in parsed.ast.iter() {
                ViewportNode {
                    node: node.clone(),
                    overrides: props.overrides.clone()
                }
            }
        }
    }
}

#[component]
pub fn ViewportNode(node: OwnedAstNode, overrides: Vec<BlockOverride>) -> Element {
    // Check if any override matches this node
    for ov in overrides.iter() {
        if (ov.matches)(&node) {
            return (ov.component)(node.clone());
        }
    }

    // Default rendering recursively based on NodeType
    match &node.node_type {
        NodeType::Paragraph => {
            rsx! { p { for c in node.children { ViewportNode { node: c, overrides: overrides.clone() } } } }
        }
        NodeType::Heading(level) => match level {
            1 => {
                rsx! { h1 { for c in node.children { ViewportNode { node: c, overrides: overrides.clone() } } } }
            }
            2 => {
                rsx! { h2 { for c in node.children { ViewportNode { node: c, overrides: overrides.clone() } } } }
            }
            3 => {
                rsx! { h3 { for c in node.children { ViewportNode { node: c, overrides: overrides.clone() } } } }
            }
            4 => {
                rsx! { h4 { for c in node.children { ViewportNode { node: c, overrides: overrides.clone() } } } }
            }
            5 => {
                rsx! { h5 { for c in node.children { ViewportNode { node: c, overrides: overrides.clone() } } } }
            }
            _ => {
                rsx! { h6 { for c in node.children { ViewportNode { node: c, overrides: overrides.clone() } } } }
            }
        },
        NodeType::Text(t) => {
            let txt = t.clone();
            rsx! { "{txt}" }
        }
        NodeType::Code(c) => {
            let code = c.clone();
            rsx! { code { "{code}" } }
        }
        NodeType::SoftBreak => rsx! { " " },
        NodeType::HardBreak => rsx! { br {} },
        NodeType::Html(h) => {
            let html = h.clone();
            rsx! { span { "{html}" } }
        }
        NodeType::Rule => rsx! { hr {} },
        NodeType::Emphasis => {
            rsx! { em { for c in node.children { ViewportNode { node: c, overrides: overrides.clone() } } } }
        }
        NodeType::Strong => {
            rsx! { strong { for c in node.children { ViewportNode { node: c, overrides: overrides.clone() } } } }
        }
        NodeType::Strikethrough => {
            rsx! { del { for c in node.children { ViewportNode { node: c, overrides: overrides.clone() } } } }
        }
        NodeType::BlockQuote => {
            rsx! { blockquote { for c in node.children { ViewportNode { node: c, overrides: overrides.clone() } } } }
        }
        NodeType::Wikilink(link) => {
            let l = link.clone();
            rsx! { a { "data-md-wikilink": "{l}", "[[{l}]]" } }
        }
        NodeType::Tag(t) => {
            let tag = t.clone();
            rsx! { span { "data-md-tag": "{tag}", "{tag}" } }
        }
        NodeType::CodeBlock(lang) => {
            let l = lang.clone();
            rsx! {
                pre {
                    "data-md-code-block": "",
                    "data-md-language": "{l}",
                    code {
                        class: "language-{l}",
                        for c in node.children { ViewportNode { node: c, overrides: overrides.clone() } }
                    }
                }
            }
        }
        NodeType::List(_) => {
            rsx! { ul { for c in node.children { ViewportNode { node: c, overrides: overrides.clone() } } } }
        }
        NodeType::Item => {
            rsx! { li { for c in node.children { ViewportNode { node: c, overrides: overrides.clone() } } } }
        }
        _ => {
            rsx! { span { for c in node.children { ViewportNode { node: c, overrides: overrides.clone() } } } }
        }
    }
}
