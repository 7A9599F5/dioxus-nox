use dioxus::document::Stylesheet;
use dioxus::prelude::*;
use dioxus_nox_select::{SelectContext, select};
use dioxus_nox_tag_input::{TagInputState, TagLike, components as tag_input};

fn main() {
    dioxus::launch(App);
}

#[derive(Clone, PartialEq, Debug)]
struct ColorTag {
    id: String,
    name: String,
}

impl ColorTag {
    fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
        }
    }
}

impl TagLike for ColorTag {
    fn id(&self) -> &str {
        &self.id
    }
    fn name(&self) -> &str {
        &self.name
    }
}

fn color_tags() -> Vec<ColorTag> {
    vec![
        ColorTag::new("red", "Red"),
        ColorTag::new("blue", "Blue"),
        ColorTag::new("green", "Green"),
        ColorTag::new("yellow", "Yellow"),
        ColorTag::new("purple", "Purple"),
        ColorTag::new("orange", "Orange"),
        ColorTag::new("pink", "Pink"),
        ColorTag::new("teal", "Teal"),
    ]
}

#[component]
fn App() -> Element {
    // Parent-owned signals -- shared by both Root components
    let mut shared_tags: Signal<Vec<ColorTag>> =
        use_signal(|| vec![ColorTag::new("blue", "Blue")]);
    let shared_query: Signal<String> = use_signal(String::new);

    let tag_count = shared_tags.read().len();

    rsx! {
        Stylesheet { href: asset!("/assets/tailwind.css") }

        div {
            class: "min-h-screen bg-slate-900 text-slate-100 flex items-center justify-center p-6",

            div {
                class: "w-full max-w-2xl space-y-6",

                h1 { class: "text-2xl font-bold text-slate-50", "Controlled Mode Demo" }
                p {
                    class: "text-sm text-slate-400",
                    "Two tag inputs sharing the same signal. Adding a tag in one appears in both. Dropdown for selection."
                }

                // External controls
                div {
                    class: "flex items-center gap-3",
                    button {
                        class: "rounded-lg bg-rose-600 hover:bg-rose-500 px-4 py-2 text-sm font-medium transition-colors",
                        "data-testid": "clear-btn",
                        onclick: move |_| shared_tags.set(vec![]),
                        "Clear All"
                    }
                    button {
                        class: "rounded-lg bg-emerald-600 hover:bg-emerald-500 px-4 py-2 text-sm font-medium transition-colors",
                        "data-testid": "preset-btn",
                        onclick: move |_| shared_tags.set(vec![
                            ColorTag::new("red", "Red"),
                            ColorTag::new("green", "Green"),
                            ColorTag::new("blue", "Blue"),
                        ]),
                        "Preset (R/G/B)"
                    }
                    {
                        let suffix = if tag_count != 1 { "s" } else { "" };
                        rsx! {
                            span {
                                class: "text-sm text-slate-400",
                                "data-testid": "tag-count",
                                "{tag_count} tag{suffix} selected"
                            }
                        }
                    }
                }

                // Input A
                div {
                    class: "rounded-2xl border border-slate-700 bg-slate-800 p-5",
                    h2 { class: "text-lg font-semibold mb-3 text-slate-200", "Input A" }
                    tag_input::Root::<ColorTag> {
                        available_tags: color_tags(),
                        value: Some(shared_tags),
                        query: Some(shared_query),
                        TagInputPanel {}
                    }
                }

                // Input B
                div {
                    class: "rounded-2xl border border-slate-700 bg-slate-800 p-5",
                    h2 { class: "text-lg font-semibold mb-3 text-slate-200", "Input B" }
                    tag_input::Root::<ColorTag> {
                        available_tags: color_tags(),
                        value: Some(shared_tags),
                        query: Some(shared_query),
                        TagInputPanel {}
                    }
                }

                // Live signal readout
                div {
                    class: "rounded-lg bg-slate-800/60 border border-slate-700/50 px-4 py-3 text-xs font-mono text-slate-400",
                    "data-testid": "signal-readout",
                    span { class: "text-slate-500", "selected: " }
                    for (i, tag) in shared_tags.read().iter().enumerate() {
                        if i > 0 {
                            ", "
                        }
                        span { class: "text-indigo-300", "{tag.name()}" }
                    }
                    br {}
                    span { class: "text-slate-500", "query: " }
                    span { class: "text-amber-300", "\"{shared_query}\"" }
                }
            }
        }
    }
}

