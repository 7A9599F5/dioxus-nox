use dioxus::document::Stylesheet;
use dioxus::prelude::*;
use dioxus_nox_select::{select, AutoComplete, SelectContext};
use dioxus_nox_tag_input::{
    Breakpoint, Tag, TagInputState, TagLike, extract_clipboard_text, use_breakpoint, use_tag_input,
};

fn main() {
    dioxus::launch(App);
}

fn sample_tags() -> Vec<Tag> {
    vec![
        Tag::new("rust", "Rust"),
        Tag::new("python", "Python"),
        Tag::new("typescript", "TypeScript"),
        Tag::new("go", "Go"),
        Tag::new("java", "Java"),
        Tag::new("csharp", "C#"),
        Tag::new("swift", "Swift"),
        Tag::new("kotlin", "Kotlin"),
        Tag::new("ruby", "Ruby"),
        Tag::new("elixir", "Elixir"),
    ]
}

#[component]
fn App() -> Element {
    let state = use_tag_input(sample_tags(), vec![]);
    // Provide TagInputState via context so the bridge can find it
    use_context_provider(|| state);
    let breakpoint = use_breakpoint();

    let is_mobile = matches!(*breakpoint.read(), Breakpoint::Mobile);

    rsx! {
        Stylesheet { href: asset!("/assets/tailwind.css") }

        div {
            class: "min-h-screen bg-slate-900 text-slate-100 flex items-start justify-center p-4 sm:p-8",

            div {
                class: "w-full max-w-xl",

                h1 {
                    class: "text-2xl font-bold mb-6 text-slate-50",
                    "Tag Input Demo"
                }

                if is_mobile {
                    MobileLayout { state: state }
                } else {
                    DesktopLayout { state: state }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Bridge: syncs select values <-> tag-input tags
// ---------------------------------------------------------------------------

#[component]
fn SelectTagBridge(available: Vec<Tag>, children: Element) -> Element {
    let mut state = use_context::<TagInputState<Tag>>();
    let mut select_ctx = use_context::<SelectContext>();

    // Forward sync: select values -> tag-input (add new selections)
    use_effect(move || {
        let selected_values = select_ctx.current_values();
        let tag_ids: Vec<String> = state
            .selected_tags
            .peek()
            .iter()
            .map(|t| t.id().to_string())
            .collect();

        for val in &selected_values {
            if !tag_ids.contains(val)
                && let Some(tag) = available.iter().find(|t| t.id() == val.as_str())
            {
                state.add_tag(tag.clone());
            }
        }
    });

    // Reverse sync: tag-input removals -> select values
    use_effect(move || {
        let tag_ids: Vec<String> = state
            .selected_tags
            .read()
            .iter()
            .map(|t| t.id().to_string())
            .collect();

        for val in &select_ctx.current_values_peek() {
            if !tag_ids.contains(val) {
                select_ctx.toggle_value(val);
            }
        }
    });

    rsx! { {children} }
}

// ---------------------------------------------------------------------------
// Desktop / Tablet layout
// ---------------------------------------------------------------------------

#[component]
fn DesktopLayout(mut state: TagInputState<Tag>) -> Element {
    let tags = sample_tags();

    rsx! {
        select::Root {
            multiple: true,
            autocomplete: AutoComplete::List,

            SelectTagBridge {
                available: tags.clone(),

                div {
                    class: "relative",

                    // Selected tags + input row
                    div {
                        class: "flex flex-wrap items-center gap-2 rounded-full border-2 border-dashed border-slate-600 bg-slate-800 px-4 py-2 focus-within:border-slate-400 transition-colors",

                        // Selected tag pills
                        for (i, tag) in state.selected_tags.read().iter().cloned().enumerate() {
                            DesktopPill {
                                key: "{tag.id}",
                                tag: tag.clone(),
                                active: (*state.active_pill.read()) == Some(i),
                                pill_id: state.pill_id(i),
                                on_remove: move |id: String| state.remove_tag(&id),
                            }
                        }

                        // Add icon
                        span {
                            class: "text-slate-400 text-lg select-none",
                            "+"
                        }

                        // Combo search input
                        ComboInput { state: state }
                    }

                    // Dropdown
                    DropdownContent { tags: tags.clone() }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Combo input: bridges select dropdown nav + tag-input pill keys
// ---------------------------------------------------------------------------

#[component]
fn ComboInput(mut state: TagInputState<Tag>) -> Element {
    let mut select_ctx = use_context::<SelectContext>();
    let listbox_id = select_ctx.listbox_id();

    use_hook(|| {
        select_ctx.mark_has_input();
    });

    rsx! {
        input {
            r#type: "text",
            role: "combobox",
            class: "flex-1 min-w-[120px] bg-transparent outline-none text-slate-100 placeholder-slate-500",
            placeholder: "Add a tag\u{2026}",
            value: "{state.search_query}",
            autocomplete: "off",
            aria_expanded: select_ctx.is_open(),
            aria_controls: "{listbox_id}",
            aria_activedescendant: select_ctx.active_descendant(),
            oninput: move |evt: Event<FormData>| {
                let val = evt.value();
                state.set_query(val.clone());
                select_ctx.set_search_query(val);
                if !select_ctx.is_open() {
                    select_ctx.set_open(true);
                }
                select_ctx.highlight_first();
            },
            onkeydown: move |evt: Event<KeyboardData>| {
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

// ---------------------------------------------------------------------------
// Dropdown content
// ---------------------------------------------------------------------------

#[component]
fn DropdownContent(tags: Vec<Tag>) -> Element {
    rsx! {
        select::Content {
            class: "absolute z-50 mt-1 w-full rounded-xl border border-slate-700 bg-slate-800 shadow-lg max-h-60 overflow-y-auto",
            select::Empty {
                class: "px-3 py-2 text-sm text-slate-500",
                "No results found."
            }
            for tag in &tags {
                select::Item {
                    value: "{tag.id}",
                    label: tag.name.to_string(),
                    class: "px-3 py-2 text-sm text-slate-200 cursor-pointer data-[highlighted]:bg-indigo-600/30 data-[state=checked]:text-indigo-300",
                    "{tag.name}"
                }
            }
        }
    }
}

#[component]
fn DesktopPill(
    tag: Tag,
    active: bool,
    pill_id: String,
    on_remove: EventHandler<String>,
) -> Element {
    let pill_ring = if active { "ring-2 ring-indigo-400" } else { "" };
    rsx! {
        span {
            id: pill_id,
            class: "inline-flex items-center gap-1 rounded-full bg-slate-600 px-3 py-1 text-sm text-slate-100 transition-shadow {pill_ring}",
            "{tag.name}"
            button {
                r#type: "button",
                class: "ml-1 rounded-full hover:bg-slate-500 p-0.5 transition-colors",
                onclick: move |_| on_remove.call(tag.id.clone()),
                "\u{00D7}"
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Mobile layout
// ---------------------------------------------------------------------------

#[component]
fn MobileLayout(mut state: TagInputState<Tag>) -> Element {
    rsx! {
        div {
            // Horizontal scroll tag strip
            div {
                class: "flex flex-nowrap overflow-x-auto gap-2 pb-3 scrollbar-none",
                for (i, tag) in state.selected_tags.read().iter().cloned().enumerate() {
                    MobilePill {
                        key: "{tag.id}",
                        tag: tag.clone(),
                        active: (*state.active_pill.read()) == Some(i),
                        pill_id: state.pill_id(i),
                        on_remove: move |id: String| state.remove_tag(&id),
                    }
                }
            }

            // Input row
            div {
                class: "flex items-center gap-2 rounded-xl border-2 border-dashed border-slate-600 bg-slate-800 px-4 py-3",
                span {
                    class: "text-slate-400 text-lg select-none",
                    "+"
                }
                input {
                    r#type: "text",
                    class: "flex-1 bg-transparent outline-none text-slate-100 placeholder-slate-500 text-base",
                    placeholder: "Add a tag\u{2026}",
                    value: "{state.search_query}",
                    oninput: move |evt| state.set_query(evt.value()),
                    onkeydown: move |evt| state.handle_keydown(evt),
                    onclick: move |_| state.handle_click(),
                }
            }
        }
    }
}

#[component]
fn MobilePill(tag: Tag, active: bool, pill_id: String, on_remove: EventHandler<String>) -> Element {
    let pill_ring = if active { "ring-2 ring-indigo-400" } else { "" };
    rsx! {
        span {
            id: pill_id,
            class: "inline-flex items-center gap-1 shrink-0 rounded-full bg-slate-600 px-3 py-2 text-sm text-slate-100 transition-shadow {pill_ring}",
            "{tag.name}"
            button {
                r#type: "button",
                class: "min-w-[44px] min-h-[44px] flex items-center justify-center rounded-full hover:bg-slate-500 transition-colors -mr-1",
                onclick: move |_| on_remove.call(tag.id.clone()),
                "\u{00D7}"
            }
        }
    }
}
