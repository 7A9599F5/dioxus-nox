# dioxus-nox Extraction Implementation Plan

## Context

The basalt-fitness application contains several UI components that are generic enough to extract into the dioxus-nox headless component library. The extraction opportunities are documented in `NOX_EXTRACTION_OPPORTUNITIES.md.txt`. This plan covers all four extraction waves: 9 new crates and 2 enhancements to existing crates.

**Guiding principle:** Every extracted crate ships state machines, ARIA attributes, and `data-*` hooks only — zero visual styles.

## Overview

| Wave | Crates | Est. Sessions | Dependencies |
|------|--------|---------------|--------------|
| **1 — Immediate** | `dioxus-nox-timer`, `dioxus-nox-modal`, `dioxus-nox-drawer` | 3 | modal & drawer share internal focus-trap/scroll-lock |
| **2 — Short-term** | `dioxus-nox-toast`, `dioxus-nox-master-detail` | 2 | None |
| **3 — Primitives** | `dioxus-nox-cycle`, `dioxus-nox-inline-confirm`, `dioxus-nox-toggle-group` | 2 | None |
| **4 — Enhancements** | shell bottom-bar, dnd reorder buttons, `dioxus-nox-password-strength` | 2 | Modify `dioxus-nox-shell` and `dioxus-nox-dnd` |

## Conventions (all crates)

Follow existing patterns from `dioxus-nox-tabs`, `dioxus-nox-shell`, etc.:
- **Cargo.toml**: `version.workspace = true`, `edition.workspace = true`, dep `dioxus = { workspace = true }`
- **lib.rs**: AI disclaimer docstring, module declarations, public re-exports, `mod tests` behind `#[cfg(test)]`
- **Compound components**: Root provides context via `use_context_provider()`, children consume via `use_context()`
- **Data attributes** for CSS targeting: `data-state`, `data-side`, etc.
- **Wasm conditional**: `#[cfg(target_arch = "wasm32")]` for DOM manipulation, no-op stubs otherwise
- **Workspace registration**: Add to `Cargo.toml` `[workspace.members]` and `[workspace.dependencies]`
- **Umbrella crate**: Add feature-gated re-export in `crates/dioxus-nox/src/lib.rs` + prelude entry

### Files to modify for every new crate
- `/home/user/dioxus-nox/Cargo.toml` — add to `members` list and `[workspace.dependencies]`
- `/home/user/dioxus-nox/crates/dioxus-nox/Cargo.toml` — add optional dep + feature flag
- `/home/user/dioxus-nox/crates/dioxus-nox/src/lib.rs` — add `#[cfg(feature = "X")]` re-export + prelude

---

## Shared Internal Utilities

Focus trap and scroll lock are needed by both `dioxus-nox-modal` and `dioxus-nox-drawer`. Rather than duplicating, create a shared internal crate.

### Plan: `dioxus-nox-core`

**Rationale:** cmdk already has focus trap (`crates/cmdk/src/helpers.rs:195-238`) and inert sibling management (`helpers.rs:122-192`). Extract these into a shared private crate so modal, drawer, and cmdk can all depend on it.

**File tree:**
```
crates/core/
├── Cargo.toml              — deps: dioxus, web-sys, wasm-bindgen
├── src/
│   ├── lib.rs              — module declarations, re-exports
│   ├── focus_trap.rs       — get_focusable_elements_in_container(), cycle_focus_forward/backward()
│   ├── scroll_lock.rs      — lock_body_scroll(), unlock_body_scroll() via overflow:hidden on body
│   ├── inert.rs            — set_siblings_inert() (moved from cmdk/helpers.rs)
│   └── tests.rs            — unit tests for pure logic portions
```

**API surface:**
- `pub fn get_focusable_elements_in_container(container_id: &str) -> Option<Vec<web_sys::HtmlElement>>` (wasm) / `Option<Vec<()>>` (non-wasm)
- `pub fn cycle_focus(container_id: &str, forward: bool)` — Tab/Shift+Tab cycling
- `pub fn set_siblings_inert(root_id: &str, inert: bool)` — background inert management
- `pub fn lock_body_scroll()` / `pub fn unlock_body_scroll()` — body overflow toggling

