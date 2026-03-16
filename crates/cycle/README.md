# dioxus-nox-cycle

Generic value cycling hook for Dioxus.

Wraps around at both ends. Useful for set type toggles, status cycles, priority selectors.

## Usage

```rust,ignore
use dioxus_nox_cycle::*;

let cycle = use_cycle(&["Low", "Medium", "High"], None);
cycle.next.call(()); // Low → Medium → High → Low
cycle.previous.call(()); // wraps backward
```
