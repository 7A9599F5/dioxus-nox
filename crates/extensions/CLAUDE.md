# dioxus-nox-extensions

Runtime plugin system for Dioxus applications.
See workspace `CLAUDE.md` for Dioxus 0.7 gotchas, Radix pattern, web_sys policy, and shared conventions.

## Crate Purpose

Defines an `Extension` trait for contributing commands/items to a host Dioxus app's dynamic list at runtime.
Callback-based integration — zero compile-time dependency on dioxus-cmdk. Any Dioxus app can wire in `use_extensions`.

## Public API Surface

- `Extension` trait: `id() -> &str`, `name() -> &str`, `commands() -> Vec<PluginCommand>`, `on_activate()`, `on_deactivate()`
- `PluginCommand` struct: `id`, `label`, `keywords`, `group`, `on_select: Rc<dyn Fn(String)>`
- `ExtensionInfo` struct: `id`, `name`, `command_count`
- `ExtensionItemRegistration` struct (bridge payload): `id`, `label`, `keywords`, `group`, `on_select`, `ext_id`
- `ExtensionHandle`: `register(ext: Box<dyn Extension>)`, `unregister(ext_id: &str)`, `extensions() -> ReadOnlySignal<Vec<ExtensionInfo>>`
- `use_extensions(register_cb: impl Fn(ExtensionItemRegistration), unregister_cb: impl Fn(&str)) -> ExtensionHandle`

## Module Structure

- `lib.rs` — re-exports only
- `extension.rs` — `Extension` trait, `PluginCommand`, `ExtensionInfo`
- `handle.rs` — `ExtensionHandle`, `use_extensions`, `ExtensionItemRegistration`
- `registry.rs` — `ExtensionRegistry` (pub(crate) — internal state management)
- `tests.rs` — unit tests (pure Rust, registry-layer, no Dioxus runtime required)

## Key Design Decisions

- **OQ-1:** Callback-based (dependency inversion). Never direct CommandContext access.
- **OQ-2:** `register()` returns `Result<(), ExtensionError>`; on duplicate ID: unregister old, register new.
- **OQ-3:** `Rc<dyn Fn(String)>` for on_select (wasm32 single-threaded).
- **OQ-4:** No shortcut field in v0.1 (deferred to v0.2).
- **OQ-5:** No ExtensionGroup in v0.1; extensions declare group via `PluginCommand.group`.

## Crate-Specific Conventions

- `Box<dyn Extension>` requires `'static` bound — extension state must be owned
- `HashMap<String, ExtensionEntry>` + `Vec<String>` for order in `ExtensionRegistry`
- ZERO web-sys/js-sys/wasm-bindgen calls. Pure Rust. Works on all targets.

## CI Commands

```bash
cargo test
cargo clippy -- -D warnings
cargo clippy --target wasm32-unknown-unknown -- -D warnings
```
