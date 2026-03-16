# dioxus-nox-drawer

Headless drawer/sheet primitive for Dioxus that slides from any edge.

## Features

- Compound component pattern: `drawer::Root` / `drawer::Overlay` / `drawer::Content`
- Four sides: Left, Right (default), Bottom, Top
- Focus trap, scroll lock, background inert
- Configurable: ESC close, overlay close

## Usage

```rust,ignore
use dioxus::prelude::*;
use dioxus_nox_drawer::*;

#[component]
fn App() -> Element {
    let mut open = use_signal(|| false);
    rsx! {
        button { onclick: move |_| open.set(true), "Open" }
        drawer::Root {
            open: *open.read(),
            on_close: move |_| open.set(false),
            side: DrawerSide::Bottom,
            drawer::Overlay {}
            drawer::Content {
                h2 { "Sheet content" }
            }
        }
    }
}
```
