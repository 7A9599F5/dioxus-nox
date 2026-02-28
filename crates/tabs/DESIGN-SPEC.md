# Design Specification: dioxus-nox-tabs

**Status:** Draft  
**Version:** 0.1.0  
**Created:** 2025-02-28  
**Crate:** `dioxus-nox-tabs` at `crates/tabs`

---

## 1. Problem Statement

Provide a headless, accessible tabs primitive for Dioxus 0.7 that follows the Radix Primitives pattern. The component must work across all Dioxus render targets (Web, Desktop, iOS, Android) and provide full keyboard navigation, ARIA compliance, and flexible composition.

---

## 2. Scope Definition

### In Scope
- Compound component architecture (Root, List, Trigger, Content)
- Horizontal and vertical orientation support
- Automatic and manual activation modes
- Controlled and uncontrolled state patterns
- Lazy rendering with optional eager mounting (`force_mount`)
- Full keyboard navigation (arrows, Home, End)
- Loop navigation (configurable)
- Disabled tab support
- ARIA compliance (WAI-ARIA Tabs pattern)
- `data-*` attributes for CSS state targeting
- `class` prop on all components

### Out of Scope
- Visual styling (headless only)
- Animation/transition primitives (consumer responsibility)
- Dynamic tab add/remove (future consideration)
- Tab drag-to-reorder (use dioxus-nox-dnd separately)
- Nested tabs (works but not explicitly designed for)

---

## 3. Validated Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Activation mode | Both, default automatic | Q1: Maximum flexibility, sensible default |
| Orientation | Both horizontal and vertical | Q2: Sidebar navigation patterns |
| Content rendering | Lazy + cache with `force_mount` | Q3: Best performance/flexibility balance |
| State pattern | Controlled + uncontrolled | Q4: Matches workspace conventions |
| Loop navigation | Configurable, default true | Q5: Matches cmdk pattern |
| Disabled tabs | Supported with ARIA | Q6: Permission-gated tabs use case |

---

## 4. API Reference

### 4.1 Module Structure

```rust
// crates/tabs/src/lib.rs
pub mod tabs;

// Re-exports for ergonomic access
pub use tabs::{Root, List, Trigger, Content};
pub use tabs::{TabsContext, Orientation, ActivationMode};
```

Usage:
```rust
use dioxus_nox_tabs::tabs;

rsx! {
    tabs::Root { /* ... */ }
}

// Or direct import
use dioxus_nox_tabs::{Root, List, Trigger, Content};
```

### 4.2 Root Component

Container that provides context to all child components.

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `value` | `Option<Signal<String>>` | `None` | Controlled active tab value |
| `default_value` | `String` | `""` | Initial value for uncontrolled mode |
| `on_value_change` | `Option<EventHandler<String>>` | `None` | Callback when active tab changes |
| `orientation` | `Orientation` | `Horizontal` | Tab list direction |
| `activation_mode` | `ActivationMode` | `Automatic` | When tabs activate (focus vs. explicit) |
| `loop` | `bool` | `true` | Whether navigation wraps at boundaries |
| `class` | `Option<String>` | `None` | CSS class for root element |
| `children` | `Element` | Required | Child components |

| Data Attribute | Values |
|----------------|--------|
| `data-orientation` | `"horizontal"`, `"vertical"` |

### 4.3 List Component

Container for tab triggers. Sets up the `tablist` ARIA role.

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `class` | `Option<String>` | `None` | CSS class |
| `aria_label` | `Option<String>` | `None` | Accessible label for the tab list |
| `aria_labelledby` | `Option<String>` | `None` | ID of element that labels the tab list |
| `children` | `Element` | Required | Trigger children |

| Data Attribute | Values |
|----------------|--------|
| `data-orientation` | `"horizontal"`, `"vertical"` |

| ARIA Attribute | Value |
|----------------|-------|
| `role` | `tablist` |
| `aria-label` | From prop |
| `aria-labelledby` | From prop |

### 4.4 Trigger Component

Individual tab button that activates associated content.

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `value` | `String` | Required | Unique identifier for this tab |
| `disabled` | `bool` | `false` | Whether the tab is interactive |
| `class` | `Option<String>` | `None` | CSS class |
| `children` | `Element` | Required | Tab label content |

| Data Attribute | Values |
|----------------|--------|
| `data-state` | `"active"`, `"inactive"` |
| `data-disabled` | Present when disabled |
| `data-orientation` | `"horizontal"`, `"vertical"` |

