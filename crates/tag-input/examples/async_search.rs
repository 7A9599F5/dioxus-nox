use dioxus::document::Stylesheet;
use dioxus::prelude::*;
use dioxus_nox_select::{select, AutoComplete, SelectContext};
use dioxus_nox_tag_input::{Tag, TagInputState, extract_clipboard_text, use_tag_input};

fn main() {
    dioxus::launch(App);
}

/// Simulated database of tags that would normally come from a server.
fn all_tags() -> Vec<Tag> {
    vec![
        Tag::new("rust", "Rust"),
        Tag::new("python", "Python"),
        Tag::new("javascript", "JavaScript"),
        Tag::new("typescript", "TypeScript"),
        Tag::new("go", "Go"),
        Tag::new("java", "Java"),
        Tag::new("csharp", "C#"),
        Tag::new("cpp", "C++"),
        Tag::new("ruby", "Ruby"),
        Tag::new("swift", "Swift"),
        Tag::new("kotlin", "Kotlin"),
        Tag::new("scala", "Scala"),
        Tag::new("elixir", "Elixir"),
        Tag::new("haskell", "Haskell"),
        Tag::new("clojure", "Clojure"),
        Tag::new("php", "PHP"),
        Tag::new("dart", "Dart"),
        Tag::new("lua", "Lua"),
        Tag::new("zig", "Zig"),
        Tag::new("nim", "Nim"),
    ]
}

/// Bridge component that syncs select context values with tag-input state.
///
/// When the select confirms a value (via Enter or click), the bridge detects
/// the new value in `select_ctx.current_values()` and adds the corresponding
/// tag. Conversely, when a tag is removed from tag-input, the bridge toggles
/// the value off in the select context.
#[component]
fn SelectTagBridge(available: Vec<Tag>, children: Element) -> Element {
    let mut state = use_context::<TagInputState<Tag>>();
    let mut select_ctx = use_context::<SelectContext>();

    // Select -> TagInput: when select adds a value, add the tag
    use_effect(move || {
        let selected_values = select_ctx.current_values();
        let tag_ids: Vec<String> = state
            .selected_tags
            .peek()
            .iter()
            .map(|t| t.id.clone())
            .collect();
        for val in &selected_values {
            if !tag_ids.contains(val) {
                if let Some(tag) = available.iter().find(|t| t.id == val.as_str()) {
                    state.add_tag(tag.clone());
                }
            }
        }
    });

    // TagInput -> Select: when a tag is removed, deselect in select context
    use_effect(move || {
        let tag_ids: Vec<String> = state
            .selected_tags
            .read()
            .iter()
            .map(|t| t.id.clone())
            .collect();
        for val in &select_ctx.current_values_peek() {
            if !tag_ids.contains(val) {
                select_ctx.toggle_value(val);
            }
        }
    });

    rsx! { {children} }
}

