# dioxus-nox-timer

Headless countdown and stopwatch timer hooks for Dioxus.

## Features

- **Wall-clock based** — survives browser tab backgrounding (no `setInterval` drift)
- **Cross-platform** — works on web (wasm32), desktop, iOS, and Android
- **Headless** — no rendered components, just hooks and state machines

## Usage

```rust,ignore
use dioxus::prelude::*;
use dioxus_nox_timer::*;

#[component]
fn Timer() -> Element {
    let (remaining, state, controls) = use_countdown(None);

    rsx! {
        div {
            role: "timer",
            aria_live: "polite",
            p { "{format_duration(*remaining.read())}" }
            button { onclick: move |_| controls.start.call(90), "Start 90s" }
        }
    }
}
```

## ARIA Recommendations

Consumers should apply:
- `role="timer"` on the display element
- `aria-live="polite"` for screen reader announcements
- `data-timer-state="idle|running|paused|complete"` for CSS styling
