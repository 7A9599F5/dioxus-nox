# dioxus-nox-modal

Headless modal dialog primitive for Dioxus with focus trap, scroll lock, and ARIA.

## Features

- Compound component pattern: `modal::Root` / `modal::Overlay` / `modal::Content`
- Focus trap (Tab/Shift+Tab cycling within modal)
- Scroll lock (body overflow hidden when open)
- Background inert (siblings marked as `inert` when open)
- Configurable: ESC close, backdrop close, focus trap, scroll lock

## Usage

```rust,ignore
use dioxus::prelude::*;
use dioxus_nox_modal::*;

#[component]
fn App() -> Element {
    let handle = use_modal(false);
    rsx! {
        button { onclick: move |_| handle.show.call(()), "Open" }
        modal::Root {
            open: *handle.open.read(),
            on_close: move |_| handle.close.call(()),
            modal::Overlay {}
            modal::Content {
                h2 { "Dialog title" }
                button { onclick: move |_| handle.close.call(()), "Close" }
            }
        }
    }
}
```
