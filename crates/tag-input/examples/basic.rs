use dioxus::document::Stylesheet;
use dioxus::prelude::*;
use dioxus_nox_tag_input::{TagInputState, TagLike, combo};

fn main() {
    dioxus::launch(App);
}

/// A tag that supports locking (locked tags cannot be removed).
#[derive(Clone, PartialEq, Debug)]
struct FruitTag {
    id: String,
    name: String,
    locked: bool,
}

impl FruitTag {
    fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            locked: false,
        }
    }
    fn locked(id: &str, name: &str) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            locked: true,
        }
    }
}

impl TagLike for FruitTag {
    fn id(&self) -> &str {
        &self.id
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn is_locked(&self) -> bool {
        self.locked
    }
}

fn fruit_tags() -> Vec<FruitTag> {
    vec![
        FruitTag::new("apple", "Apple"),
        FruitTag::new("banana", "Banana"),
        FruitTag::new("cherry", "Cherry"),
        FruitTag::new("grape", "Grape"),
        FruitTag::new("mango", "Mango"),
        FruitTag::new("orange", "Orange"),
    ]
}

#[component]
fn App() -> Element {
    let mut last_event = use_signal(|| String::from("No events yet"));
    let available = fruit_tags();

    rsx! {
        Stylesheet { href: asset!("/assets/tailwind.css") }

        div {
            class: "min-h-screen bg-slate-900 text-slate-100 flex items-center justify-center p-6",

            div {
                class: "w-full max-w-md rounded-2xl border border-slate-700 bg-slate-800 p-6 shadow-xl",

                h1 {
                    class: "text-xl font-bold mb-1 text-slate-50",
                    "Pick some fruits"
                }
                p {
                    class: "text-sm text-slate-400 mb-4",
                    "Cherry is locked and cannot be removed. Max 4 tags."
                }

                combo::Root::<FruitTag> {
                    available_tags: available.clone(),
                    initial_selected: vec![FruitTag::locked("cherry", "Cherry")],
                    max_tags: Some(4),
                    on_add: move |tag: FruitTag| last_event.set(format!("Added: {}", tag.name())),
                    on_remove: move |tag: FruitTag| last_event.set(format!("Removed: {}", tag.name())),
                    BasicUI { available: available }
                }

                // Callback event log
                div {
                    class: "mt-3 rounded-lg bg-slate-900/60 border border-slate-700/50 px-3 py-2 text-xs text-slate-400",
                    span { class: "text-slate-500 mr-1", "Last event:" }
                    "{last_event}"
                }

                // Screen-reader live region for status announcements
                div {
                    role: "status",
                    aria_live: "polite",
                    class: "absolute w-px h-px p-0 -m-px overflow-hidden [clip:rect(0,0,0,0)] whitespace-nowrap border-0",
                }

                // Keyboard shortcuts hint
                p {
                    class: "mt-3 text-xs text-slate-500",
                    span { class: "font-mono bg-slate-700/50 rounded px-1 py-0.5 mr-1", "\u{2190}\u{2192}" }
                    "pills  "
                    span { class: "font-mono bg-slate-700/50 rounded px-1 py-0.5 mr-1", "Enter" }
                    "select  "
                    span { class: "font-mono bg-slate-700/50 rounded px-1 py-0.5 mr-1", "Bksp" }
                    "remove  "
                    span { class: "font-mono bg-slate-700/50 rounded px-1 py-0.5 mr-1", "Esc" }
                    "close"
                }
            }
        }
    }
}

#[component]
fn BasicUI(available: Vec<FruitTag>) -> Element {
    let ctx = use_context::<TagInputState<FruitTag>>();

    rsx! {
        div { class: "relative",
            combo::Control::<FruitTag> {
                class: "flex flex-wrap items-center gap-2 rounded-xl border border-slate-600 bg-slate-900 px-3 py-2 focus-within:border-indigo-500 focus-within:ring-1 focus-within:ring-indigo-500/50 transition-all motion-reduce:transition-none",

                for (i, tag) in ctx.visible_tags.read().iter().cloned().enumerate() {
                    {
                        let is_locked = tag.is_locked();
                        let key = tag.id().to_string();
                        let name = tag.name().to_string();
                        rsx! {
                            combo::Tag {
                                key: "{key}",
                                tag: tag.clone(),
                                index: i,
                                class: "inline-flex items-center gap-1 rounded-lg bg-indigo-600/30 border border-indigo-500/40 px-2.5 py-0.5 text-sm text-indigo-200 transition-shadow motion-reduce:transition-none focus-visible:ring-2 focus-visible:ring-indigo-400 focus-visible:ring-offset-1 focus-visible:ring-offset-slate-900",
                                "{name}"
                                if is_locked {
                                    span { class: "ml-0.5 text-indigo-400/50 text-xs", "\u{1F512}" }
                                } else {
                                    combo::TagRemove {
                                        tag: tag.clone(),
                                        class: "ml-0.5 rounded hover:bg-indigo-500/30 px-1 transition-colors motion-reduce:transition-none",
                                    }
                                }
                            }
                        }
                    }
                }

                combo::Input::<FruitTag> {
                    class: "flex-1 min-w-[100px] bg-transparent outline-none text-slate-100 placeholder-slate-500 text-sm",
                }

                if *ctx.is_at_limit.read() {
                    span { class: "text-xs text-amber-400 ml-1", "(limit reached)" }
                }
            }

            // Dropdown with available tags
            combo::Dropdown {
                class: "absolute z-50 mt-1 w-full rounded-xl border border-slate-700 bg-slate-800 shadow-lg max-h-80 overflow-y-auto",

                combo::Empty {
                    class: "px-3 py-2 text-sm text-slate-500",
                    "No matching fruits."
                }
                for tag in &available {
                    combo::Item {
                        value: "{tag.id()}",
                        label: tag.name().to_string(),
                        class: "px-3 py-2 text-sm text-slate-200 cursor-pointer data-[highlighted]:bg-indigo-600/30 data-[state=checked]:text-indigo-300",
                        "{tag.name()}"
                    }
                }
            }
        }

        combo::LiveRegion::<FruitTag> {}
    }
}
