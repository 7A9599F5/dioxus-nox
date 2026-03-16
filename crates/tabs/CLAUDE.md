# dioxus-nox-tabs — Headless tab management primitives

> See workspace `CLAUDE.md` for Dioxus 0.7 gotchas, Radix conventions, web_sys policy.

## Purpose
Headless tab management following the WAI-ARIA Tabs pattern. Compound components (`Root`, `List`, `Trigger`, `Content`) with proper ARIA roles, keyboard navigation, and data attributes for styling — shipping zero visual styles.

## Module Structure
- `lib.rs` — re-exports; `tabs` compound namespace
- `types.rs` — `Orientation`, `ActivationMode`, `TabsContext`, `TabEntry`, `navigate()` (pure fn)
- `components.rs` — `Root`, `List`, `Trigger`, `Content`, ID helpers
- `tests.rs` — unit tests (navigation logic, context operations)

## Key Design Decisions
1. Self-registering pattern: `Trigger` registers via `use_hook` on mount, `use_drop` on unmount
2. Uncontrolled default, optionally controllable via `value: Signal<String>` + `on_value_change`
3. `TabsContext` is `Copy` (all fields are Signal) — accessed via `use_context`/`consume_context`
4. JS focus via `document::eval` gated behind `#[cfg(target_arch = "wasm32")]`
5. Navigation is a pure function (`navigate()`) — testable without Dioxus runtime
6. `close()` picks next non-disabled neighbour; consumer owns unmounting

## Data Attributes
| Attribute | Component | Values |
|---|---|---|
| `data-tabs-orientation` | Root, List | `"horizontal"` \| `"vertical"` |
| `data-state` | Trigger, Content | `"active"` \| `"inactive"` |
| `data-disabled` | Trigger | `"true"` (present when disabled) |

## CI
```bash
cargo check -p dioxus-nox-tabs
cargo test -p dioxus-nox-tabs
cargo clippy -p dioxus-nox-tabs --target wasm32-unknown-unknown -- -D warnings
```
