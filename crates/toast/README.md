# dioxus-nox-toast

Headless toast notification system for Dioxus with queue management and auto-dismiss.

## Features

- Generic `Toast<T>` — consumer defines toast data type
- FIFO queue with configurable max (default 3)
- Auto-dismiss via wall-clock countdown
- Undo support (`show_undoable`)
- Toast IDs: default u64 counter, optional `uuid` feature

## Usage

```rust,ignore
use dioxus::prelude::*;
use dioxus_nox_toast::*;

#[component]
fn App() -> Element {
    let mut mgr = use_toast_manager::<String>(3);
    rsx! {
        button {
            onclick: move |_| mgr.show("Saved!".into(), std::time::Duration::from_secs(3)),
            "Save"
        }
        ToastViewport { render_toast: |t: Toast<String>| rsx! { div { "{t.data}" } } }
    }
}
```