| ARIA Attribute | Value |
|----------------|-------|
| `role` | `tab` |
| `aria-selected` | `"true"`, `"false"` |
| `aria-controls` | `{value}-content` |
| `tabindex` | `0` (active) or `-1` (inactive) |

### 4.5 Content Component

Panel containing the tab's content.

| Prop | Type | Default | Description |
|------|------|---------|-------------|
| `value` | `String` | Required | Unique identifier (matches Trigger) |
| `force_mount` | `bool` | `false` | Render immediately, even if inactive |
| `class` | `Option<String>` | `None` | CSS class |
| `children` | `Element` | Required | Tab panel content |

| Data Attribute | Values |
|----------------|--------|
| `data-state` | `"active"`, `"inactive"` |
| `data-orientation` | `"horizontal"`, `"vertical"` |

| ARIA Attribute | Value |
|----------------|-------|
| `role` | `tabpanel` |
| `aria-labelledby` | `{value}-trigger` |
| `tabindex` | `0` |

### 4.6 Types

```rust
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Orientation {
    #[default]
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ActivationMode {
    /// Tab activates immediately on focus (arrow key navigation)
    #[default]
    Automatic,
    /// Tab activates only on Enter/Space
    Manual,
}
```

---

## 5. Context Implementation

```rust
/// Shared state for a tabs instance.
#[derive(Clone, Copy)]
pub struct TabsContext {
    /// Current active tab value
    pub value: Signal<String>,
    /// Tab list orientation
    pub orientation: Signal<Orientation>,
    /// Activation behavior
    pub activation_mode: Signal<ActivationMode>,
    /// Whether navigation wraps at boundaries
    pub loop_navigation: Signal<bool>,
    /// Callback when value changes
    pub on_value_change: Signal<Option<EventHandler<String>>>,
    /// Unique instance identifier for PartialEq
    pub(crate) instance_id: u32,
}

impl PartialEq for TabsContext {
    fn eq(&self, other: &Self) -> bool {
        self.instance_id == other.instance_id
    }
}
```

### Context Access

```rust
/// Hook to access the nearest TabsContext.
/// Panics if called outside a tabs::Root.
pub fn use_tabs_context() -> TabsContext {
    use_context::<TabsContext>()
}
```

---

## 6. Keyboard Interactions

### Focus Management

| Key | Action (Horizontal) | Action (Vertical) |
|-----|--------------------|--------------------|
| `Tab` | Focus active trigger; from trigger, focus active content | Same |
| `ArrowRight` | Next trigger (or loop) | — |
| `ArrowLeft` | Previous trigger (or loop) | — |
| `ArrowDown` | — | Next trigger (or loop) |
| `ArrowUp` | — | Previous trigger (or loop) |
| `Home` | First trigger | First trigger |
| `End` | Last trigger | Last trigger |
| `Enter` | Activate tab (manual mode only) | Same |
| `Space` | Activate tab (manual mode only) | Same |

### Activation Behavior

| Mode | Arrow Key Press | Enter/Space |
|------|----------------|-------------|
| Automatic | Focus + Activate | (no-op, already active) |
| Manual | Focus only | Activate focused tab |

### Disabled Tab Behavior

- Disabled tabs are focusable (for screen reader discovery)
- Arrow keys skip disabled tabs in navigation
- Enter/Space on disabled tab has no effect
- `aria-disabled="true"` set on disabled triggers

---

## 7. Rendering Strategy

### Lazy Mount with Cache

```
Initial state:
  - Only active tab's Content is rendered
  - Inactive Content not in DOM

On tab switch:
  - New active Content renders (if not cached)
  - Previously active Content hidden (stays in DOM if cached)
  - `force_mount: true` Content always rendered
```

### Implementation

```rust
// In Content component
fn render(self, ctx: &mut RenderContext) -> Element {
    let tabs_ctx = use_tabs_context();
    let is_active = (tabs_ctx.value)() == self.value;
    let mut mounted = use_signal(|| is_active || self.force_mount);
    
    // Update mounted state when becoming active
    use_effect(move || {
        if is_active {
            mounted.set(true);
        }
    });
    
    let should_render = mounted() || self.force_mount;
    
    if !should_render {
        return None;
    }
    
    rsx! {
        div {
            role: "tabpanel",
            "data-state": if is_active { "active" } else { "inactive" },
            "data-orientation": match (tabs_ctx.orientation)() {
                Orientation::Horizontal => "horizontal",
                Orientation::Vertical => "vertical",
            },
            class: self.class,
            // ... rest of attributes
            {self.children}
        }
    }
}
```

