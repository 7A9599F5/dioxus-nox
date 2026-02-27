# dioxus-nox-extensions — Runtime plugin system (Extension trait)

> See workspace `CLAUDE.md` for Dioxus 0.7 gotchas, Radix conventions, web_sys policy.

## Purpose
Callback-based runtime plugin system. Extensions contribute commands/items to a host Dioxus app's dynamic list at runtime. Zero compile-time dependency on dioxus-cmdk — any Dioxus app can wire in `use_extensions`. Pure Rust, works on all targets.

## Module Structure
- `lib.rs` — re-exports only
- `extension.rs` — `Extension` trait, `PluginCommand`, `ExtensionInfo`
- `handle.rs` — `ExtensionHandle`, `use_extensions`, `ExtensionItemRegistration`
- `registry.rs` — `ExtensionRegistry` (pub(crate) — internal state management)
- `tests.rs` — unit tests (pure Rust, no Dioxus runtime required)

## Key Design Decisions
1. Callback-based (dependency inversion) — never direct `CommandContext` access
2. Duplicate ID on `register()`: unregisters old, registers new (returns `Result<(), ExtensionError>`)
3. `Rc<dyn Fn(String)>` for `on_select` — wasm32 single-threaded constraint

## Further Reading
Detailed context in `.context/` — read on demand:
- `api.md` — full API surface, registry internals, all design decisions

## CI
```bash
cargo check -p dioxus-nox-extensions
cargo test -p dioxus-nox-extensions
cargo clippy -p dioxus-nox-extensions --target wasm32-unknown-unknown -- -D warnings
```
