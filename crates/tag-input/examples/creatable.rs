use dioxus::document::Stylesheet;
use dioxus::prelude::*;
use dioxus_nox_select::{SelectContext, select};
use dioxus_nox_tag_input::{Tag, TagInputState, extract_clipboard_text, use_tag_input};

fn main() {
    dioxus::launch(App);
}

/// Simple counter for generating unique IDs.
static NEXT_ID: GlobalSignal<u32> = Signal::global(|| 1000);

fn next_id() -> String {
    let id = *NEXT_ID.read();
    *NEXT_ID.write() += 1;
    format!("created-{id}")
}

fn seed_tags() -> Vec<Tag> {
    vec![
        Tag::new("work", "Work"),
        Tag::new("personal", "Personal"),
        Tag::new("urgent", "Urgent"),
        Tag::new("meeting", "Meeting"),
        Tag::new("followup", "Follow Up"),
        Tag::new("review", "Review"),
    ]
}

/// Bridge component that syncs select values <-> tag-input state.
#[component]
fn SelectTagBridge(available: Vec<Tag>, children: Element) -> Element {
    let mut state = use_context::<TagInputState<Tag>>();
    let mut select_ctx = use_context::<SelectContext>();

    // Mark that we have an input (combobox mode)
    use_hook(|| {
        select_ctx.mark_has_input();
    });

    // Forward sync: select -> tag-input (when select adds a value)
    let avail_fwd = available.clone();
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
                if let Some(tag) = avail_fwd.iter().find(|t| t.id == val.as_str()) {
                    state.add_tag(tag.clone());
                }
            }
        }
    });

    // Reverse sync: tag-input -> select (when tag is removed)
    use_effect(move || {
        let tag_ids: Vec<String> = state
            .selected_tags
            .read()
            .iter()
            .map(|t| t.id.clone())
            .collect();
        let select_values = select_ctx.current_values_peek();
        for val in &select_values {
            if !tag_ids.contains(val) {
                select_ctx.toggle_value(val);
            }
        }
    });

    rsx! { {children} }
}