---

## 8. Accessibility Requirements

### ARIA Roles

| Element | Role | Notes |
|---------|------|-------|
| List | `tablist` | Contains all tab triggers |
| Trigger | `tab` | Interactive tab button |
| Content | `tabpanel` | Content area for active tab |

### ARIA Attributes

| Attribute | Element | Value |
|-----------|---------|-------|
| `aria-label` / `aria-labelledby` | List | From prop |
| `aria-selected` | Trigger | `"true"` / `"false"` |
| `aria-controls` | Trigger | `{value}-content` |
| `aria-disabled` | Trigger | `"true"` when disabled |
| `aria-labelledby` | Content | `{value}-trigger` |

### Focus Management

1. **Tab into tabs**: Focus lands on active trigger
2. **Tab from trigger**: Focus moves to active content
3. **Arrow navigation**: Focus moves between triggers
4. **Home/End**: Focus first/last trigger
5. **Content focus**: Content has `tabindex="0"` for direct focus

### Screen Reader Announcements

- Trigger label announced with "selected" or "not selected"
- Disabled triggers announced as "unavailable"
- Content role announced as "tab panel"

---

## 9. Data Attributes (CSS Targeting)

```css
/* Horizontal vs vertical layout */
[data-orientation="horizontal"] { flex-direction: row; }
[data-orientation="vertical"] { flex-direction: column; }

/* Active/inactive states */
[data-state="active"] { display: block; }
[data-state="inactive"] { display: none; }

/* Disabled trigger */
[data-disabled] { opacity: 0.5; cursor: not-allowed; }
```

---

## 10. Usage Examples

### Basic Uncontrolled Tabs

```rust
use dioxus_nox_tabs::tabs;

rsx! {
    tabs::Root {
        default_value: "account",
        tabs::List {
            aria_label: "Settings",
            tabs::Trigger { value: "account", "Account" }
            tabs::Trigger { value: "password", "Password" }
        }
        tabs::Content {
            value: "account",
            // Account settings form
        }
        tabs::Content {
            value: "password",
            // Password change form
        }
    }
}
```

### Controlled Tabs

```rust
let mut active_tab = use_signal(|| "tab1".to_string());

rsx! {
    tabs::Root {
        value: active_tab,
        on_value_change: move |v| active_tab.set(v),
        // ...
    }
}
```

### Vertical Tabs

```rust
rsx! {
    tabs::Root {
        orientation: Orientation::Vertical,
        // ...
    }
}
```

### Manual Activation

```rust
rsx! {
    tabs::Root {
        activation_mode: ActivationMode::Manual,
        // Tabs activate only on Enter/Space
    }
}
```

### Disabled Tab

```rust
rsx! {
    tabs::Trigger {
        value: "admin",
        disabled: !user.is_admin,
        "Admin Settings"
    }
}
```

### Eager Mount (SEO/Preload)

```rust
rsx! {
    tabs::Content {
        value: "important",
        force_mount: true,
        // Always in DOM for SEO or preloading
    }
}
```

---

## 11. Test Strategy

### Unit Tests

| Test | Description |
|------|-------------|
| `context_creation` | Root provides context correctly |
| `value_sync` | Controlled value updates propagate |
| `default_value` | Uncontrolled mode uses default |
| `orientation_attribute` | data-orientation set correctly |
| `activation_mode_automatic` | Arrow keys activate immediately |
| `activation_mode_manual` | Arrow keys focus only, Enter activates |

### Integration Tests

| Test | Description |
|------|-------------|
| `keyboard_navigation_horizontal` | ArrowLeft/Right move focus |
| `keyboard_navigation_vertical` | ArrowUp/Down move focus |
| `keyboard_loop` | Navigation wraps at boundaries |
| `keyboard_no_loop` | Navigation stops at boundaries |
| `home_end_keys` | Home/End jump to first/last |
| `disabled_skip` | Disabled tabs skipped in navigation |
| `lazy_mount` | Inactive content not rendered initially |
| `lazy_mount_cache` | Content stays in DOM after first render |
| `force_mount` | Content rendered even when inactive |
| `aria_roles` | Correct role attributes on all parts |
| `aria_selected` | aria-selected reflects active state |
| `aria_controls` | Trigger aria-controls matches content id |

