use dioxus::prelude::*;
use dioxus_nox_suggest::{TriggerConfig, TriggerSelectEvent, suggest, use_suggestion};

// ── Static data ───────────────────────────────────────────────────────────────

const SLASH_COMMANDS: &[(&str, &str)] = &[
    ("bold", "Bold"),
    ("italic", "Italic"),
    ("heading", "Heading"),
    ("code", "Code block"),
    ("divider", "Divider"),
];

const MENTIONS: &[&str] = &["alice", "bob", "carol", "dave", "eve"];

// ── Styles ────────────────────────────────────────────────────────────────────

const CSS: &str = r#"
*, *::before, *::after { box-sizing: border-box; }

body {
    margin: 0;
    font-family: system-ui, -apple-system, sans-serif;
    background: #f8fafc;
    color: #1e293b;
}

.composer {
    max-width: 600px;
    margin: 48px auto;
    padding: 0 24px;
}

.composer h1 { margin: 0 0 4px; font-size: 1.5rem; }
.composer p  { margin: 0 0 16px; color: #64748b; font-size: 0.9rem; }

/* Textarea */
.editor {
    width: 100%;
    height: 120px;
    padding: 12px;
    font-size: 15px;
    resize: vertical;
    border: 1px solid #cbd5e1;
    border-radius: 8px;
    outline: none;
    font-family: inherit;
    background: #fff;
    transition: border-color 0.15s;
}
.editor:focus { border-color: #94a3b8; }

/* Suggestion list — position:fixed is set by the component via inline style */
[data-slot="trigger-list"] { position: fixed; z-index: 50; }

[data-slot="trigger-list"][data-state="closed"] { display: none; }

[data-slot="trigger-list"][data-state="open"] {
    background: #ffffff;
    border: 1px solid #e2e8f0;
    border-radius: 8px;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.12);
    overflow-y: auto;
    min-width: 200px;
    max-height: 280px;
}

/* Items */
[role="option"] {
    padding: 8px 12px;
    cursor: pointer;
    font-size: 14px;
    transition: background 0.1s;
}
[role="option"]:hover               { background: #f1f5f9; }
[role="option"][data-highlighted="true"] { background: #e2e8f0; }

/* Empty state */
[data-suggest-empty] {
    padding: 8px 12px;
    font-size: 14px;
    color: #94a3b8;
    font-style: italic;
}
"#;

// ── App ───────────────────────────────────────────────────────────────────────

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut text = use_signal(String::new);

    rsx! {
        style { {CSS} }
        div { class: "composer",
            h1 { "Suggest Demo" }
            p { "Type / for commands · @ to mention someone" }

            suggest::Root {
                triggers: vec![TriggerConfig::slash(), TriggerConfig::mention()],
                on_select: move |ev: TriggerSelectEvent| {
                    let mut t = text.write();
                    let end = ev.trigger_offset
                        + ev.trigger_char.len_utf8()
                        + ev.filter.len();
                    t.replace_range(ev.trigger_offset..end, &ev.value);
                },

                suggest::Trigger {
                    textarea {
                        class: "editor",
                        placeholder: "Start typing…",
                        value: "{text.read()}",
                        oninput: move |e| text.set(e.value()),
                    }
                }

                SuggestionPopup {}
            }
        }
    }
}

// ── SuggestionPopup ───────────────────────────────────────────────────────────

/// Renders the floating list of suggestions for the active trigger.
///
/// Demonstrates the full consumer-side pattern:
/// - `use_suggestion()` for reactive access to trigger state
/// - Filtering static data against the live filter string
/// - `suggest::List` / `suggest::Item` / `suggest::Empty` assembly
#[component]
fn SuggestionPopup() -> Element {
    let handle = use_suggestion();

    // Read both signals in the render body to subscribe this component to
    // changes in active_char and filter (Dioxus 0.7 reactivity model).
    let active = handle.active_char();
    let filter = handle.filter().to_lowercase();

    // Pre-compute filtered lists.
    let slash_items: Vec<(&str, &str)> = SLASH_COMMANDS
        .iter()
        .copied()
        .filter(|(cmd, _)| cmd.contains(filter.as_str()))
        .collect();

    let mention_items: Vec<&str> = MENTIONS
        .iter()
        .copied()
        .filter(|name| name.starts_with(filter.as_str()))
        .collect();

    let show_empty = match active {
        Some('/') => slash_items.is_empty(),
        Some('@') => mention_items.is_empty(),
        _ => false,
    };

    rsx! {
        suggest::List {
            if active == Some('/') {
                for (cmd, label) in slash_items {
                    suggest::Item { key: "{cmd}", value: "/{cmd}", "{label}" }
                }
            }
            if active == Some('@') {
                for name in mention_items {
                    suggest::Item { key: "{name}", value: "@{name}", "{name}" }
                }
            }
            if show_empty {
                suggest::Empty { "No results" }
            }
        }
    }
}