/// Bridge that syncs select values <-> tag-input state for compound component usage.
#[component]
fn SelectTagBridgeControlled(children: Element) -> Element {
    let mut tag_ctx = use_context::<TagInputState<ColorTag>>();
    let mut select_ctx = use_context::<SelectContext>();

    // Mark that we have an input (combobox mode)
    use_hook(|| {
        select_ctx.mark_has_input();
    });

    let available = color_tags();

    // Forward sync: select -> tag-input (when select adds a value)
    use_effect(move || {
        let selected_values = select_ctx.current_values();
        let tag_ids: Vec<String> = tag_ctx
            .selected_tags
            .peek()
            .iter()
            .map(|t| t.id().to_string())
            .collect();
        for val in &selected_values {
            if !tag_ids.contains(val) {
                if let Some(tag) = available.iter().find(|t| t.id() == val.as_str()) {
                    tag_ctx.add_tag(tag.clone());
                }
            }
        }
    });

    // Reverse sync: tag-input -> select (when tag is removed)
    use_effect(move || {
        let tag_ids: Vec<String> = tag_ctx
            .selected_tags
            .read()
            .iter()
            .map(|t| t.id().to_string())
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

/// Combo-style input wired to both tag-input and select contexts.
#[component]
fn ControlledComboInput() -> Element {
    let mut tag_ctx = use_context::<TagInputState<ColorTag>>();
    let mut select_ctx = use_context::<SelectContext>();

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
            placeholder: "Type to search\u{2026}",
            value: "{tag_ctx.search_query}",
            "data-slot": "input",
            oninput: move |evt| {
                let val = evt.value();
                tag_ctx.set_query(val.clone());
                select_ctx.set_search_query(val);
                if !select_ctx.is_open() {
                    select_ctx.set_open(true);
                }
                select_ctx.highlight_first();
            },
            onkeydown: move |evt| {
                // If a pill is active, delegate entirely to tag-input
                if tag_ctx.active_pill.read().is_some() {
                    tag_ctx.handle_keydown(evt);
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
                            select_ctx.confirm_highlighted();
                            tag_ctx.set_query(String::new());
                            select_ctx.set_search_query(String::new());
                        }
                    }
                    Key::Escape => {
                        evt.prevent_default();
                        if select_ctx.is_open() {
                            select_ctx.set_open(false);
                        }
                        tag_ctx.active_pill.set(None);
                    }
                    Key::Tab => {
                        if select_ctx.is_open() {
                            select_ctx.set_open(false);
                        }
                    }
                    _ => {
                        tag_ctx.handle_input_keydown(evt);
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
                tag_ctx.handle_click();
                if !select_ctx.is_open() {
                    select_ctx.set_open(true);
                }
            },
        }
    }
}

#[component]
fn TagInputPanel() -> Element {
    let tag_ctx = use_context::<TagInputState<ColorTag>>();
    let available = color_tags();

    rsx! {
        select::Root {
            multiple: true,
            autocomplete: dioxus_nox_select::AutoComplete::List,
            open_on_focus: true,
            class: "relative",

            SelectTagBridgeControlled {
                tag_input::Control::<ColorTag> {
                    class: "flex flex-wrap items-center gap-2 rounded-xl border border-slate-600 bg-slate-900 px-3 py-2 focus-within:border-indigo-500 focus-within:ring-1 focus-within:ring-indigo-500/50 transition-all motion-reduce:transition-none",

                    for (i, tag) in tag_ctx.visible_tags.read().iter().cloned().enumerate() {
                        {
                            let key = tag.id().to_string();
                            let name = tag.name().to_string();
                            rsx! {
                                tag_input::Tag {
                                    key: "{key}",
                                    tag: tag.clone(),
                                    index: i,
                                    class: "inline-flex items-center gap-1 rounded-lg bg-indigo-600/30 border border-indigo-500/40 px-2.5 py-0.5 text-sm text-indigo-200 transition-shadow motion-reduce:transition-none focus-visible:ring-2 focus-visible:ring-indigo-400 focus-visible:ring-offset-1 focus-visible:ring-offset-slate-900",
                                    "{name}"
                                    tag_input::TagRemove {
                                        tag: tag.clone(),
                                        class: "ml-0.5 rounded hover:bg-indigo-500/30 px-1 transition-colors motion-reduce:transition-none",
                                    }
                                }
                            }
                        }
                    }

                    ControlledComboInput {}
                }

                select::Content {
                    class: "absolute z-50 mt-1 w-full rounded-xl border border-slate-700 bg-slate-800 shadow-lg max-h-60 overflow-y-auto",
                    select::Empty {
                        class: "px-3 py-2 text-sm text-slate-500",
                        "No results found."
                    }
                    for tag in &available {
                        select::Item {
                            value: tag.id().to_string(),
                            label: tag.name().to_string(),
                            class: "px-3 py-2 text-sm text-slate-200 cursor-pointer data-[highlighted]:bg-indigo-600/30 data-[state=checked]:text-indigo-300",
                            "{tag.name()}"
                        }
                    }
                }
            }
        }

        tag_input::LiveRegion::<ColorTag> {}
    }
}