**Migration:** After creating this crate, update `dioxus-nox-cmdk` to depend on `dioxus-nox-core` and remove the duplicated helpers from `cmdk/src/helpers.rs`.

---

## Wave 1 — Immediate

### 1.1 `dioxus-nox-timer`

**Action:** NEW CRATE

**Source files to read:**
- `basalt-fitness/crates/basalt-ui/src/hooks/use_rest_timer.rs` — countdown state machine
- `basalt-fitness/crates/basalt-ui/src/components/workout/elapsed_time.rs` — stopwatch/elapsed time

**File tree:**
```
crates/timer/
├── Cargo.toml              — deps: dioxus; wasm target: gloo-timers, wasm-bindgen, js-sys
├── README.md               — usage examples for countdown + stopwatch
├── src/
│   ├── lib.rs              — module declarations, re-exports of TimerState, use_countdown, use_stopwatch, format_duration
│   ├── types.rs            — TimerState enum, CountdownControls struct
│   ├── time.rs             — cross-platform wall-clock abstraction (now_ms(), sleep_ms())
│   ├── countdown.rs        — use_countdown() hook implementation
│   ├── stopwatch.rs        — use_stopwatch() hook implementation
│   ├── format.rs           — format_duration() pure function
│   └── tests.rs            — state transition tests, format_duration tests
```

**Cross-platform time strategy:**
- Use raw i64 millisecond timestamps internally (no chrono dependency)
- `time.rs` provides `now_ms() -> i64`:
  - **wasm32**: `js_sys::Date::now() as i64` (wall-clock, survives tab backgrounding)
  - **non-wasm** (desktop/iOS/Android): `std::time::SystemTime::now().duration_since(UNIX_EPOCH).as_millis() as i64`
- Tick loop via Dioxus `spawn()` + platform-appropriate sleep:
  - **wasm32**: `gloo_timers::future::TimeoutFuture` (already a workspace dep)
  - **non-wasm**: `tokio::time::sleep` (provided by Dioxus runtime)
- Stopwatch API uses `i64` (epoch ms) instead of `DateTime<Utc>` to avoid chrono dep

**Stripping checklist:**
- Remove `audio_feedback()` and `haptic_feedback()` calls → consumer uses `on_complete` callback
- Remove `rest_timer` naming → generic `countdown` naming
- Remove `RestTimerState` → `TimerState`
- Remove basalt-specific imports (`use crate::hooks::*`, workout domain types)
- Remove any hardcoded durations (90s, 120s rest defaults)

**Generalization steps:**
- `use_rest_timer(duration, on_complete, on_tick)` → `use_countdown(on_complete: Option<Callback<()>>)` returning `(Signal<i64>, Signal<TimerState>, CountdownControls)`
- `CountdownControls` struct: `start: Callback<i64>`, `pause/resume/skip/adjust/dismiss: Callback<_>`
- `use_stopwatch(started_at_ms: i64, ended_at_ms: Option<i64>) -> Signal<i64>` (epoch milliseconds)
- Wall-clock based: store `end_time_ms` and `paused_remaining_ms` as signals, compute remaining via `now_ms()` diff
- Async tick loop: 100ms for countdown (smooth updates), 1000ms for stopwatch

**ARIA contract:**
- Timer crate is hook-only (no rendered components), so no ARIA attributes emitted directly
- Document that consumers should use `role="timer"` and `aria-live="polite"` on their display element
- `data-timer-state="idle|running|paused|complete"` recommended attribute pattern in README

**Test surface:**
- `TimerState` transitions: Idle→Running, Running→Paused, Paused→Running, Running→Complete, Complete→Idle
- `format_duration`: 0→"0:00", 65→"1:05", 3661→"1:01:01", negative→"0:00"
- `CountdownControls.adjust`: clamp to >= 0
- Wraparound: skip from Running → Idle, dismiss from Complete → Idle

---

### 1.2 `dioxus-nox-modal`

**Action:** NEW CRATE

**Source files to read:**
- `basalt-fitness/crates/basalt-ui/src/components/ui/modal.rs` — dialog component

