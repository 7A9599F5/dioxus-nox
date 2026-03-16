# dioxus-nox-select — Research

## WAI-ARIA Roles, States, and Properties

### Combobox Pattern (W3C APG)

**Elements and Roles:**

| Element | Role | Required |
|---------|------|----------|
| Trigger (select-only) | `combobox` on `<button>` | Yes |
| Input (editable) | `combobox` on `<input>` | Yes (for combobox variant) |
| Popup container | `listbox` | Yes |
| Option | `option` | Yes |
| Option group | `group` | Optional |
| Group label | (referenced via `aria-labelledby`) | Optional |

**States/Properties on combobox element:**

| Attribute | Values | Notes |
|-----------|--------|-------|
| `aria-expanded` | `true` / `false` | Required. Popup visibility. |
| `aria-haspopup` | `listbox` | Required. Popup type. |
| `aria-controls` | `[listbox-id]` | Required. References popup element. |
| `aria-activedescendant` | `[option-id]` | When an option is highlighted. DOM focus stays on combobox. |
| `aria-autocomplete` | `none` / `list` / `both` | Editable combobox only. Describes filtering behavior. |
| `aria-disabled` | `true` | When combobox is disabled. |

**States/Properties on listbox:**

| Attribute | Values | Notes |
|-----------|--------|-------|
| `aria-multiselectable` | `true` | Only for multi-select mode. |
| `aria-label` | string | Accessible name. |

**States/Properties on option:**

| Attribute | Values | Notes |
|-----------|--------|-------|
| `aria-selected` | `true` / `false` | Selection state. |
| `aria-disabled` | `true` | When option is disabled. |

### Keyboard Interactions

#### Select-Only Combobox

| Key | Popup Closed | Popup Open |
|-----|-------------|------------|
| Space / Enter | Open popup, highlight current value | Select highlighted, close |
| Down Arrow | Open, highlight first/next | Highlight next |
| Up Arrow | Open, highlight last/prev | Highlight prev |
| Home | Open, highlight first | Highlight first |
| End | Open, highlight last | Highlight last |
| Escape | No action | Close popup |
| Printable char | Type-ahead: open + jump to matching item | Type-ahead: jump to matching item |
| Tab | Move focus out | Select highlighted, close, move focus |

#### Editable Combobox (autocomplete="list")

| Key | Popup Closed | Popup Open |
|-----|-------------|------------|
| Down Arrow | Open, highlight first | Highlight next |
| Up Arrow | No action | Highlight prev |
| Alt+Down Arrow | Open without highlighting | No action |
| Enter | No action | Select highlighted, close |
| Escape | No action | Close popup |
| Home / End | Standard text cursor | Standard text cursor |
| Printable | Filter list, open popup | Filter list |

#### Multi-Select Additions

- Selection does NOT follow focus (highlighting ≠ selecting)
- Space toggles selection of highlighted item (popup stays open)
- Enter toggles selection of highlighted item (popup stays open)
- `aria-multiselectable="true"` on listbox

### Focus Management

- DOM focus stays on the combobox element (trigger button or input)
- Visual focus in the listbox is managed via `aria-activedescendant`
- Scroll highlighted option into view when keyboard navigating
- On open: highlight current value (or first item if none)
- On close: return focus to combobox element

## Reference Implementation Comparison

| Feature | Radix Select | Radix Vue Combobox | Reka UI Combobox | Headless UI Combobox | **Our Implementation** |
|---------|-------------|-------------------|-----------------|---------------------|----------------------|
| Select-only | Yes | No | No | No (use Listbox) | Yes |
| Searchable | No | Yes | Yes | Yes | Yes |
| Multi-select | No | Yes (via `multiple`) | Yes | Yes | Yes |
| Autocomplete | No | Yes | Yes | Yes | Yes |
| Filtering | N/A | Built-in + custom | Built-in + custom | Consumer-managed | Nucleo fuzzy + custom |
| Groups | Yes | Yes | Yes | No | Yes |
| Controlled | Yes | Yes | Yes | Yes | Yes |
| Uncontrolled | Yes | Yes | Yes | Yes | Yes |

