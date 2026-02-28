# dioxus-nox Workspace

Headless component library for Dioxus 0.7 (Rust/WASM). 10 crates in `crates/`.

## Crate Inventory

| Crate | Path | Version | Purpose |
|---|---|---|---|
| `dioxus-nox-cmdk` | `crates/cmdk` | 0.13.0 | Headless command palette primitive |
| `dioxus-nox-virtualize` | `crates/virtualize` | 0.1.0 | Virtual list viewport math (helper for cmdk) |
| `dioxus-nox-dnd` | `crates/dnd` | 0.2.0 | Composable drag-and-drop |
| `dioxus-nox-extensions` | `crates/extensions` | 0.1.0 | Runtime plugin system (Extension trait) |
| `dioxus-nox-gestures` | `crates/gestures` | 0.1.0 | Touch gesture primitives (swipe, long-press) |
| `dioxus-nox-preview` | `crates/preview` | 0.1.0 | Debounced preview hook + LRU cache |
| `dioxus-nox-shell` | `crates/shell` | 0.2.0 | Application shell layout primitive |
| `dioxus-nox-suggest` | `crates/suggest` | 0.1.0 | Headless inline-trigger suggestion primitive (slash, @mentions, #hashtags) |
| `dioxus-nox-tag-input` | `crates/tag-input` | 0.1.0 | Headless tag/multi-select input |
| `dioxus-nox-markdown` | `crates/markdown` | 0.1.0 | Headless markdown editor/previewer/display |

@crates/cmdk/CLAUDE.md
@crates/virtualize/CLAUDE.md
@crates/extensions/CLAUDE.md
@crates/gestures/CLAUDE.md
@crates/preview/CLAUDE.md
@crates/shell/CLAUDE.md
@crates/dnd/CLAUDE.md
@crates/suggest/CLAUDE.md
@crates/tag-input/CLAUDE.md
@crates/markdown/CLAUDE.md

### Cross-Crate Relationships

- `cmdk` optionally depends on `virtualize` (feature `"virtualize"`)
- `shell` dev-depends on `cmdk` (examples only; not a runtime dep)
- `tag-input` dev-depends on `dnd` (sortable pills example only)
- All others are standalone

### Workspace Cargo Commands

```bash
# Check all crates (native target)
cargo check --workspace

# Run all tests
cargo test --workspace

# Lint all crates
cargo clippy --workspace -- -D warnings

# WASM lint (catches web-sys / wasm-bindgen misuse)
cargo clippy --workspace --target wasm32-unknown-unknown -- -D warnings

# Target a specific crate
cargo check -p dioxus-nox-dnd
cargo test -p dioxus-nox-cmdk
```

---

## Dioxus 0.7 Gotchas

Applies to every crate in this workspace.

- **Signal field access in RSX:** `(ctx.field)()` not `ctx.field()` — parentheses disambiguate field from method call
- **`use_drop` not in prelude:** `use dioxus_core::use_drop;`
- **Optional context:** `try_use_context::<T>()` (returns `Option`)
- **Hooks are unconditional:** never call hooks conditionally — move flags inside the hook closure
- **Global event listeners:** `Closure::forget()` is correct for app-lifetime listeners (wasm)
- **Function-pointer props:** need a newtype wrapper with `PartialEq` returning `false`
- **Async focus:** `MountedData::set_focus(bool)` is async — wrap in `spawn(async { ... })`
- **Signal mutation from `&self`:** `ctx.field.set(v)` fails ("cannot borrow as mutable") — shadow first: `let mut f = ctx.field; f.set(v);`
- **`use_effect` subscriptions:** read every signal the effect depends on *before* any early-return guard, or it won't re-run on that signal's change
- **Signal transition detection:** use `Rc<Cell<bool>>` via `use_hook` to track previous value (avoids extra reactive subscription)
- **Edition 2024 let-chains:** `if a { if let Some(x) = b { ... } }` → `if a && let Some(x) = b { ... }` (satisfies clippy `collapsible_if`)
- **`ReadOnlySignal` deprecated:** use `ReadSignal` instead
- **Signal borrow gotcha:** `ctx.active_signal().read()` fails — the temporary signal is freed before the read guard. Bind first: `let sig = ctx.active_signal(); let val = sig.read();`
- **Signal `.set()` in closures/effects:** `Signal<T>` is `Copy` but `.set()` requires `mut` — shadow inside the closure: `let mut s = my_sig; s.set(v);`
- **AP-3 (signal write in render body):** bare `sig.set()` in the component body causes re-render loops — always wrap in `use_effect(move || { let mut s = sig; s.set(v); });`
- **Boolean data attributes:** `"data-foo": if cond { "true" } else { "" }` — empty string is still *present* in DOM so `[data-foo]` CSS matches everything. Use `.then_some("true")` to make the attribute absent when false.

---

## Component Architecture: Radix Primitives Pattern

All components must conform to the Radix Primitives pattern (reference: https://github.com/DioxusLabs/components).
These are non-negotiable conventions.

### Platform Targets
Every component must compile and function correctly on **all Dioxus render targets**: Web (WASM), Desktop, iOS, Android. No browser-only assumptions.

### 1. Compound Composition (Module Namespacing)
Decompose every component into named sub-parts inside a module. Consumers assemble parts explicitly.
```rust
use my_lib::accordion;
rsx! { accordion::Root { accordion::Item { accordion::Trigger {} accordion::Content {} } } }
```
Common part names: `Root`, `Item`, `Trigger`, `Content`, `Label`, `Indicator`, `Thumb`, `Track`, `Portal`, `Overlay`.

### 2. Zero Visual Styles
Ship **no** colors, fonts, spacing, borders, shadows, animations, or layout opinions. Inline styles only for runtime-computed values (e.g., `left`/`top` from pointer events). Classify every inline style as `FUNCTIONAL` (keep) or `VISUAL`/`LAYOUT` (remove).

### 3. State via Data Attributes
Communicate internal state via `data-*` attributes, never via prop-driven conditional class strings.
`data-state="open"/"closed"`, `data-disabled`, `data-orientation`, `data-highlighted`, `data-selected`.

### 4. Class Prop on Every Part
Every sub-component accepts `class: Option<String>` on its root DOM element.

### 5. Uncontrolled Default, Optionally Controllable
Work out of the box with internal state. Consumers can optionally control via `value` + `on_value_change`.

### 6. Accessibility (WAI-ARIA)
Correct ARIA roles, attributes, and keyboard handling for every interactive component. All interactive elements reachable via Tab; Escape closes overlays; Arrow keys for group navigation.

### 7. Context for Compound State Sharing
Use `use_context`/`provide_context` to share state between compound parts. Never global signals or prop drilling across more than one level.

### 8. Children / Slot Pattern
Parts that wrap consumer content use `children: Element`. The component provides behavior; the consumer controls content.

---

## web_sys / js_sys Usage Policy

**Prefer native Dioxus 0.7 APIs over raw web platform bindings at all times.**

`web_sys` and `js_sys` are browser-only — any call will fail on Desktop/iOS/Android.

Before using them: search Dioxus 0.7 docs, dioxus-primitives source, Context7/Perplexity for a native alternative. Only use if no native equivalent exists.

### Common Substitutions

| web_sys / js_sys usage | Dioxus-native alternative |
|---|---|
| `window().add_event_listener_*` | `onmousedown`, `onkeydown`, etc. in RSX; `use_global_event` |
| `document().get_element_by_id` | Signals, context, component refs |
| Manual focus via DOM | `onmounted` + `MountedData::set_focus` |
| Element dimensions | `onmounted` + `MountedData::get_client_rect` |
| `setTimeout` / `setInterval` | `use_future`, `spawn`, `async_std::task::sleep` |
| `gloo-timers` callback/timeout on non-WASM | Gate import `#[cfg(target_arch = "wasm32")]`; split timer type `Option<Timeout>` / `Option<()>`; fire immediately on native. Add `let _ = delay_ms;` in the non-WASM branch to suppress unused-variable lint. |

If you must use `web_sys`/`js_sys`:
- Gate behind `#[cfg(target_arch = "wasm32")]`
- Provide a non-web fallback or graceful no-op
- Leave a comment citing the search confirming no native alternative:
```rust
// web_sys used here: confirmed no Dioxus 0.7 native API for X as of YYYY-MM-DD.
// Source: [link or query]
// Non-WASM targets: [fallback description]
#[cfg(target_arch = "wasm32")]
{ /* web_sys call */ }
```

---

## Shared Conventions

- **Edition:** 2024 (`edition.workspace = true` in all crates)
- **Dioxus pin:** `dioxus = "=0.7.3"` via `workspace.dependencies`
- **Components:** `#[component]` macro for all components; `#[props(default)]` / `#[props(into)]` for props
- **Visibility:** `pub(crate)` for internal helpers, `pub` only for user-facing API
- **Commits:** conventional commits — `feat(scope):`, `fix(scope):`, `test(scope):`, `docs(scope):`
- **gen keyword:** `gen` is reserved in Edition 2024 — rename any `gen` variables (e.g., to `measure_gen`)