**File tree:**
```
crates/modal/
├── Cargo.toml              — deps: dioxus, dioxus-nox-core
├── README.md               — usage examples with compound component pattern
├── src/
│   ├── lib.rs              — module declarations, re-exports, `pub mod modal { ... }` namespace
│   ├── types.rs            — ModalContext struct, ModalHandle struct
│   ├── hook.rs             — use_modal() hook
│   ├── components.rs       — ModalRoot, ModalOverlay, ModalContent compound components
│   └── tests.rs            — state transition tests, context propagation
```

**Stripping checklist:**
- Remove all Tailwind classes (backdrop opacity, rounded corners, shadow, padding, etc.)
- Remove `size` variants (sm/md/lg/xl) — purely visual concerns
- Remove hardcoded z-index values
- Remove any basalt theme imports
- Remove app-specific close button rendering — consumer provides own

**Generalization steps:**
- `ModalRoot` props: `open: bool`, `on_close: EventHandler<()>`, `close_on_escape: bool`, `close_on_backdrop: bool`, `trap_focus: bool`, `lock_scroll: bool`
- `ModalHandle` from `use_modal(initial_open)`: `open: Signal<bool>`, `show/close/toggle: Callback<()>`
- Compound pattern: `ModalRoot` provides `ModalContext` via `use_context_provider`, `ModalOverlay`/`ModalContent` consume it
- `ModalRoot` renders nothing when `open = false`
- Use `dioxus-nox-core` for focus trap and scroll lock
- Generate unique IDs for `aria-labelledby` association

**ARIA contract:**
- `ModalRoot`: container element for portal/context
- `ModalContent`: `role="dialog"`, `aria-modal="true"`, `aria-labelledby="{generated-id}"`, `tabindex="-1"`
- `ModalOverlay`: `aria-hidden="true"` (decorative backdrop)
- Data attributes: `data-state="open|closed"` on Root and Content

**Test surface:**
- State: `use_modal(false)` → open=false; `.show()` → open=true; `.close()` → open=false; `.toggle()` flips
- ESC key fires `on_close` when `close_on_escape=true`, does not fire when `false`
- Backdrop click fires `on_close` when `close_on_backdrop=true`
- Content click does NOT propagate to backdrop (stopPropagation)
- Renders nothing when `open=false`

---

### 1.3 `dioxus-nox-drawer`

**Action:** NEW CRATE

**Source files to read:**
- `basalt-fitness/crates/basalt-ui/src/components/ui/slide_over_panel.rs` — slide-over sheet

**File tree:**
```
crates/drawer/
├── Cargo.toml              — deps: dioxus, dioxus-nox-core
├── README.md               — usage examples showing all four sides
├── src/
│   ├── lib.rs              — module declarations, re-exports, `pub mod drawer { ... }` namespace
│   ├── types.rs            — DrawerSide enum, DrawerContext struct
│   ├── components.rs       — DrawerRoot, DrawerOverlay, DrawerContent compound components
│   └── tests.rs            — DrawerSide serialization, state tests
```

**Stripping checklist:**
- Remove all Tailwind classes (translate-x, w-96, shadow, etc.)
- Remove hardcoded width/height dimensions
- Remove animation/transition CSS — consumer applies via `data-state` + `data-side`
- Remove any slide-over-panel-specific naming → generic drawer terminology
- Remove basalt app-specific close button and header rendering

**Generalization steps:**
- `DrawerSide` enum: `Left`, `Right` (default), `Bottom`, `Top`
- `DrawerRoot` props: `open: bool`, `on_close: EventHandler<()>`, `side: DrawerSide`, `close_on_escape: bool`, `close_on_overlay: bool`, `lock_scroll: bool`
- Compound pattern: `DrawerRoot` provides `DrawerContext`, `DrawerOverlay`/`DrawerContent` consume it
- Shares focus-trap and scroll-lock with modal via `dioxus-nox-core`
- Renders nothing when `open = false`

**ARIA contract:**
- `DrawerContent`: `role="dialog"`, `aria-modal="true"`, `aria-labelledby="{generated-id}"`
- `DrawerOverlay`: `aria-hidden="true"`
- Data attributes: `data-state="open|closed"`, `data-side="left|right|bottom|top"` on Content

