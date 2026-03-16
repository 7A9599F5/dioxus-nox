# dioxus-nox-master-detail

Headless adaptive master-detail layout for Dioxus.

## Features

- Compound component pattern: `master_detail::Root` / `Master` / `Detail` / `Backdrop`
- Pure CSS responsive via data attributes (no JS breakpoint detection)
- `data-detail="open|closed"` for CSS targeting

## Usage

```rust,ignore
use dioxus::prelude::*;
use dioxus_nox_master_detail::*;

#[component]
fn App() -> Element {
    let mut open = use_signal(|| false);
    rsx! {
        master_detail::Root {
            detail_open: *open.read(),
            on_detail_close: move |_| open.set(false),
            master_detail::Master { /* list content */ }
            master_detail::Detail { /* detail content */ }
            master_detail::Backdrop {}
        }
    }
}
```
