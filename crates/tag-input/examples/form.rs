use dioxus::document::Stylesheet;
use dioxus::prelude::*;
use dioxus_nox_tag_input::{use_tag_input, Tag};

fn main() {
    dioxus::launch(App);
}

fn skill_tags() -> Vec<Tag> {
    vec![
        Tag::new("rust", "Rust"),
        Tag::new("python", "Python"),
        Tag::new("typescript", "TypeScript"),
        Tag::new("go", "Go"),
        Tag::new("java", "Java"),
        Tag::new("csharp", "C#"),
        Tag::new("ruby", "Ruby"),
        Tag::new("elixir", "Elixir"),
    ]
}

fn interest_tags() -> Vec<Tag> {
    vec![
        Tag::new("gaming", "Gaming"),
        Tag::new("music", "Music"),
        Tag::new("cooking", "Cooking"),
        Tag::new("hiking", "Hiking"),
        Tag::new("reading", "Reading"),
        Tag::new("photography", "Photography"),
        Tag::new("travel", "Travel"),
        Tag::new("art", "Art"),
    ]
}

fn role_tags() -> Vec<Tag> {
    vec![
        Tag::new("frontend", "Frontend"),
        Tag::new("backend", "Backend"),
        Tag::new("fullstack", "Full Stack"),
        Tag::new("devops", "DevOps"),
        Tag::new("mobile", "Mobile"),
        Tag::new("data", "Data Engineer"),
    ]
}

#[component]
fn App() -> Element {
    let mut skills = use_tag_input(skill_tags(), vec![]);
    let mut interests = use_tag_input(interest_tags(), vec![]);
    let mut primary_role = use_tag_input(role_tags(), vec![]);
    let mut submitted = use_signal(|| false);
    let mut form_disabled = use_signal(|| false);

    // Validate skills: reject tag names shorter than 2 characters.
    use_hook(|| {
        skills.validate.set(Some(Callback::new(move |tag: Tag| {
            if tag.name.len() < 2 {
                Err("Tag name must be at least 2 characters.".into())
            } else {
                Ok(())
            }
        })));
    });

    // Primary role: select_mode with max_tags=1 (single-value select)
    use_hook(|| {
        primary_role.select_mode.set(true);
        primary_role.max_tags.set(Some(1));
    });

    rsx! {
        Stylesheet { href: asset!("/assets/tailwind.css") }

        div {
            class: "min-h-screen bg-slate-900 text-slate-100 flex items-start justify-center p-6 sm:p-10",

            div {
                class: "w-full max-w-lg",

                h1 {
                    class: "text-2xl font-bold mb-1 text-slate-50",
                    "Create Your Profile"
                }
                p {
                    class: "text-sm text-slate-400 mb-6",
                    "Add your skills and interests to get started."
                }

                div {
                    class: "space-y-5",

                    TagField {
                        label: "Skills",
                        state: skills,
                        accent: "emerald",
                        placeholder: "Add a skill\u{2026}",
                    }

                    TagField {
                        label: "Interests",
                        state: interests,
                        accent: "violet",
                        placeholder: "Add an interest\u{2026}",
                    }

                    TagField {
                        label: "Primary Role (single select)",
                        state: primary_role,
                        accent: "emerald",
                        placeholder: "Choose your primary role\u{2026}",
                    }

                    // Hidden form inputs with serialized tag IDs
                    input {
                        r#type: "hidden",
                        name: "skills",
                        value: "{skills.form_value}",
                    }
                    input {
                        r#type: "hidden",
                        name: "interests",
                        value: "{interests.form_value}",
                    }
                    input {
                        r#type: "hidden",
                        name: "primary_role",
                        value: "{primary_role.form_value}",
                    }

                    // Submit button
                    button {
                        r#type: "button",
                        class: "w-full rounded-xl bg-indigo-600 hover:bg-indigo-500 active:bg-indigo-700 text-white font-medium py-2.5 transition-colors disabled:opacity-50 disabled:cursor-not-allowed",
                        disabled: *form_disabled.read(),
                        onclick: move |_| {
                            submitted.set(true);
                            form_disabled.set(true);
                            skills.is_disabled.set(true);
                            interests.is_disabled.set(true);
                            primary_role.is_disabled.set(true);
                        },
                        "Save Profile"
                    }

                    // Reset button (only shown when disabled)
                    if *form_disabled.read() {
                        button {
                            r#type: "button",
                            class: "w-full rounded-xl bg-slate-700 hover:bg-slate-600 active:bg-slate-800 text-slate-300 font-medium py-2.5 transition-colors",
                            onclick: move |_| {
                                form_disabled.set(false);
                                skills.is_disabled.set(false);
                                interests.is_disabled.set(false);
                                primary_role.is_disabled.set(false);
                            },
                            "Reset Form"
                        }
                    }

                    // JSON output
                    if *submitted.read() {
                        div {
                            class: "rounded-xl border border-slate-700 bg-slate-800 p-4",

                            h3 {
                                class: "text-sm font-semibold text-slate-300 mb-2",
                                "Submitted data"
                            }

                            pre {
                                class: "text-xs text-slate-400 whitespace-pre-wrap break-words font-mono",
                                {format_json(&skills.selected_tags.read(), &interests.selected_tags.read(), &primary_role.selected_tags.read())}
                            }
                            div {
                                class: "mt-2 text-xs text-slate-500",
                                "form_value (skills): {skills.form_value}"
                            }
                            div {
                                class: "text-xs text-slate-500",
                                "form_value (role): {primary_role.form_value}"
                            }
                        }
                    }
                }
            }
        }
    }
}