**Test surface:**
- `DrawerSide` default is `Right`
- ESC key and overlay click close behavior (configurable)
- Content click does NOT close (stopPropagation)
- `data-side` matches the `side` prop value
- Renders nothing when `open=false`

---

## Wave 2 — Short-term

### 2.1 `dioxus-nox-toast`

**Action:** NEW CRATE

**Source files to read:**
- `basalt-fitness/crates/basalt-ui/src/components/ui/toast.rs` — toast/undo-toast system

**File tree:**
```
crates/toast/
├── Cargo.toml              — deps: dioxus; wasm target: js-sys (for Date::now())
├── README.md               — usage with generic toast data type
├── src/
│   ├── lib.rs              — module declarations, re-exports
│   ├── types.rs            — ToastId (dual: u64 default + optional uuid feature), Toast<T>
│   ├── manager.rs          — ToastManager<T> with show/dismiss/get, auto-dismiss logic
│   ├── hook.rs             — use_toast_manager<T>() context provider hook
│   ├── components.rs       — ToastViewport<T> headless renderer
│   └── tests.rs            — queue management, max-toast eviction, auto-dismiss
```

**Toast ID strategy:** Default `ToastId(u64)` via atomic counter (zero deps). Optional `uuid` feature flag enables `ToastId::from(Uuid)` constructor for persistence scenarios.

```toml
[dependencies]
uuid = { version = "1", optional = true }

[features]
uuid = ["dep:uuid"]
```

**Stripping checklist:**
- Remove `UndoableAction` enum and `UndoActionType` → generic `T: Clone + 'static`
- Remove `execute_undo()` domain logic — consumer handles undo via `on_undo` callback
- Remove all Tailwind classes (toast positioning, slide animations, colors)
- Remove hardcoded toast messages ("Exercise deleted", "Set removed")
- Remove basalt API calls in undo handlers

**Generalization steps:**
- `Toast<T: Clone + 'static>` — generic over user-defined data type
- `ToastManager<T>` — generic manager with `show(data: T, duration: Duration)`, `show_undoable(data: T, duration: Duration)`, `dismiss(id: ToastId)`, `get(id: ToastId)`
- `max_toasts: usize` (default 3), FIFO eviction of oldest when exceeded
- Each toast has wall-clock expiry via `expires_at_ms: i64` (epoch ms, same pattern as timer crate)
- `ToastViewport<T>` takes a `render_toast: fn(Toast<T>) -> Element` callback
- `use_toast_manager<T>(max_toasts)` provides context at app root

**ARIA contract:**
- `ToastViewport`: `role="region"`, `aria-label="Notifications"`, `aria-live="polite"`
- Each toast container: `role="status"`, `aria-atomic="true"`
- Data attributes: `data-toast-state="active|dismissing"` per toast

**Test surface:**
- Queue: add 4 toasts with max=3 → oldest evicted
- Dismiss: `dismiss(id)` removes specific toast
- Auto-dismiss: toast with 5s duration expires (testable with mock clock)
- `show_undoable` sets `undoable=true` on the toast
- `get(id)` returns `Some` for active, `None` for dismissed

---

### 2.2 `dioxus-nox-master-detail`

**Action:** NEW CRATE

**Source files to read:**
- `basalt-fitness/crates/basalt-ui/src/components/ui/master_detail_layout.rs` — adaptive 2-panel layout

**File tree:**
```
crates/master-detail/
├── Cargo.toml              — deps: dioxus
├── README.md               — usage with responsive behavior explanation
├── src/
│   ├── lib.rs              — module declarations, re-exports, `pub mod master_detail { ... }` namespace
│   ├── types.rs            — MasterDetailContext struct
│   ├── components.rs       — MasterDetail, MasterPanel, DetailPanel, DetailBackdrop
│   └── tests.rs            — context propagation, data-attribute generation
```

**Stripping checklist:**
- Remove all Tailwind classes (grid columns, flex, responsive breakpoints)
- Remove any workout-specific detail panel content
- Remove hardcoded breakpoint pixel values — use `data-layout` for consumer CSS
- Remove basalt routing/navigation dependencies