### Component Part Matrix

| Radix Select | Reka UI Combobox | Headless UI | **Dioxus (ours)** |
|-------------|-----------------|-------------|------------------|
| Root | ComboboxRoot | Combobox | `select::Root` |
| Trigger | ComboboxTrigger | ComboboxButton | `select::Trigger` |
| Value | — | — | `select::Value` |
| — | ComboboxInput | ComboboxInput | `select::Input` |
| — | ComboboxCancel | — | `select::ClearButton` |
| Content | ComboboxContent | ComboboxOptions | `select::Content` |
| Viewport | ComboboxViewport | — | (part of Content) |
| Item | ComboboxItem | ComboboxOption | `select::Item` |
| ItemText | — | — | `select::ItemText` |
| ItemIndicator | ComboboxItemIndicator | — | `select::ItemIndicator` |
| Group | ComboboxGroup | — | `select::Group` |
| Label | ComboboxLabel | ComboboxLabel | `select::Label` |
| Separator | ComboboxSeparator | — | `select::Separator` |
| — | ComboboxEmpty | — | `select::Empty` |

### Why Radix React Has No Combobox (Issue #1342)

Radix UI Primitives never shipped a native Combobox because it was considered extremely difficult to build correctly as a headless primitive — the interaction patterns, focus management, and ARIA requirements are significantly more complex than a basic Select. The issue was labeled "Difficulty: Hard." The community uses Ariakit or cmdk as alternatives. Radix Vue (now Reka UI) did implement one for the Vue ecosystem.

## Codebase Conventions (from existing crates)

1. **File structure**: `lib.rs` (docs + re-exports), `types.rs` (enums, structs), `components.rs` (compound components), `tests.rs`
2. **Compound components**: Namespaced module (`select::Root`, `select::Item`, etc.)
3. **Context**: `use_context_provider()` in Root, `use_context()` / `consume_context()` in children
4. **Controlled + Uncontrolled**: Internal `Signal<T>` + optional controlled `Signal<T>`, with methods that write to whichever is active
5. **Props**: `#[props(extends = GlobalAttributes)] attributes: Vec<Attribute>`, spread with `..attributes`
6. **Registration**: `use_hook()` to register on mount, `use_drop()` to deregister on unmount
7. **Keyboard**: `onkeydown` handler, match on `event.key()`, `prevent_default()` for handled keys
8. **Focus management**: `document::eval` on WASM to call `focus()` / `scrollIntoView()`
9. **IDs**: Atomic counter for instance IDs, format `"nox-select-{instance}-{suffix}"`
10. **Data attributes**: `data-state`, `data-disabled`, crate-prefixed variants
11. **Dependencies**: Minimal — `dioxus = { workspace = true }`, additional only when needed
12. **Tests**: Pure logic tests for navigation, filtering — no Dioxus runtime required
13. **Docs**: AI disclaimer, WAI-ARIA link, layers table, quick-start example, data attributes table, keyboard table

## Sources

- [WAI-ARIA Combobox Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/combobox/)
- [WAI-ARIA Listbox Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/listbox/)
- [Select-Only Combobox Example](https://www.w3.org/WAI/ARIA/apg/patterns/combobox/examples/combobox-select-only/)
- [Editable Combobox with List Autocomplete](https://www.w3.org/WAI/ARIA/apg/patterns/combobox/examples/combobox-autocomplete-list/)
- [Radix UI Select](https://www.radix-ui.com/primitives/docs/components/select)
- [Radix Vue Combobox](https://www.radix-vue.com/components/combobox)
- [Reka UI Combobox](https://reka-ui.com/docs/components/combobox)
- [Headless UI Combobox](https://headlessui.com/react/combobox)
- [Radix Issue #1342](https://github.com/radix-ui/primitives/issues/1342)