### Accessibility Tests

| Test | Description |
|------|-------------|
| `screen_reader_compatible` | ARIA attributes complete |
| `focus_visible` | Focus rings work correctly |
| `keyboard_only_navigation` | Full workflow without mouse |

---

## 12. File Structure

```
crates/tabs/
├── Cargo.toml
├── DESIGN-SPEC.md (this file)
├── CLAUDE.md
└── src/
    ├── lib.rs           # Public API + re-exports
    ├── context.rs       # TabsContext + use_tabs_context
    ├── components/
    │   ├── mod.rs
    │   ├── root.rs      # Root component
    │   ├── list.rs      # List component
    │   ├── trigger.rs   # Trigger component
    │   └── content.rs   # Content component
    ├── types.rs         # Orientation, ActivationMode
    └── navigation.rs    # Keyboard navigation helpers
```

---

## 13. Dependencies

```toml
[package]
name = "dioxus-nox-tabs"
version = "0.1.0"
edition.workspace = true

[dependencies]
dioxus = { workspace = true }

[dev-dependencies]
# For examples/testing
```

---

## 14. Implementation Phases

### Phase 1: Core Structure
- [ ] Create crate structure
- [ ] Define types (Orientation, ActivationMode)
- [ ] Implement TabsContext
- [ ] Implement Root component with context provider

### Phase 2: Basic Components
- [ ] Implement List component
- [ ] Implement Trigger component (basic)
- [ ] Implement Content component (basic)
- [ ] Wire up data attributes

### Phase 3: State Management
- [ ] Controlled mode (value + on_value_change)
- [ ] Uncontrolled mode (default_value)
- [ ] Prop sync via use_effect

### Phase 4: Keyboard Navigation
- [ ] Arrow key navigation
- [ ] Home/End keys
- [ ] Loop behavior
- [ ] Manual activation mode

### Phase 5: Advanced Features
- [ ] Disabled tabs
- [ ] Lazy mounting
- [ ] force_mount prop
- [ ] Vertical orientation

### Phase 6: Polish
- [ ] ARIA attributes
- [ ] Unit tests
- [ ] Integration tests
- [ ] Documentation
- [ ] Examples

---

## 15. Risk Register

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Focus management complexity on multi-platform | Medium | High | Test on all targets early |
| Lazy mount timing issues | Low | Medium | Use Dioxus effects properly |
| ARIA ID generation conflicts | Low | Low | Use instance_id prefix |
| Keyboard conflicts with parent components | Low | Medium | Document event propagation |

---

## 16. Success Criteria

1. ✅ All ARIA attributes pass accessibility audit
2. ✅ Full keyboard navigation works on Web, Desktop, iOS, Android
3. ✅ Matches Radix Tabs API surface
4. ✅ Follows dioxus-nox patterns (context, signals, data attributes)
5. ✅ Zero visual styles shipped
6. ✅ Test coverage > 80%
7. ✅ Compiles on wasm32-unknown-unknown without web_sys
8. ✅ Documentation with 5+ usage examples

---

## 17. Open Questions (Deferred)

| Question | Status | Notes |
|----------|--------|-------|
| Dynamic tab add/remove API | Deferred | Wait for user feedback |
| Tab close button pattern | Deferred | Out of scope for v1 |
| Nested tabs styling guidance | Deferred | Document as consumer responsibility |

---

## Appendix: Radix Alignment Checklist

| Radix Feature | dioxus-nox-tabs | Status |
|---------------|-----------------|--------|
| Root component | ✅ | Planned |
| List component | ✅ | Planned |
| Trigger component | ✅ | Planned |
| Content component | ✅ | Planned |
| `value` / `defaultValue` | ✅ | Planned |
| `onValueChange` | ✅ | Planned |
| `orientation` | ✅ | Planned |
| `activationMode` | ✅ | Planned |
| `loop` (List) | ✅ | Planned |
| `disabled` (Trigger) | ✅ | Planned |
| `forceMount` (Content) | ✅ | Planned |
| `data-state` | ✅ | Planned |
| `data-orientation` | ✅ | Planned |
| `data-disabled` | ✅ | Planned |
| ARIA roles | ✅ | Planned |
| Keyboard navigation | ✅ | Planned |