**Generalization steps:**
- `MasterDetail` props: `list_content: Element`, `detail_content: Element`, `detail_open: bool`, `on_detail_close: EventHandler<()>`
- Pure CSS responsive via data attributes — no JS breakpoint detection needed
- Consumer styles panels via `data-detail="open|closed"` and `data-layout` selectors
- Compound pattern: Root provides context, MasterPanel/DetailPanel/DetailBackdrop consume it
- Focus management: when detail opens on mobile, move focus to detail panel

**ARIA contract:**
- `MasterPanel`: `role="region"`, `aria-label="List"`
- `DetailPanel`: `role="region"`, `aria-label="Detail"`, `aria-hidden` when closed on mobile
- `DetailBackdrop`: `aria-hidden="true"` (decorative)
- Data attributes: `data-detail="open|closed"` on Root, `data-layout="overlay|side|inline"` on DetailPanel

**Test surface:**
- `detail_open=false` → detail panel has `data-detail="closed"`
- `detail_open=true` → `data-detail="open"`
- Backdrop click fires `on_detail_close`
- Context provides `detail_open` state to children

---

## Wave 3 — Primitives Bundle

### 3.1 `dioxus-nox-cycle`

**Action:** NEW CRATE (standalone; candidate for future `dioxus-nox-primitives` collection)

**Source files to read:**
- `basalt-fitness/crates/basalt-ui/src/components/workout/set_type_toggle.rs` — cycling toggle

**File tree:**
```
crates/cycle/
├── Cargo.toml              — deps: dioxus
├── README.md               — usage for enum cycling, set-type toggles, etc.
├── src/
│   ├── lib.rs              — module declarations, re-exports of use_cycle, CycleState
│   ├── hook.rs             — use_cycle<T>() hook implementation
│   ├── types.rs            — CycleState<T> struct
│   └── tests.rs            — wraparound, boundary, set_index tests
```

**Stripping checklist:**
- Remove `SetType` enum (Normal, Warmup, Dropset, Failure) — generic `T`
- Remove set-type-specific colors, icons, labels
- Remove workout domain imports

**Generalization steps:**
- `use_cycle<T: Clone + PartialEq + 'static>(items: &[T], initial_index: Option<usize>) -> CycleState<T>`
- `CycleState<T>`: `current: Signal<T>`, `index: Signal<usize>`, `next: Callback<()>`, `previous: Callback<()>`, `set_index: Callback<usize>`
- `next()` wraps last→first; `previous()` wraps first→last
- `set_index` clamps to valid range
- Pure signal-based, no effects needed

**ARIA contract:**
- Hook-only crate, no rendered components
- README documents recommended: consumer should use `aria-label` describing current value on their toggle element

**Breaking change risk:** N/A (new crate)

**Test surface:**
- `next()` from last index → wraps to 0
- `previous()` from index 0 → wraps to last
- `set_index(n)` → current matches items[n]
- `set_index` out of bounds → clamps to last valid index
- Single-item list: next/previous stay at 0

---

### 3.2 `dioxus-nox-inline-confirm`

**Action:** NEW CRATE

**Source files to read:**
- `basalt-fitness/crates/basalt-ui/src/components/ui/inline_confirm.rs` — inline confirmation

**File tree:**
```
crates/inline-confirm/
├── Cargo.toml              — deps: dioxus
├── README.md               — usage for destructive action confirmation
├── src/
│   ├── lib.rs              — module declarations, re-exports, `pub mod inline_confirm { ... }`
│   ├── types.rs            — ConfirmState enum, InlineConfirmContext, InlineConfirmHandle
│   ├── hook.rs             — use_inline_confirm() hook
│   ├── components.rs       — InlineConfirmRoot, InlineConfirmTrigger, InlineConfirmAction
│   └── tests.rs            — state transitions, auto-cancel timeout
```

**Stripping checklist:**
- Remove Tailwind classes (red button colors, spacing, transitions)
- Remove hardcoded confirm text ("Are you sure?", "Delete")
- Remove basalt-specific action handlers (delete exercise, remove set)

