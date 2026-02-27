# dioxus-nox-shell — Application shell layout primitive

> See workspace `CLAUDE.md` for Dioxus 0.7 gotchas, Radix conventions, web_sys policy.

## Purpose
Provides `AppShell` — a persistent, always-visible split-pane layout with 9 named slots (sidebar, children, preview, footer, tabs, sheet, modal, fab, search). All layout via `data-shell*` attributes. Standalone — zero runtime dependency on dioxus-nox-cmdk.

## Module Structure
- `lib.rs` — re-exports only
- `shell.rs` — `AppShell` component, `ShellLayout` enum
- `context.rs` — `ShellContext` struct, `use_shell_context` hook
- `breakpoint.rs` — `ShellBreakpoint`, `MobileSidebar`, `DesktopSidebar`, `SheetSnap`, `use_shell_breakpoint`
- `tests.rs` — unit tests (pure logic, no component rendering)

## Key Design Decisions
1. Desktop sidebar always stays in DOM (CSS controls via `data-shell-sidebar-visible`) for smooth CSS transitions (OQ-6)
2. Mobile sidebar is a separate tree node per `MobileSidebar` variant; desktop is width-transitioned
3. Breakpoint detection via `document::eval` matchMedia — all WebView targets; not Blitz/native (OQ-7)

## Further Reading
Detailed context in `.context/` — read on demand:
- `architecture.md` — full slot system, ShellContext methods, all 7 design decisions
- `data-attributes.md` — all 27 `data-shell*` attributes with element and value reference
- `breakpoint.md` — dual sidebar pattern, mobile tree rebuild vs desktop CSS transitions

## CI
```bash
cargo check -p dioxus-nox-shell
cargo test -p dioxus-nox-shell
cargo clippy -p dioxus-nox-shell --target wasm32-unknown-unknown -- -D warnings
```