fn format_json(skills: &[Tag], interests: &[Tag], role: &[Tag]) -> String {
    let sk: Vec<&str> = skills.iter().map(|t| t.name.as_str()).collect();
    let int: Vec<&str> = interests.iter().map(|t| t.name.as_str()).collect();
    let rl: Vec<&str> = role.iter().map(|t| t.name.as_str()).collect();
    format!(
        "{{\n  \"skills\": {:?},\n  \"interests\": {:?},\n  \"primary_role\": {:?}\n}}",
        sk, int, rl
    )
}

// ---------------------------------------------------------------------------
// Accent color theme — concrete class strings so Tailwind can scan them
// ---------------------------------------------------------------------------

struct AccentTheme {
    pill: &'static str,
    pill_hover: &'static str,
    focus_ring: &'static str,
}

const EMERALD: AccentTheme = AccentTheme {
    pill: "bg-emerald-600/25 border-emerald-500/40 text-emerald-200",
    pill_hover: "hover:bg-emerald-500/30",
    focus_ring:
        "focus-within:border-emerald-500 focus-within:ring-1 focus-within:ring-emerald-500/50",
};

const VIOLET: AccentTheme = AccentTheme {
    pill: "bg-violet-600/25 border-violet-500/40 text-violet-200",
    pill_hover: "hover:bg-violet-500/30",
    focus_ring:
        "focus-within:border-violet-500 focus-within:ring-1 focus-within:ring-violet-500/50",
};

fn accent_theme(name: &str) -> &'static AccentTheme {
    match name {
        "violet" => &VIOLET,
        _ => &EMERALD,
    }
}

// ---------------------------------------------------------------------------
// Reusable tag field component with configurable accent color
// ---------------------------------------------------------------------------