**Generalization steps:**
- `ConfirmState` enum: `Idle`, `Confirming`
- `use_inline_confirm(auto_cancel_ms: Option<u64>) -> InlineConfirmHandle`
- `InlineConfirmHandle`: `state: Signal<ConfirmState>`, `request/confirm/cancel: Callback<()>`
- `InlineConfirmRoot` props: `state: ConfirmState`, `on_confirm: EventHandler<()>`, `on_cancel: EventHandler<()>`, `auto_cancel_ms: Option<u64>`
- Compound components: `InlineConfirmTrigger` (shown when Idle), `InlineConfirmAction` (shown when Confirming)
- Auto-cancel: spawn timer on entering Confirming, cancel → Idle after timeout

**ARIA contract:**
- `InlineConfirmRoot`: `data-state="idle|confirming"`
- `InlineConfirmTrigger`: consumer provides `aria-label` for the trigger button
- `InlineConfirmAction`: consumer provides confirm/cancel button labeling
- No specific role needed on root (consumer wraps in appropriate context)

**Breaking change risk:** N/A (new crate)

**Test surface:**
- Idle → `request()` → Confirming
- Confirming → `confirm()` → Idle + on_confirm fired
- Confirming → `cancel()` → Idle + on_cancel fired
- Auto-cancel: enter Confirming, wait > timeout → auto-transitions to Idle
- Double `request()` while already Confirming → no-op

---

### 3.3 `dioxus-nox-toggle-group`

**Action:** NEW CRATE

**Source files to read:**
- `basalt-fitness/crates/basalt-ui/src/components/ui/filter_chips.rs` — filter chip bar
- `basalt-fitness/crates/basalt-ui/src/components/ui/toggle.rs` — toggle switch

**File tree:**
```
crates/toggle-group/
├── Cargo.toml              — deps: dioxus
├── README.md               — usage for segmented controls, filter chips, radio groups
├── src/
│   ├── lib.rs              — module declarations, re-exports, `pub mod toggle_group { ... }`
│   ├── types.rs            — ToggleItem<K>, ToggleGroupContext<K>, ToggleGroupState<K>, Orientation enum
│   ├── hook.rs             — use_toggle_group<K>() hook
│   ├── components.rs       — ToggleGroupRoot<K>, ToggleGroupItem<K>
│   └── tests.rs            — selection, keyboard nav, disabled item skipping
```

**Stripping checklist:**
- Remove Tailwind classes (chip colors, active background, rounded-full, etc.)
- Remove `FilterKey` domain enum → generic `K: Clone + PartialEq`
- Remove hardcoded filter labels ("All", "Chest", "Back", "Legs")
- Remove basalt-specific filter logic

**Generalization steps:**
- `ToggleItem<K: Clone + PartialEq>`: `key: K`, `label: String`, `disabled: bool`
- `use_toggle_group<K>(items, initial) -> ToggleGroupState<K>`
- `ToggleGroupRoot<K>` props: `value: K`, `on_value_change: EventHandler<K>`, `multi_select: bool`, `orientation: Orientation`
- `ToggleGroupItem<K>` props: `value: K`, `children: Element`, `disabled: bool`
- Single-select mode: `role="radiogroup"` / `role="radio"` with `aria-checked`
- Multi-select mode: `role="group"` / `role="checkbox"` with `aria-checked`
- Keyboard: Arrow keys navigate between items, Space/Enter activates, Home/End jump

**ARIA contract:**
- `ToggleGroupRoot`: `role="radiogroup"` (single) or `role="group"` (multi), `aria-orientation="horizontal|vertical"`, `aria-label` (consumer provides)
- `ToggleGroupItem`: `role="radio"` (single) or `role="checkbox"` (multi), `aria-checked="true|false"`, `tabindex="0|-1"` (roving tabindex), `data-state="on|off"`, `data-disabled` (when disabled)
- Keyboard: ArrowLeft/ArrowRight (horizontal), ArrowUp/ArrowDown (vertical), Home, End, Space/Enter

**Breaking change risk:** N/A (new crate)

**Test surface:**
- Single-select: clicking item B when A is active → B active, A inactive
- Multi-select: clicking active item toggles it off
- Disabled item: cannot be activated, skipped in keyboard navigation
- Keyboard: Arrow navigation wraps around, disabled items skipped
- `aria-checked` reflects current selection state

---

## Wave 4 — Enhancements

