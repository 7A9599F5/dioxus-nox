# dioxus-nox-inline-confirm

Headless inline confirmation pattern for Dioxus.

Replaces a destructive action trigger with confirm/cancel buttons inline. Two-phase: Idle shows trigger, Confirming shows confirm/cancel.

## Usage

```rust,ignore
use dioxus_nox_inline_confirm::*;

let handle = use_inline_confirm(Some(5000)); // auto-cancel 5s
// handle.request.call(()); → Idle → Confirming
// handle.confirm.call(()); → fires on_confirm
// handle.cancel.call(()); → back to Idle
```
