# dioxus-nox-shell

Application shell layout primitive for Dioxus.
See workspace `CLAUDE.md` for Dioxus 0.7 gotchas, Radix pattern, web_sys policy, and shared conventions.

## Crate Purpose

Provides `AppShell` — a persistent, always-visible split-pane layout with named slots
(sidebar, children, preview, footer, tabs, sheet, modal, fab, search). Headless: all layout via `data-shell*` attributes.
Standalone — zero runtime dependency on dioxus-nox-cmdk (cmdk is a dev-dep for examples only).

## Public API Surface

- `AppShell` component props: `children`, `sidebar?`, `preview?`, `footer?`, `tabs?`, `sheet?`, `modal?`, `fab?`, `search?`, `layout`, `mobile_sidebar`, `desktop_sidebar`, `compact_below`, `expanded_above`, `class?`, `sidebar_role`, `preview_label`
- `ShellLayout` enum: `Horizontal` (default), `Vertical`, `Sidebar`
- `ShellBreakpoint` enum: `Compact`, `Medium`, `Expanded` (default) — `is_compact()`, `is_mobile()`
- `MobileSidebar` enum: `Drawer` (default), `Rail`, `Hidden`
- `DesktopSidebar` enum: `Full` (default), `Rail`, `Expandable`
- `SheetSnap` enum: `Hidden` (default), `Peek`, `Half`, `Full` — `as_str()`, `is_visible()`
- `ShellContext`: signals for layout, breakpoint, sidebar state, modal, search, sheet, stack depth
  - `toggle_sidebar()`, `sidebar_state() -> &'static str`, `push_stack()` / `pop_stack()` / `can_go_back()`
  - `open_modal()` / `close_modal()`, `open_search()` / `close_search()`, `set_sheet_snap()`
- `use_shell_context() -> ShellContext`
- `use_shell_breakpoint(compact_below: f64, expanded_above: f64) -> ReadSignal<ShellBreakpoint>`

## Data Attributes (27 total)

| Attribute | Element | Values |
|---|---|---|
| `data-shell` | Root | presence |
| `data-shell-layout` | Root | `"horizontal"`, `"vertical"`, `"sidebar"` |
| `data-shell-breakpoint` | Root | `"compact"`, `"medium"`, `"expanded"` |
| `data-shell-sidebar-state` | Root | `"expanded"`, `"collapsed"`, `"open"`, `"closed"` |
| `data-shell-sidebar` | Sidebar slot | presence |
| `data-shell-sidebar-visible` | Sidebar (desktop) | `"true"`, `"false"` |
| `data-shell-sidebar-mobile` | Sidebar (mobile) | `"true"` |
| `data-shell-sidebar-variant` | Sidebar (mobile) | `"drawer"`, `"rail"` |
| `data-shell-columns` | Root | `"1"`, `"2"` |
| `data-shell-display-mode` | Root | `"stack"`, `"side-by-side"` |
| `data-shell-stack-depth` | Root | `"1"`, `"2"`, … |
| `data-shell-can-go-back` | Root | `"true"`, `"false"` |
| `data-shell-search-active` | Root | `"true"`, `"false"` |
| `data-shell-modal-state` | Root | `"presented"`, `"dismissed"` |
| `data-shell-content` | Main content | presence |
| `data-shell-preview` | Preview pane | presence |
| `data-shell-footer` | Footer | presence |
| `data-shell-tabs` | Tab bar slot | presence |
| `data-shell-sheet` | Bottom sheet slot | presence |
| `data-shell-sheet-state` | Bottom sheet | `"hidden"`, `"peek"`, `"half"`, `"full"` |
| `data-shell-fab` | FAB slot | presence |
| `data-shell-search` | Search overlay slot | presence |
| `data-shell-modal` | Modal slot | presence |

## Module Structure

- `lib.rs` — re-exports only
- `shell.rs` — `AppShell` component, `ShellLayout` enum
- `context.rs` — `ShellContext` struct, `use_shell_context` hook
- `breakpoint.rs` — `ShellBreakpoint`, `MobileSidebar`, `DesktopSidebar`, `SheetSnap`, `use_shell_breakpoint`
- `tests.rs` — unit tests (pure logic, no component rendering)

## Key Design Decisions

- **OQ-1:** Standalone crate (not cmdk-internal).
- **OQ-2:** No `role="application"` by default; expose `sidebar_role: Option<&'static str>` prop.
- **OQ-3:** Preview slot outside CommandRoot tree — consumers lift state via Signal.
- **OQ-4:** `sidebar_visible` always defaults to `true`.
- **OQ-5:** Simple 3-variant `Copy` enum for ShellLayout.
- **OQ-6:** Desktop sidebar always stays in DOM (CSS controls via `data-shell-sidebar-visible`) for smooth CSS transitions.
- **OQ-7:** Breakpoint detection via `document::eval` matchMedia — all WebView targets supported (wasm, Wry, iOS, Android).

## Crate-Specific Conventions

- Pure RSX — ZERO web-sys, ZERO wasm-bindgen, ZERO `cfg(wasm32)` guards
- `document::eval()` for JS interop (all WebView targets); not supported on Blitz/native
- Dual-state sidebar: desktop uses `sidebar_visible` (CSS width transitions, stays in DOM); mobile uses `sidebar_mobile_open` (separate tree node per `MobileSidebar` variant)

## CI Commands

```bash
cargo test
cargo clippy -- -D warnings
cargo clippy --target wasm32-unknown-unknown -- -D warnings
```