### 4.1 Shell Bottom Action Bar Slot

**Action:** ENHANCEMENT to `dioxus-nox-shell`

**Files to modify:**
- `/home/user/dioxus-nox/crates/shell/src/shell.rs` — add `action_bar` prop to `AppShell`

**Source files to read:**
- `basalt-fitness/crates/basalt-ui/src/components/workout/responsive_layout.rs` — mobile action bar

**Changes:**
- Add `action_bar: Option<Element>` prop to `AppShell` component (after existing `fab` prop)
- Render as `div { data_shell_action_bar: "", ..action_bar }` when `Some`, positioned after main content
- Only render when breakpoint is compact/mobile (check `ShellBreakpoint`)
- Add `data-shell-action-bar` data attribute for CSS targeting

**ARIA contract:**
- `role="toolbar"`, `aria-label="Actions"` on the action bar container

**Breaking change risk:** **LOW** — purely additive. New optional prop with default `None`. No existing API changes.

**Test surface:**
- `action_bar: None` → no action bar rendered
- `action_bar: Some(el)` → rendered with `data-shell-action-bar` attribute
- Verify `role="toolbar"` present on rendered element

---

### 4.2 DnD Accessible Reorder Buttons

**Action:** ENHANCEMENT to `dioxus-nox-dnd`

**Files to modify:**
- `/home/user/dioxus-nox/crates/dnd/src/patterns/sortable/` — add reorder button component

**Source files to read:**
- `basalt-fitness/crates/basalt-ui/src/components/workout/draggable_list.rs` — move up/down buttons

**Changes:**
- Add new file `crates/dnd/src/patterns/sortable/reorder_buttons.rs`
- Add `ReorderButtons` component: renders nothing visible (headless), provides `move_up`/`move_down` callbacks
- Props: `index: usize`, `total: usize`, `on_reorder: EventHandler<ReorderEvent>`
- Integrate with existing `SortableContext` — reads context to determine valid moves
- Re-export from `crates/dnd/src/patterns/sortable/mod.rs`

**ARIA contract:**
- Move up button: `aria-label="Move item up"`, `aria-disabled="true"` when at index 0
- Move down button: `aria-label="Move item down"`, `aria-disabled="true"` when at last index
- Both: `role="button"`, `tabindex="0"`

**Breaking change risk:** **LOW** — purely additive. New component, no existing API changes. New re-export in sortable module.

**Test surface:**
- At index 0: move_up disabled, move_down enabled
- At last index: move_down disabled, move_up enabled
- Middle index: both enabled
- `move_up` fires `ReorderEvent { from: index, to: index - 1 }`
- `move_down` fires `ReorderEvent { from: index, to: index + 1 }`

---

### 4.3 `dioxus-nox-password-strength`

**Action:** NEW CRATE

**Source files to read:**
- `basalt-fitness/crates/basalt-ui/src/components/password_strength.rs` — strength meter

**File tree:**
```
crates/password-strength/
├── Cargo.toml              — deps: dioxus (optional, behind "dioxus" feature flag)
├── README.md               — usage as pure function + optional Dioxus hook
├── src/
│   ├── lib.rs              — module declarations, re-exports; core module always available, hook behind cfg
│   ├── types.rs            — StrengthLevel enum, StrengthCheck, StrengthResult
│   ├── assess.rs           — assess_password_strength() pure function, default_checks()
│   ├── hook.rs             — use_password_strength() Dioxus hook (behind "dioxus" feature)
│   └── tests.rs            — comprehensive strength assessment tests
```

**Critical constraint:** `assess_password_strength` must be a pure function with **zero Dioxus dependency** so it can be used in non-Dioxus contexts (CLI tools, server-side validation, etc.).

**Cargo.toml structure:**
```toml
[dependencies]
dioxus = { workspace = true, optional = true }

[features]
default = ["dioxus"]
dioxus = ["dep:dioxus"]
```

**Stripping checklist:**
- Remove Tailwind classes (bar colors, widths, text styling)
- Remove rendered meter/bar component — consumer builds own UI
- Remove basalt auth page imports