#[component]
fn TagField(
    label: String,
    mut state: dioxus_nox_tag_input::TagInputState<Tag>,
    accent: String,
    placeholder: String,
) -> Element {
    let theme = accent_theme(&accent);

    use_effect(move || {
        let count = state.filtered_suggestions.read().len();
        state.announce_suggestions(count);
    });

    rsx! {
        div {
            class: if *state.is_disabled.read() { "opacity-50 pointer-events-none" } else { "" },

            label {
                class: "block text-sm font-medium text-slate-300 mb-1.5",
                "{label}"
            }

            div {
                class: "relative",

                div {
                    class: "flex flex-wrap items-center gap-2 rounded-xl border border-slate-600 bg-slate-900 px-3 py-2 transition-all motion-reduce:transition-none {theme.focus_ring}",

                    for (i, tag) in state.selected_tags.read().iter().cloned().enumerate() {
                        {
                            let is_pill_active = (*state.active_pill.read()) == Some(i);
                            let pill_ring = if is_pill_active { "ring-2 ring-indigo-400" } else { "" };
                            rsx! {
                                span {
                                    key: "{tag.id}",
                                    id: state.pill_id(i),
                                    class: "inline-flex items-center gap-1 rounded-lg border px-2.5 py-0.5 text-sm transition-shadow motion-reduce:transition-none focus-visible:ring-2 focus-visible:ring-indigo-400 focus-visible:ring-offset-1 focus-visible:ring-offset-slate-900 {theme.pill} {pill_ring}",
                                    "{tag.name}"
                                    button {
                                        r#type: "button",
                                        class: "ml-0.5 rounded px-1 transition-colors motion-reduce:transition-none {theme.pill_hover}",
                                        onclick: move |_| state.remove_tag(&tag.id),
                                        "\u{00D7}"
                                    }
                                }
                            }
                        }
                    }

                    input {
                        r#type: "text",
                        role: "combobox",
                        aria_expanded: state.aria_expanded(),
                        aria_controls: state.listbox_id(),
                        aria_activedescendant: state.active_descendant(),
                        aria_autocomplete: "list",
                        disabled: *state.is_disabled.read(),
                        class: "flex-1 min-w-[100px] bg-transparent outline-none text-slate-100 placeholder-slate-500 text-sm",
                        placeholder: "{placeholder}",
                        value: "{state.search_query}",
                        oninput: move |evt| state.set_query(evt.value()),
                        onkeydown: move |evt| state.handle_keydown(evt),
                        onclick: move |_| state.handle_click(),
                        onfocus: move |_| state.is_dropdown_open.set(true),
                        onblur: move |_| state.close_dropdown(),
                        onpaste: move |evt: Event<ClipboardData>| {
                            if let Some(text) = dioxus_nox_tag_input::extract_clipboard_text(&evt) {
                                evt.prevent_default();
                                state.handle_paste(text);
                            }
                        },
                    }
                }

                // Validation error
                if let Some(err) = state.validation_error.read().as_ref() {
                    p {
                        class: "mt-1 text-xs text-red-400",
                        "{err}"
                    }
                }

                // Dropdown
                if *state.is_dropdown_open.read() && !state.filtered_suggestions.read().is_empty() {
                    div {
                        id: state.listbox_id(),
                        role: "listbox",
                        aria_multiselectable: "true",
                        class: "absolute z-50 mt-1 w-full rounded-xl border border-slate-700 bg-slate-800 shadow-lg max-h-80 overflow-y-auto",

                        for (i, suggestion) in state.filtered_suggestions.read().iter().cloned().enumerate() {
                            {
                                let is_active = *state.highlighted_index.read() == Some(i);
                                let bg = if is_active { "bg-indigo-600/80 text-white" } else { "" };
                                rsx! {
                                    div {
                                        key: "{suggestion.id}",
                                        id: state.suggestion_id(i),
                                        role: "option",
                                        aria_selected: if is_active { "true" } else { "false" },
                                        class: "px-3 py-2 text-sm cursor-pointer transition-colors motion-reduce:transition-none hover:bg-slate-700 {bg}",
                                        onmouseenter: move |_| state.highlighted_index.set(Some(i)),
                                        onmousedown: move |evt: Event<MouseData>| {
                                            evt.prevent_default();
                                            state.add_tag(suggestion.clone());
                                        },
                                        "{suggestion.name}"
                                    }
                                }
                            }
                        }
                    }
                }

                div {
                    role: "status",
                    aria_live: "polite",
                    class: "absolute w-px h-px p-0 -m-px overflow-hidden [clip:rect(0,0,0,0)] whitespace-nowrap border-0",
                    "{state.status_message.read()}"
                }
            }
        }
    }
}