#[component]
fn App() -> Element {
    let tags = all_tags();
    let mut state = use_tag_input(tags.clone(), vec![]);

    // Provide TagInputState as context so the bridge can access it
    use_context_provider(|| state);

    rsx! {
        Stylesheet { href: asset!("/assets/tailwind.css") }

        div {
            class: "min-h-screen bg-slate-900 text-slate-100 flex items-center justify-center p-6",

            div {
                class: "w-full max-w-md rounded-2xl border border-slate-700 bg-slate-800 p-6 shadow-xl",

                h1 {
                    class: "text-xl font-bold mb-1 text-slate-50",
                    "Tag Search"
                }
                p {
                    class: "text-sm text-slate-400 mb-4",
                    "Type to search programming languages. Select from the dropdown to add tags."
                }

                select::Root {
                    multiple: true,
                    autocomplete: AutoComplete::List,
                    open_on_focus: true,
                    class: "relative",

                    SelectTagBridge { available: tags.clone(),

                        div {
                            class: "flex flex-wrap items-center gap-2 rounded-xl border border-slate-600 bg-slate-900 px-3 py-2 focus-within:border-cyan-500 focus-within:ring-1 focus-within:ring-cyan-500/50 transition-all motion-reduce:transition-none",

                            for (i, tag) in state.selected_tags.read().iter().cloned().enumerate() {
                                {
                                    let is_pill_active = (*state.active_pill.read()) == Some(i);
                                    let pill_ring = if is_pill_active { "ring-2 ring-cyan-400" } else { "" };
                                    rsx! {
                                        span {
                                            key: "{tag.id}",
                                            id: state.pill_id(i),
                                            class: "inline-flex items-center gap-1 rounded-lg bg-cyan-600/30 border border-cyan-500/40 px-2.5 py-0.5 text-sm text-cyan-200 transition-shadow motion-reduce:transition-none focus-visible:ring-2 focus-visible:ring-cyan-400 {pill_ring}",
                                            "{tag.name}"
                                            button {
                                                r#type: "button",
                                                class: "ml-0.5 rounded hover:bg-cyan-500/30 px-1 transition-colors motion-reduce:transition-none",
                                                onclick: move |_| state.remove_tag(&tag.id),
                                                "\u{00D7}"
                                            }
                                        }
                                    }
                                }
                            }

                            ComboInput { state }
                        }

                        // Dropdown
                        select::Content {
                            class: "absolute z-50 mt-1 w-full rounded-xl border border-slate-700 bg-slate-800 shadow-lg max-h-60 overflow-y-auto",
                            select::Empty {
                                class: "px-3 py-2 text-sm text-slate-500",
                                "No results found."
                            }
                            for tag in &tags {
                                select::Item {
                                    value: "{tag.id}",
                                    label: tag.name.clone(),
                                    class: "px-3 py-2 text-sm text-slate-200 cursor-pointer data-[highlighted]:bg-indigo-600/30 data-[state=checked]:text-indigo-300",
                                    "{tag.name}"
                                }
                            }
                        }
                    }
                }

                // Screen-reader live region
                div {
                    role: "status",
                    aria_live: "polite",
                    class: "absolute w-px h-px p-0 -m-px overflow-hidden [clip:rect(0,0,0,0)] whitespace-nowrap border-0",
                    "{state.status_message}"
                }

                p {
                    class: "mt-3 text-xs text-slate-500",
                    "Demonstrates: tag-input + select dropdown with fuzzy search"
                }
            }
        }
    }
}

/// Combobox-style input that wires keyboard/mouse events to both
/// tag-input state and select context.
#[component]
fn ComboInput(mut state: TagInputState<Tag>) -> Element {
    let mut select_ctx = use_context::<SelectContext>();
    let listbox_id = select_ctx.listbox_id();

    rsx! {
        input {
            r#type: "text",
            role: "combobox",
            class: "flex-1 min-w-[100px] bg-transparent outline-none text-slate-100 placeholder-slate-500 text-sm",
            placeholder: "Search languages\u{2026}",
            value: "{state.search_query}",
            autocomplete: "off",
            aria_expanded: select_ctx.is_open(),
            aria_controls: "{listbox_id}",
            aria_activedescendant: select_ctx.active_descendant(),
            oninput: move |evt| {
                let val = evt.value();
                state.set_query(val.clone());
                select_ctx.set_search_query(val);
                if !select_ctx.is_open() {
                    select_ctx.set_open(true);
                }
                select_ctx.highlight_first();
            },
            onkeydown: move |evt| {
                match evt.key() {
                    Key::ArrowDown => {
                        evt.prevent_default();
                        if !select_ctx.is_open() {
                            select_ctx.set_open(true);
                            select_ctx.highlight_first();
                        } else {
                            select_ctx.highlight_next();
                        }
                    }
                    Key::ArrowUp => {
                        if select_ctx.is_open() {
                            evt.prevent_default();
                            select_ctx.highlight_prev();
                        }
                    }
                    Key::Enter => {
                        evt.prevent_default();
                        if select_ctx.is_open() && select_ctx.has_highlighted() {
                            select_ctx.confirm_highlighted();
                            state.set_query(String::new());
                            select_ctx.set_search_query(String::new());
                        } else {
                            state.handle_input_keydown(evt);
                        }
                    }
                    Key::Escape => {
                        evt.prevent_default();
                        if select_ctx.is_open() {
                            select_ctx.set_open(false);
                        }
                        state.active_pill.set(None);
                    }
                    Key::Tab => {
                        if select_ctx.is_open() {
                            select_ctx.set_open(false);
                        }
                    }
                    _ => {
                        state.handle_input_keydown(evt);
                    }
                }
            },
            onfocus: move |_| {
                if select_ctx.open_on_focus() {
                    select_ctx.set_open(true);
                }
            },
            onblur: move |_| {
                select_ctx.set_open(false);
            },
            onclick: move |_| {
                state.handle_click();
                if !select_ctx.is_open() {
                    select_ctx.set_open(true);
                }
            },
            onpaste: move |evt: Event<ClipboardData>| {
                if let Some(text) = extract_clipboard_text(&evt) {
                    evt.prevent_default();
                    state.handle_paste(text);
                }
            },
        }
    }
}