**Generalization steps:**
- `StrengthLevel` enum: `None=0`, `Weak=1`, `Fair=2`, `Good=3`, `Strong=4` (implements `Ord`)
- `StrengthCheck { label: &'static str, passed: bool }`
- `StrengthResult { level: StrengthLevel, score: u8, label: &'static str, checks: Vec<StrengthCheck> }`
- `assess_password_strength(password: &str, checks: &[Box<dyn Fn(&str) -> StrengthCheck>]) -> StrengthResult` — pure function with custom checks
- `assess_password_strength_default(password: &str) -> StrengthResult` — convenience wrapper using `default_checks()`
- `default_checks() -> Vec<Box<dyn Fn(&str) -> StrengthCheck>>` — returns: length ≥ 8, length ≥ 12, has uppercase, has number, has special char
- Score = count of passing checks, clamped to 0-4
- `use_password_strength(password: Signal<String>) -> Signal<StrengthResult>` (Dioxus hook, behind feature, uses default checks)
- `use_password_strength_with(password: Signal<String>, checks: Vec<...>) -> Signal<StrengthResult>` (custom checks variant)

**ARIA contract:**
- Hook-only + pure function crate, no rendered components
- README documents recommended: consumer should use `role="meter"`, `aria-valuenow={score}`, `aria-valuemin="0"`, `aria-valuemax="4"`, `aria-label="Password strength"`

**Breaking change risk:** N/A (new crate)

**Test surface:**
- Empty string → `StrengthLevel::None`, score 0
- "abc" → `Weak` (only short, no checks pass except maybe none)
- "Abcdefgh1!" → multiple checks pass → `Good` or `Strong`
- "Abcdefghijkl1!" (≥12, uppercase, number, special) → `Strong`, score 4
- Each individual check: length, uppercase, number, special char tested independently
- `StrengthLevel` ordering: `None < Weak < Fair < Good < Strong`

---

## Verification Plan

### Per-crate verification
1. `cargo check -p dioxus-nox-{crate}` — compiles without errors
2. `cargo test -p dioxus-nox-{crate}` — all unit tests pass
3. `cargo doc -p dioxus-nox-{crate} --no-deps` — docs generate cleanly

### Workspace verification
1. `cargo check --workspace` — full workspace compiles
2. `cargo test --workspace` — all tests pass including existing crates
3. Verify umbrella crate: `cargo check -p dioxus-nox --features full`

### Integration checks
- After Wave 1: verify `dioxus-nox-core` is used by both modal and drawer (no duplicated focus trap code)
- After Wave 4: verify `dioxus-nox-cmdk` still works after extracting helpers to internal crate
- After Wave 4: verify `dioxus-nox-shell` and `dioxus-nox-dnd` pass existing tests after enhancements

### ARIA audit
- For each component crate: grep for expected `role`, `aria-*`, and `data-*` attributes in component source

---

## Resolved Decisions

1. **Timer time dependency**: Use raw i64 millisecond timestamps with platform-specific `now_ms()`. Wasm: `js_sys::Date::now()`. Desktop/iOS/Android: `std::time::SystemTime`. No chrono dependency.
2. **Toast IDs**: Default `u64` atomic counter (zero deps). Optional `uuid` feature flag for persistence scenarios.
3. **Shared utilities**: Separate `dioxus-nox-core` crate. Modal, drawer, and cmdk all depend on it.
4. **Password extensibility**: `assess_password_strength` accepts custom check functions. `default_checks()` provided as convenience.

## Open Questions

1. **Toggle group: roving tabindex vs aria-activedescendant**: The spec implies roving tabindex. Recommend committing to roving tabindex only (better assistive technology support), matching the pattern used in `dioxus-nox-tabs`.

2. **Master-detail breakpoint detection**: The spec says "Pure CSS responsive via data attributes." Recommend staying pure-CSS (simpler, no JS eval). Consumers needing JS breakpoint detection can use `use_shell_breakpoint()` from the shell crate separately.

3. **Timer: stopwatch start/end API**: The spec uses `DateTime<Utc>`. With our no-chrono decision, should the stopwatch accept epoch-ms `i64` values, or should it accept `std::time::SystemTime` on non-wasm and `f64` (Date.now()) on wasm? Recommend: accept `i64` epoch-ms on all platforms for simplicity.