#[component]
fn App() -> Element {
    let mut state = use_tag_input(seed_tags(), vec![]);
    use_context_provider(|| state);

    // Enable ad-hoc creation: type anything and press Enter.
    use_hook(|| {
        state.on_create.set(Some(Callback::new(move |text: String| {
            let tag = Tag::new(next_id(), text);
            Some(tag)
        })));
    });

    // Enable paste splitting on comma, newline, and tab.
    use_hook(|| {
        state.paste_delimiters.set(Some(vec![',', '\n', '\t']));
    });

    // Enable comma as a commit delimiter while typing.
    use_hook(|| {
        state.delimiters.set(Some(vec![',']));
    });

    let available = seed_tags();

    rsx! {
        Stylesheet { href: asset!("/assets/tailwind.css") }

        div {
            class: "min-h-screen bg-slate-900 text-slate-100 flex items-center justify-center p-6",

            div {
                class: "w-full max-w-md rounded-2xl border border-slate-700 bg-slate-800 p-6 shadow-xl",

                h1 {
                    class: "text-xl font-bold mb-1 text-slate-50",
                    "Tag your tasks"
                }
                p {
                    class: "text-sm text-slate-400 mb-4",
                    "Pick from the dropdown or type a new tag name and press Enter to create it. Comma commits too."
                }

                select::Root {
                    multiple: true,
                    autocomplete: dioxus_nox_select::AutoComplete::List,
                    open_on_focus: true,
                    class: "relative",

                    SelectTagBridge {
                        available: available.clone(),

                        // Tag input area
                        div {
                            class: "flex flex-wrap items-center gap-2 rounded-xl border border-slate-600 bg-slate-900 px-3 py-2 focus-within:border-emerald-500 focus-within:ring-1 focus-within:ring-emerald-500/50 transition-all motion-reduce:transition-none",

                            for (i, tag) in state.selected_tags.read().iter().cloned().enumerate() {
                                {
                                    let is_pill_active = (*state.active_pill.read()) == Some(i);
                                    let pill_ring = if is_pill_active { "ring-2 ring-emerald-400" } else { "" };
                                    rsx! {
                                        span {
                                            key: "{tag.id}",
                                            id: state.pill_id(i),
                                            class: "inline-flex items-center gap-1 rounded-lg bg-emerald-600/30 border border-emerald-500/40 px-2.5 py-0.5 text-sm text-emerald-200 transition-shadow motion-reduce:transition-none focus-visible:ring-2 focus-visible:ring-emerald-400 focus-visible:ring-offset-1 focus-visible:ring-offset-slate-900 {pill_ring}",
                                            "{tag.name}"
                                            button {
                                                r#type: "button",
                                                class: "ml-0.5 rounded hover:bg-emerald-500/30 px-1 transition-colors motion-reduce:transition-none",
                                                onclick: move |_| state.remove_tag(&tag.id),
                                                "\u{00D7}"
                                            }
                                        }
                                    }
                                }
                            }

                            CreatableComboInput { state }
                        }

                        select::Content {
                            class: "absolute z-50 mt-1 w-full rounded-xl border border-slate-700 bg-slate-800 shadow-lg max-h-60 overflow-y-auto",
                            select::Empty {
                                class: "px-3 py-2 text-sm text-slate-500",
                                "No matches. Press Enter to create."
                            }
                            for tag in &available {
                                select::Item {
                                    value: "{tag.id}",
                                    label: tag.name.clone(),
                                    class: "px-3 py-2 text-sm text-slate-200 cursor-pointer data-[highlighted]:bg-emerald-600/30 data-[state=checked]:text-emerald-300",
                                    "{tag.name}"
                                }
                            }
                        }
                    }
                }

                // Screen reader live region for status announcements
                div {
                    role: "status",
                    aria_live: "polite",
                    class: "sr-only absolute w-px h-px p-0 -m-px overflow-hidden [clip:rect(0,0,0,0)] whitespace-nowrap border-0",
                    "{state.status_message}"
                }

                // Keyboard hints
                p {
                    class: "mt-3 text-xs text-slate-500",
                    span { class: "font-mono bg-slate-700/50 rounded px-1 py-0.5 mr-1", "Enter" }
                    "create / select  "
                    span { class: "font-mono bg-slate-700/50 rounded px-1 py-0.5 mr-1", "Bksp" }
                    "remove  "
                    span { class: "font-mono bg-slate-700/50 rounded px-1 py-0.5 mr-1", "," }
                    "commit"
                }
            }
        }
    }
}

/// Combo-style input that handles select keyboard events, tag-input keyboard events,
/// and on_create fallback when no dropdown item is highlighted.
#[component]
fn CreatableComboInput(state: TagInputState<Tag>) -> Element {
    let mut select_ctx = use_context::<SelectContext>();
    let mut state = state;

    let listbox_id = select_ctx.listbox_id();
    let active_desc = select_ctx.active_descendant();
    let is_open = select_ctx.is_open();

    rsx! {
        input {
            r#type: "text",
            role: "combobox",
            aria_expanded: if is_open { "true" } else { "false" },
            aria_haspopup: "listbox",
            aria_controls: "{listbox_id}",
            aria_activedescendant: if !active_desc.is_empty() { "{active_desc}" },
            class: "flex-1 min-w-[100px] bg-transparent outline-none text-slate-100 placeholder-slate-500 text-sm",
            placeholder: "Add a tag\u{2026}",
            value: "{state.search_query}",
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
                // If a pill is active, delegate entirely to tag-input
                if state.active_pill.read().is_some() {
                    state.handle_keydown(evt);
                    return;
                }
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
                            // Select from dropdown
                            select_ctx.confirm_highlighted();
                            state.set_query(String::new());
                            select_ctx.set_search_query(String::new());
                        } else {
                            // No highlighted item — delegate to tag-input for on_create
                            state.handle_input_keydown(evt);
                            // Clear search query in select too
                            select_ctx.set_search_query(String::new());
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
                        // ArrowLeft, Backspace, delimiters, etc. -> tag-input handles
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
