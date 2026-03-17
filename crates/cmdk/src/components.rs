use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use dioxus::prelude::*;
use dioxus_core::use_drop;
use keyboard_types::Modifiers;

use crate::context::{CommandContext, init_command_context};
use crate::helpers::{make_input_id, make_item_dom_id, make_listbox_id, scroll_item_into_view};
use crate::hook::use_is_mobile;
use crate::hook::{CommandHistoryContext, CommandPaletteHandle};
use crate::shortcut::Hotkey;
use crate::types::sheet_math;
use crate::types::{
    ActionRegistration, CustomFilter, DragState, GroupId, GroupRegistration, ItemId,
    ItemRegistration, ItemSelectCallback, PageId, PageRegistration, PaletteMode,
    ScoringStrategyProp, Side,
};

/// Props for the CommandRoot component.
#[component]
pub fn CommandRoot(
    children: Element,
    on_select: Option<EventHandler<String>>,
    #[props(default)] filter: Option<CustomFilter>,
    /// Pre-fill the search input on mount.
    #[props(default)]
    initial_search: Option<String>,
    /// Called whenever the search query changes.
    on_search_change: Option<EventHandler<String>>,
    /// Called when the active (highlighted) item changes.
    /// Receives `Some(resolved_value)` or `None` when no item is active.
    on_active_change: Option<EventHandler<Option<String>>>,
    /// Enable Ctrl+N/J (next) and Ctrl+P/K (prev) vim-style navigation.
    #[props(default = false)]
    vim_bindings: bool,
    #[props(default)] class: Option<String>,
    /// Pluggable scoring strategy for post-nucleo score adjustment.
    /// Wrap in `ScoringStrategyProp` for Dioxus component compatibility.
    #[props(default)]
    scoring_strategy: Option<ScoringStrategyProp>,
    /// Accessible label for the command palette. Renders a visually-hidden
    /// `<label>` linked to the search input, and sets `aria-label` on the input.
    #[props(default, into)]
    label: Option<String>,
    /// Disable pointer (mouse/touch) selection of items. When `true`,
    /// items won't become active on hover. Keyboard navigation still works.
    #[props(default = false)]
    disable_pointer_selection: bool,
    /// When `true` (default), navigation wraps around from last to first and vice versa.
    /// When `false`, navigation stops at list boundaries.
    #[props(default = true)]
    loop_navigation: bool,
    /// Default active item value. When the active item resets (e.g., after filtering),
    /// the palette tries to match this value against visible items' `value` or `id`
    /// before falling back to the first item.
    #[props(default, into)]
    default_value: Option<String>,
    /// When `true` (default), applies fuzzy scoring to filter items.
    /// When `false`, all non-hidden items are returned in registration order,
    /// ignoring the search query for filtering purposes.
    #[props(default = true)]
    should_filter: bool,
    /// Delay in milliseconds before the search query updates the filtered results.
    /// `0` (default) means no debouncing — results update on every keystroke.
    /// Use this to reduce re-render frequency when search involves slow operations.
    #[props(default = 0)]
    search_debounce_ms: u32,
    /// Controlled active-item value. When provided, the palette keeps the active
    /// item in sync with this signal. Match is attempted by `item.value` first,
    /// then by `item.id`. Set to `None` to clear the active item.
    #[props(default)]
    value: Option<Signal<Option<String>>>,
    /// Called when the resolved value of the active item changes.
    /// Receives the item's `value` prop (or `id` as fallback).
    /// Complements `on_active_change` with a `String`-typed callback.
    on_value_change: Option<EventHandler<String>>,
    /// P-022: When `true` (default), Tab and Shift+Tab cycle only within the
    /// palette — focus cannot escape to background elements.
    /// On wasm32, uses `querySelectorAll` to enumerate focusable children.
    /// On desktop/mobile, this is a no-op (Dioxus tab guards in dialogs handle it).
    #[props(default = true)]
    trap_focus: bool,
    /// Number of items to advance/retreat per PageDown/PageUp key press.
    /// Defaults to 10.
    #[props(default = 10)]
    page_size: usize,
    /// Called when the palette closes (ctx.is_open transitions true → false).
    /// Use this to sync your own `is_open` signal in standalone floating mode.
    on_close: Option<EventHandler<()>>,
) -> Element {
    let ctx = init_command_context(
        on_select,
        filter,
        initial_search,
        on_search_change,
        on_active_change,
        scoring_strategy.map(|p| p.0),
        label.clone(),
        disable_pointer_selection,
        vim_bindings,
        loop_navigation,
        default_value,
        should_filter,
        search_debounce_ms,
        value,
        on_value_change,
        page_size,
    );

    // P-004: DOM id for the palette root container, used to exempt it from `inert`.
    let palette_root_dom_id = format!("cmdk-palette-root-{}", ctx.instance_id);
    let palette_root_dom_id_eff = palette_root_dom_id.clone();

    // P-004: Watch is_open; mark background siblings inert when open.
    use_effect(move || {
        let open = (ctx.is_open)();
        // Update inert_background signal (readable by tests and child components)
        let mut ib = ctx.inert_background;
        ib.set(open);
        crate::helpers::set_siblings_inert(&palette_root_dom_id_eff, open);
    });

    // Fire on_close when ctx.is_open transitions true → false.
    // Uses a non-reactive Rc<Cell<bool>> so only ctx.is_open subscribes this effect.
    let prev_open = use_hook(|| Rc::new(std::cell::Cell::new(false)));
    let prev_open_eff = prev_open.clone();
    use_effect(move || {
        let is_open = (ctx.is_open)();
        let was_open = prev_open_eff.get();
        prev_open_eff.set(is_open);
        if was_open
            && !is_open
            && let Some(h) = on_close
        {
            h.call(());
        }
    });

    // P-023: Wire 5 screen-reader announcement triggers.
    // 1. Active item change → announce item label
    use_effect(move || {
        let active_id = ctx.active_item.read().clone();
        if let Some(ref id) = active_id {
            let items_list = ctx.items.peek();
            if let Some(item) = items_list.iter().find(|i| i.id == *id) {
                let label = item.label.clone();
                drop(items_list);
                let mut ann = ctx.announcer;
                ann.set(label);
            }
        }
    });

    // 2. Page navigation → announce current page title
    use_effect(move || {
        let feat = *ctx.page_feature.read();
        if let Some(pf) = feat {
            let stack = pf.page_stack.read();
            if let Some(page_id) = stack.last() {
                let pages_list = pf.pages.peek();
                let title = pages_list
                    .iter()
                    .find(|p| p.id == *page_id)
                    .and_then(|p| p.title.clone())
                    .unwrap_or_else(|| page_id.clone());
                drop(pages_list);
                let mut ann = ctx.announcer;
                ann.set(title);
            }
        }
    });

    // 3. Empty state → announce "No results"
    // 4. Filter cleared → announce "Filter cleared"
    use_effect(move || {
        let query = ctx.search.read().clone();
        let count = (ctx.filtered_count)();
        let mut ann = ctx.announcer;
        if query.is_empty() {
            // Search was cleared — only announce if it was previously non-empty.
            // We detect "previously non-empty" by checking if count > 0 (items exist).
            // A simpler heuristic: always announce "Filter cleared" when query becomes empty.
            ann.set("Filter cleared".to_string());
        } else if count == 0 {
            ann.set("No results".to_string());
        }
    });

    // 5. Mode activation → announce mode name
    use_effect(move || {
        let mode = ctx.active_mode.read();
        if let Some(ref m) = *mode {
            let label = m.label.clone();
            drop(mode);
            let mut ann = ctx.announcer;
            ann.set(format!("{label} mode"));
        }
    });

    // Capture-phase keydown listener to preventDefault() on registered item shortcuts
    // while the palette is open. Without this, browser defaults (Ctrl+P=print, Ctrl+S=save,
    // Ctrl+H=history, Ctrl+N=new window, Ctrl+L=address bar) fire before Dioxus's
    // delegated onkeydown handler can intercept them.
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::prelude::*;

        let items = ctx.items;
        let is_open = ctx.is_open;
        type KbClosureHolder =
            Rc<RefCell<Option<Closure<dyn FnMut(web_sys::KeyboardEvent)>>>>;
        let shortcut_closure_holder: KbClosureHolder =
            use_hook(|| Rc::new(RefCell::new(None)));

        let ch = shortcut_closure_holder.clone();
        use_effect(move || {
            // Remove old listener first
            if let Some(old_cl) = ch.borrow_mut().take() {
                use wasm_bindgen::JsCast;
                if let Some(w) = web_sys::window() {
                    let _ = w.remove_event_listener_with_callback_and_bool(
                        "keydown",
                        old_cl.as_ref().unchecked_ref(),
                        true,
                    );
                }
            }

            if !is_open() {
                return;
            }

            let closure = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
                // Skip events without modifiers — no registered shortcut can match
                if !event.ctrl_key() && !event.meta_key() && !event.alt_key() {
                    return;
                }
                let key = event.key();
                let items_list = items.peek();
                for item in items_list.iter() {
                    if item.disabled {
                        continue;
                    }
                    if let Some(ref hk) = item.shortcut
                        && hk.matches_raw(
                            &key,
                            event.ctrl_key(),
                            event.shift_key(),
                            event.alt_key(),
                            event.meta_key(),
                        )
                    {
                        event.prevent_default();
                        return;
                    }
                }
            })
                as Box<dyn FnMut(web_sys::KeyboardEvent)>);

            if let Some(w) = web_sys::window() {
                use wasm_bindgen::JsCast;
                let _ = w.add_event_listener_with_callback_and_bool(
                    "keydown",
                    closure.as_ref().unchecked_ref(),
                    true, // capture phase — fires before browser defaults
                );
            }
            *ch.borrow_mut() = Some(closure);
        });

        let ch2 = shortcut_closure_holder.clone();
        use_drop(move || {
            if let Some(cl) = ch2.borrow_mut().take() {
                use wasm_bindgen::JsCast;
                if let Some(w) = web_sys::window() {
                    let _ = w.remove_event_listener_with_callback_and_bool(
                        "keydown",
                        cl.as_ref().unchecked_ref(),
                        true,
                    );
                }
            }
        });
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        // Non-wasm: no browser shortcuts to intercept.
        let _ = &ctx;
    }

    // P-022: Tab/Shift+Tab focus trap — cycles within the palette container.
    let palette_root_id_trap = palette_root_dom_id.clone();
    let onkeydown_root = move |event: KeyboardEvent| {
        if !trap_focus {
            return;
        }
        if event.key() != Key::Tab {
            return;
        }
        let shift = event.modifiers().contains(Modifiers::SHIFT);

        // wasm32: use querySelectorAll to find all focusable elements
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            if let Some(focusables) =
                crate::helpers::get_focusable_elements_in_container(&palette_root_id_trap)
            {
                if focusables.is_empty() {
                    event.prevent_default();
                    return;
                }
                // Determine which element is currently focused
                let active_el = web_sys::window()
                    .and_then(|w| w.document())
                    .and_then(|d| d.active_element());

                let current_idx = active_el.and_then(|ae| {
                    focusables
                        .iter()
                        .position(|fe| fe.dyn_ref::<web_sys::Element>().is_some_and(|el| el == &ae))
                });

                let next_idx = match (current_idx, shift) {
                    (None, false) => 0,
                    (None, true) => focusables.len() - 1,
                    (Some(i), false) => (i + 1) % focusables.len(),
                    (Some(i), true) => {
                        if i == 0 {
                            focusables.len() - 1
                        } else {
                            i - 1
                        }
                    }
                };

                if let Some(target) = focusables.get(next_idx) {
                    let _ = target.focus();
                    event.prevent_default();
                }
            }
        }
        // non-wasm: no-op; Dioxus tab guard sentinels in CommandDialog/CommandSheet
        // redirect focus back to the input when they receive focus.
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = &palette_root_id_trap;
            let _ = shift;
        }
    };

    rsx! {
        div {
            id: "{palette_root_dom_id}",
            class: class.unwrap_or_default(),
            "data-cmdk-root": "true",
            "data-palette-root": "true",
            onkeydown: onkeydown_root,

            // Visually-hidden accessible label linked to the search input
            if let Some(ref label_text) = label {
                label {
                    r#for: crate::helpers::make_input_id(ctx.instance_id),
                    style: "position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0,0,0,0);white-space:nowrap;border:0;",
                    "{label_text}"
                }
            }

            // P-023: Separate announcer region (polite, atomic) for state transitions.
            // Distinct from status_message so announcements don't interfere with result counts.
            div {
                role: "status",
                "aria-live": "polite",
                "aria-atomic": "true",
                style: "position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0,0,0,0);white-space:nowrap;border:0;",
                "{(ctx.announcer)()}"
            }

            // Existing result-count live region (kept for backward compat)
            div {
                "aria-live": "polite",
                "aria-atomic": "true",
                role: "status",
                style: "position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0,0,0,0);white-space:nowrap;border:0;",
                "{ctx.status_message}"
            }

            {children}
        }
    }
}

/// The search input for the command palette.
#[component]
pub fn CommandInput(
    #[props(default)] placeholder: Option<String>,
    #[props(default)] class: Option<String>,
    #[props(default)] autofocus: bool,
    /// Controlled value. When provided, keeps `ctx.search` in sync with this signal.
    /// The parent controls the input by writing to the signal; `on_search_change`
    /// still fires on every keystroke for uncontrolled-style usage.
    #[props(default)]
    value: Option<Signal<String>>,
    /// Called when the input receives focus.
    #[props(default)]
    onfocus: Option<EventHandler<FocusEvent>>,
    /// Called when the input loses focus.
    #[props(default)]
    onblur: Option<EventHandler<FocusEvent>>,
) -> Element {
    let ctx: CommandContext = use_context();

    // P-013: Sync controlled value → ctx.search when signal changes
    use_effect(move || {
        if let Some(v) = value {
            let mut search = ctx.search;
            search.set(v());
        }
    });
    let mode = ctx.active_mode.read();
    let effective_placeholder = mode
        .as_ref()
        .and_then(|m| m.placeholder.clone())
        .or(placeholder.clone());
    drop(mode);

    let onmounted = move |event: MountedEvent| {
        let data = event.data();
        let mut el = ctx.input_element;
        el.set(Some(data.clone()));
        if autofocus {
            spawn(async move {
                let _ = data.set_focus(true).await;
            });
        }
    };

    let onkeydown = move |event: KeyboardEvent| {
        // IME composition guard: do not process keys during CJK/IME input
        if event.is_composing() {
            return;
        }
        // Vim bindings: Ctrl+N/J -> next, Ctrl+P/K -> prev
        if (ctx.vim_bindings)()
            && event.modifiers().contains(Modifiers::CONTROL)
            && let Key::Character(ref c) = event.key()
        {
            match c.to_lowercase().as_str() {
                "n" | "j" => {
                    event.prevent_default();
                    ctx.select_next();
                    return;
                }
                "p" | "k" => {
                    event.prevent_default();
                    ctx.select_prev();
                    return;
                }
                _ => {}
            }
        }
        match event.key() {
            // Group navigation: Alt+Shift+Arrow (Alt+Arrow is reserved for history navigation)
            Key::ArrowDown
                if event.modifiers().contains(Modifiers::ALT)
                    && event.modifiers().contains(Modifiers::SHIFT) =>
            {
                event.prevent_default();
                ctx.select_next_group();
            }
            Key::ArrowUp
                if event.modifiers().contains(Modifiers::ALT)
                    && event.modifiers().contains(Modifiers::SHIFT) =>
            {
                event.prevent_default();
                ctx.select_prev_group();
            }
            // Alt+ArrowUp → history prev (must appear before unguarded ArrowUp)
            Key::ArrowUp if event.modifiers().contains(Modifiers::ALT) => {
                event.prevent_default();
                if let Some(hist) = try_use_context::<CommandHistoryContext>() {
                    let h = hist.handle;
                    if !h.is_navigating() {
                        let search_val = ctx.search.read();
                        h.save_draft(&search_val);
                    }
                    if let Some(entry) = h.prev() {
                        let mut search = ctx.search;
                        search.set(entry);
                    }
                }
            }
            // Alt+ArrowDown → history next (must appear before unguarded ArrowDown)
            Key::ArrowDown if event.modifiers().contains(Modifiers::ALT) => {
                event.prevent_default();
                if let Some(hist) = try_use_context::<CommandHistoryContext>() {
                    let h = hist.handle;
                    match h.next() {
                        Some(entry) => {
                            let mut search = ctx.search;
                            search.set(entry);
                        }
                        None => {
                            if let Some(draft) = h.take_draft() {
                                let mut search = ctx.search;
                                search.set(draft);
                            }
                            h.reset_navigation();
                        }
                    }
                }
            }
            // P-039: Action panel navigation — when panel is open, intercept arrow/enter/escape
            Key::ArrowDown if ctx.peek_action_panel_open() => {
                event.prevent_default();
                let mut ctx = ctx;
                ctx.select_next_action();
            }
            Key::ArrowUp if ctx.peek_action_panel_open() => {
                event.prevent_default();
                let mut ctx = ctx;
                ctx.select_prev_action();
            }
            Key::Enter if ctx.peek_action_panel_open() => {
                event.prevent_default();
                let mut ctx = ctx;
                ctx.confirm_action();
            }
            // P-039: Escape or ArrowLeft closes the action panel (without closing the palette)
            Key::Escape | Key::ArrowLeft if ctx.peek_action_panel_open() => {
                event.prevent_default();
                let mut ctx = ctx;
                ctx.close_action_panel();
            }
            // P-039: Tab or ArrowRight opens the action panel if the active item has actions registered
            Key::Tab | Key::ArrowRight
                if !ctx.peek_action_panel_open()
                    && ctx.peek_has_action_items()
                    && ctx.active_item.read().is_some() =>
            {
                event.prevent_default();
                if let Some(active_id) = ctx.active_item.read().clone() {
                    let mut ctx = ctx;
                    ctx.open_action_panel(active_id);
                }
            }
            Key::ArrowDown => {
                event.prevent_default();
                ctx.select_next();
            }
            Key::ArrowUp => {
                event.prevent_default();
                ctx.select_prev();
            }
            Key::PageDown => {
                event.prevent_default();
                let steps = (ctx.page_size)();
                ctx.select_next_by(steps);
            }
            Key::PageUp => {
                event.prevent_default();
                let steps = (ctx.page_size)();
                ctx.select_prev_by(steps);
            }
            Key::Enter => {
                event.prevent_default();
                if let Some(hist) = try_use_context::<CommandHistoryContext>()
                    && let Some(data) = ctx.active_item_data()
                {
                    let value = data.value.clone().unwrap_or_else(|| data.id.clone());
                    hist.handle.push(&value);
                }
                ctx.confirm_selection();
            }
            Key::Escape => {
                event.prevent_default();
                let mut is_open = ctx.is_open;
                is_open.set(false);
            }
            Key::Home => {
                event.prevent_default();
                let visible = ctx.visible_item_ids.read();
                if let Some(first) = visible.first() {
                    scroll_item_into_view(ctx.instance_id, first);
                    let mut active = ctx.active_item;
                    active.set(Some(first.clone()));
                }
            }
            Key::End => {
                event.prevent_default();
                let visible = ctx.visible_item_ids.read();
                if let Some(last) = visible.last() {
                    scroll_item_into_view(ctx.instance_id, last);
                    let mut active = ctx.active_item;
                    active.set(Some(last.clone()));
                }
            }
            Key::Backspace => {
                let has_pages = ctx
                    .page_feature
                    .peek()
                    .is_some_and(|pf| !pf.page_stack.peek().is_empty());
                if ctx.search.read().is_empty() && has_pages {
                    event.prevent_default();
                    ctx.pop_page();
                }
            }
            _ => {
                if ctx.try_execute_shortcut(&event.key(), event.modifiers()) {
                    event.prevent_default();
                }
            }
        } // close match
    };

    let oninput = move |event: FormEvent| {
        let mut search = ctx.search;
        search.set(event.value());
    };

    let active_descendant = ctx
        .active_item
        .read()
        .as_ref()
        .map(|id| make_item_dom_id(ctx.instance_id, id))
        .unwrap_or_default();

    rsx! {
        div {
            role: "search",
            "data-cmdk-search": "true",
            input {
                id: make_input_id(ctx.instance_id),
                class: class.unwrap_or_default(),
                "data-cmdk-input": "true",
                "data-cmdk-no-drag": "true",
                r#type: "text",
                role: "combobox",
                "aria-haspopup": "listbox",
                "aria-expanded": if (ctx.is_open)() { "true" } else { "false" },
                "aria-controls": if (ctx.is_open)() { make_listbox_id(ctx.instance_id) } else { String::new() },
                "aria-activedescendant": active_descendant,
                "aria-autocomplete": "list",
                "aria-label": (ctx.label)().as_deref().unwrap_or("").to_string(),
                autocomplete: "off",
                autocorrect: "off",
                spellcheck: "false",
                placeholder: effective_placeholder.unwrap_or_default(),
                value: if let Some(v) = value { v() } else { (ctx.search)() },
                onmounted,
                onkeydown,
                oninput,
                onfocus: move |evt| { if let Some(h) = onfocus { h.call(evt); } },
                onblur:  move |evt| { if let Some(h) = onblur  { h.call(evt); } },
            }
        }
    }
}

/// Persistent, non-modal inline input for the command palette.
///
/// Unlike [`CommandInput`], this component:
/// - Ignores `is_open` (always active, no open/close lifecycle)
/// - Clears search on Escape instead of closing the palette
/// - Confirms selection AND clears search on Enter
/// - Does not pop pages on Backspace-when-empty
#[component]
pub fn CommandQuickInput(
    #[props(default)] placeholder: Option<String>,
    #[props(default)] class: Option<String>,
    #[props(default)] autofocus: bool,
    /// Controlled value. When provided, keeps `ctx.search` in sync with this signal.
    #[props(default)]
    value: Option<Signal<String>>,
) -> Element {
    let ctx: CommandContext = use_context();

    // P-013: Sync controlled value → ctx.search when signal changes
    use_effect(move || {
        if let Some(v) = value {
            let mut search = ctx.search;
            search.set(v());
        }
    });
    let mode = ctx.active_mode.read();
    let effective_placeholder = mode
        .as_ref()
        .and_then(|m| m.placeholder.clone())
        .or(placeholder.clone());
    drop(mode);

    let onmounted = move |event: MountedEvent| {
        let data = event.data();
        let mut el = ctx.input_element;
        el.set(Some(data.clone()));
        if autofocus {
            spawn(async move {
                let _ = data.set_focus(true).await;
            });
        }
    };

    let onkeydown = move |event: KeyboardEvent| {
        // IME composition guard: do not process keys during CJK/IME input
        if event.is_composing() {
            return;
        }
        // Vim bindings: Ctrl+N/J -> next, Ctrl+P/K -> prev
        if (ctx.vim_bindings)()
            && event.modifiers().contains(Modifiers::CONTROL)
            && let Key::Character(ref c) = event.key()
        {
            match c.to_lowercase().as_str() {
                "n" | "j" => {
                    event.prevent_default();
                    ctx.select_next();
                    return;
                }
                "p" | "k" => {
                    event.prevent_default();
                    ctx.select_prev();
                    return;
                }
                _ => {}
            }
        }
        match event.key() {
            Key::ArrowDown => {
                event.prevent_default();
                ctx.select_next();
            }
            Key::ArrowUp => {
                event.prevent_default();
                ctx.select_prev();
            }
            Key::Enter => {
                event.prevent_default();
                ctx.confirm_selection();
                let mut search = ctx.search;
                search.set(String::new());
            }
            Key::Escape => {
                event.prevent_default();
                let mut search = ctx.search;
                search.set(String::new());
            }
            Key::Home => {
                event.prevent_default();
                let visible = ctx.visible_item_ids.read();
                if let Some(first) = visible.first() {
                    scroll_item_into_view(ctx.instance_id, first);
                    let mut active = ctx.active_item;
                    active.set(Some(first.clone()));
                }
            }
            Key::End => {
                event.prevent_default();
                let visible = ctx.visible_item_ids.read();
                if let Some(last) = visible.last() {
                    scroll_item_into_view(ctx.instance_id, last);
                    let mut active = ctx.active_item;
                    active.set(Some(last.clone()));
                }
            }
            _ => {}
        } // close match
    };

    let oninput = move |event: FormEvent| {
        let mut search = ctx.search;
        search.set(event.value());
    };

    let active_descendant = ctx
        .active_item
        .read()
        .as_ref()
        .map(|id| make_item_dom_id(ctx.instance_id, id))
        .unwrap_or_default();

    rsx! {
        input {
            id: make_input_id(ctx.instance_id),
            class: class.unwrap_or_default(),
            "data-cmdk-quick-input": "true",
            "data-cmdk-no-drag": "true",
            r#type: "text",
            role: "combobox",
            "aria-expanded": "true",
            "aria-controls": make_listbox_id(ctx.instance_id),
            "aria-activedescendant": active_descendant,
            "aria-autocomplete": "list",
            autocomplete: "off",
            autocorrect: "off",
            spellcheck: "false",
            placeholder: effective_placeholder.unwrap_or_default(),
            value: if let Some(v) = value { v() } else { (ctx.search)() },
            onmounted,
            onkeydown,
            oninput,
        }
    }
}

/// Wraps the reference element that a floating [`CommandList`] positions against.
///
/// Follows the Radix `ComboboxAnchor` / `PopoverAnchor` compound pattern.
/// Stores its `MountedData` in [`CommandContext`] so `CommandList { floating: true }`
/// can measure the bounding rect for placement without additional props.
///
/// ## Optional
/// When no `CommandAnchor` is present, `CommandList { floating: true }` falls back
/// to measuring the [`CommandInput`] element directly.
///
/// ## Layout
/// Renders a single `<div data-cmdk-anchor>` wrapper. Add `class: "contents"` (or
/// equivalent) if you need the wrapper to be layout-transparent.
#[component]
pub fn CommandAnchor(children: Element, #[props(default)] class: Option<String>) -> Element {
    let ctx = use_context::<CommandContext>();

    let onmounted = move |event: MountedEvent| {
        let data = event.data();
        let mut ae = ctx.anchor_element;
        ae.set(Some(data.clone()));
    };

    rsx! {
        div {
            class: class.unwrap_or_default(),
            "data-cmdk-anchor": "true",
            onmounted,
            {children}
        }
    }
}

/// Container for the list of command items. Provides listbox role.
///
/// When rendered inside a `CommandSheet`, scroll events on the list will
/// temporarily lock drag gestures to prevent scroll/drag conflicts.
///
/// Set `floating: true` to position the list relative to the nearest
/// [`CommandAnchor`] (or [`CommandInput`] as fallback) using `position:fixed`.
#[component]
pub fn CommandList(
    children: Element,
    #[props(default)] label: Option<String>,
    #[props(default)] class: Option<String>,
    /// Position the list with `position:fixed` relative to the nearest
    /// `CommandAnchor` (or `CommandInput` as fallback). Measures the anchor's
    /// bounding rect each time the palette opens.
    ///
    /// Apply `data-side` CSS to style enter animations and item ordering:
    /// ```css
    /// [data-cmdk-list][data-side="top"] { flex-direction: column-reverse; }
    /// ```
    #[props(default = false)]
    floating: bool,
    /// Preferred opening direction when `floating = true`.
    /// Auto-flips on wasm32 if the preferred side has less space than the other.
    #[props(default)]
    preferred_side: Side,
    /// Gap in CSS pixels between the anchor edge and the floating list.
    #[props(default = 4.0_f64)]
    side_offset: f64,
    /// Enable virtual scrolling for large lists. Requires `features = ["virtualize"]`.
    /// When `true`, only items in the visible viewport range are rendered.
    #[props(default = false)]
    virtualize: bool,
    /// Height of each item in pixels. Only used when `virtualize = true`.
    /// All items must have the same height when virtualization is enabled.
    #[props(default = 40)]
    item_height: u32,
) -> Element {
    let ctx: CommandContext = use_context();

    // If inside a CommandSheet, coordinate scroll locking
    let sheet_ctx = try_use_context::<SheetDragContext>();

    // Virtual scroll state (only active when feature + prop enabled)
    #[cfg(feature = "virtualize")]
    let _scroll_top = use_signal(|| 0u32);
    #[cfg(feature = "virtualize")]
    let _container_height = use_signal(|| item_height * 20);
    #[cfg(feature = "virtualize")]
    let _virtualize = virtualize;
    #[cfg(feature = "virtualize")]
    let _item_height = item_height;

    // Suppress unused variable warnings when feature is not enabled
    #[cfg(not(feature = "virtualize"))]
    let _ = (virtualize, item_height);

    // ── Floating placement ──────────────────────────────────────────────────
    let computed_side: Signal<Side> = use_signal(|| preferred_side);
    let float_style: Signal<String> = use_signal(String::new);

    // In standalone mode (no CommandDialog/CommandSheet), nothing else sets
    // ctx.is_open. Set it when CommandList mounts so the floating placement
    // effect can run. Inside a dialog/sheet the redundant set(true)/set(false)
    // are harmless no-ops.
    use_hook(move || {
        let mut is_open = ctx.is_open;
        is_open.set(true);
    });
    use dioxus_core::use_drop;
    use_drop(move || {
        let mut is_open = ctx.is_open;
        is_open.set(false);
    });

    use_effect(move || {
        let is_open = (ctx.is_open)();
        // Read anchor_element BEFORE the early-return so this effect subscribes
        // to it. When CommandAnchor's onmounted fires after CommandList is already
        // mounted, this subscription causes the effect to re-run and measure the
        // correct rect.
        let anchor_ref = ctx.anchor_element.read().clone();
        if !floating || !is_open {
            return;
        }
        // Prefer explicit CommandAnchor; fall back to CommandInput element.
        let mounted = anchor_ref.or_else(|| ctx.input_element.read().clone());
        let Some(data) = mounted else { return };
        let pref = preferred_side;
        let offset = side_offset;
        spawn(async move {
            let Ok(rect) = data.get_client_rect().await else {
                return;
            };
            let vp_h = crate::helpers::get_viewport_height();
            let side = crate::placement::compute_side(pref, rect.min_y(), vp_h - rect.max_y());
            let style = crate::placement::compute_float_style(
                side,
                rect.min_x(),
                rect.min_y(),
                rect.max_y(),
                rect.size.width,
                offset,
                vp_h,
            );
            let mut cs = computed_side;
            cs.set(side);
            let mut fs = float_style;
            fs.set(style);
        });
    });

    let onscroll = move |_: Event<ScrollData>| {
        if let Some(ref _sctx) = sheet_ctx {
            #[cfg(target_arch = "wasm32")]
            {
                let now = crate::helpers::now_ms();
                let lock_until = now + _sctx.scroll_lock_timeout as f64;
                let mut state = _sctx.drag_state.borrow_mut();
                state.scroll_locked_until = lock_until;
            }
        }
    };

    rsx! {
        div {
            id: make_listbox_id(ctx.instance_id),
            class: class.unwrap_or_default(),
            "data-cmdk-list": "true",
            "data-cmdk-no-drag": "true",
            role: "listbox",
            "aria-label": label.unwrap_or_else(|| "Commands".to_string()),
            style: if floating { (float_style)() } else { String::new() },
            "data-side": if floating {
                match (computed_side)() {
                    Side::Bottom => "bottom",
                    Side::Top => "top",
                }
            } else { "" },
            onscroll,
            {children}
        }
    }
}

/// A single command item. Self-registers on mount and unregisters on drop.
#[component]
pub fn CommandItem(
    children: Element,
    #[props(into)] id: String,
    #[props(into)] label: String,
    #[props(default)] keywords: Vec<String>,
    #[props(default)] disabled: bool,
    #[props(default)] force_mount: bool,
    /// Semantic value sent to `on_select`. Falls back to `id` when `None`.
    #[props(default)]
    value: Option<String>,
    /// Keyboard shortcut that triggers this item when pressed while the palette is open.
    #[props(default)]
    shortcut: Option<Hotkey>,
    on_select: Option<EventHandler<String>>,
    #[props(default)] class: Option<String>,
    /// Controls whether this item participates in scoring.
    /// When `false`, the item is hidden from results entirely.
    /// Accepts reactive expressions: `visible: is_workout_active()`.
    #[props(default = true)]
    visible: bool,
    /// Additive score modifier for ranking adjustment.
    /// Positive = higher ranking, negative = lower.
    #[props(default = 0)]
    boost: i32,
    /// Mode this item belongs to. When a mode is active, only items
    /// with matching mode_id (or no mode_id) are visible.
    #[props(default)]
    mode_id: Option<String>,
    /// P-015: Fired when this item mounts. Receives the item's id.
    on_mount: Option<EventHandler<String>>,
    /// P-015: Fired when this item unmounts. Receives the item's id.
    /// Note: `data-leaving` / deferred unmount is deferred to Wave 6.
    on_unmount: Option<EventHandler<String>>,
    /// P-039: Optional action panel element for this item.
    /// Rendered as a child slot when the action panel is open for this item.
    #[props(default)]
    actions: Option<Element>,
) -> Element {
    let ctx: CommandContext = use_context();
    let group_id = try_use_context::<GroupId>().map(|g| g.0);
    let page_id = try_use_context::<PageId>().map(|p| p.0);
    let item_id = id.clone();

    // P-029: Provide ItemId via context so child CommandHighlight components
    // can auto-read the parent item's ID without requiring an explicit prop.
    use_context_provider(|| ItemId(item_id.clone()));

    // P-015: Track enter animation state.
    // Starts as `true` on first render frame, then transitions to `false` after one tick.
    // Skipped entirely if prefers_reduced_motion() returns true.
    let entering = use_signal(|| !crate::helpers::prefers_reduced_motion());

    // P-015 deferred: Track leaving state for data-leaving attribute.
    // Set briefly when item transitions from visible to hidden.
    let leaving = use_signal(|| false);

    // P-015: Fire on_mount callback and clear data-entering after one tick.
    {
        let item_id_mount = item_id.clone();
        use_effect(move || {
            if let Some(ref handler) = on_mount {
                handler.call(item_id_mount.clone());
            }
            // Clear the entering state after the first render so CSS transitions fire.
            // spawn() schedules after the current render cycle.
            if entering() {
                let mut e = entering;
                spawn(async move {
                    e.set(false);
                });
            }
        });
    }

    // P-015: Fire on_unmount callback when item drops.
    {
        let item_id_drop_cb = item_id.clone();
        use_drop(move || {
            if let Some(ref handler) = on_unmount {
                handler.call(item_id_drop_cb.clone());
            }
        });
    }

    // P-015 deferred: Track visible→hidden transition for data-leaving.
    {
        let item_id_vis = item_id.clone();
        let prev_visible = use_hook(|| Rc::new(RefCell::new(true)));
        use_effect(move || {
            let is_vis = ctx.visible_item_set.read().contains(&item_id_vis);
            let was_vis = *prev_visible.borrow();
            if was_vis && !is_vis && !crate::helpers::prefers_reduced_motion() {
                let mut lv = leaving;
                lv.set(true);
                spawn(async move {
                    lv.set(false);
                });
            }
            *prev_visible.borrow_mut() = is_vis;
        });
    }

    // Register on mount
    use_hook({
        let reg = ItemRegistration {
            id: item_id.clone(),
            label: label.clone(),
            keywords_cached: keywords.join(" "),
            keywords: keywords.clone(),
            group_id: group_id.clone(),
            disabled,
            force_mount,
            value: value.clone(),
            shortcut: shortcut.clone(),
            page_id: page_id.clone(),
            hidden: !visible,
            boost,
            mode_id: mode_id.clone(),
            on_select: on_select.as_ref().map(|h| ItemSelectCallback(*h)),
        };
        let ctx_copy = ctx;
        move || {
            ctx_copy.register_item(reg);
        }
    });

    // Unregister on drop
    let item_id_drop = item_id.clone();
    use_drop(move || {
        ctx.unregister_item(&item_id_drop);
    });

    // Update registration when props change
    use_effect({
        let item_id = item_id.clone();
        let label = label.clone();
        let keywords = keywords.clone();
        let group_id = group_id.clone();
        let value = value.clone();
        let shortcut = shortcut.clone();
        let page_id = page_id.clone();
        let mode_id = mode_id.clone();
        let on_select_cb = on_select.as_ref().map(|h| ItemSelectCallback(*h));
        move || {
            let mut items = ctx.items;
            let mut items_write = items.write();
            if let Some(existing_rc) = items_write.iter_mut().find(|i| i.id == item_id) {
                // P-051: use Rc::make_mut for in-place prop sync (clones only if shared)
                let existing = std::rc::Rc::make_mut(existing_rc);
                existing.label = label.clone();
                existing.keywords_cached = keywords.join(" ");
                existing.keywords = keywords.clone();
                existing.group_id = group_id.clone();
                existing.disabled = disabled;
                existing.force_mount = force_mount;
                existing.value = value.clone();
                existing.shortcut = shortcut.clone();
                existing.page_id = page_id.clone();
                existing.hidden = !visible;
                existing.boost = boost;
                existing.mode_id = mode_id.clone();
                existing.on_select = on_select_cb.clone();
                // Index is keyed by id — id doesn't change on prop updates, no rebuild needed.
            }
        }
    });

    let is_visible = ctx.is_item_visible(&item_id);
    let is_active = ctx
        .active_item
        .read()
        .as_ref()
        .is_some_and(|a| a == &item_id);
    // P-039: Whether the action panel is open for this specific item.
    let panel_open = ctx
        .read_action_panel()
        .is_some_and(|s| s.item_id == item_id);

    let dom_id = make_item_dom_id(ctx.instance_id, &item_id);

    let resolved_value = value.clone().unwrap_or_else(|| item_id.clone());

    let onclick = {
        let item_id = item_id.clone();
        let resolved_value = resolved_value.clone();
        move |_: MouseEvent| {
            if disabled {
                return;
            }
            let mut active = ctx.active_item;
            active.set(Some(item_id.clone()));
            if let Some(ref handler) = on_select {
                handler.call(resolved_value.clone());
            } else {
                ctx.confirm_selection();
            }
        }
    };

    let onpointermove = {
        let item_id = item_id.clone();
        move |_: PointerEvent| {
            if disabled || (ctx.disable_pointer_selection)() {
                return;
            }
            let current = ctx.active_item.peek().clone();
            if current.as_deref() != Some(&item_id) {
                let mut active = ctx.active_item;
                active.set(Some(item_id.clone()));
            }
        }
    };

    rsx! {
        div {
            id: dom_id,
            class: class.unwrap_or_default(),
            "data-cmdk-item": "true",
            "data-value": "{resolved_value}",
            "data-disabled": if disabled { "true" } else { "" },
            // data-focused: item is the current keyboard-navigation position
            "data-focused": if is_active { "true" } else { "" },
            // data-active: alias for data-focused (backwards-compat style hook)
            "data-active": if is_active { "true" } else { "" },
            // P-015: Present on first render frame for CSS enter animations.
            // Removed after one tick (spawned async). Skipped when prefers-reduced-motion.
            "data-entering": if entering() { "true" } else { "" },
            // P-015 deferred: Set briefly when item transitions visible→hidden.
            "data-leaving": if leaving() { "true" } else { "" },
            role: "option",
            "aria-selected": if is_active { "true" } else { "false" },
            "aria-disabled": if disabled { "true" } else { "false" },
            // P-039: Whether the action panel is open for this item.
            "data-panel-open": if panel_open { "true" } else { "" },
            // Hide via CSS attribute, never unmount
            "data-hidden": if !is_visible { "true" },
            style: if !is_visible { "display:none;" },
            onclick,
            onpointermove,
            {children}
            // P-039: Render action panel slot when open for this item.
            if panel_open
                && let Some(action_el) = actions
            {
                {action_el}
            }
        }
    }
}

/// A group of command items with an optional heading.
#[component]
pub fn CommandGroup(
    children: Element,
    #[props(into)] id: String,
    #[props(default)] heading: Option<String>,
    #[props(default)] class: Option<String>,
    /// When `true`, the group is always visible regardless of whether it has
    /// any matching items. Useful for groups that contain non-filterable content.
    #[props(default = false)]
    force_mount: bool,
) -> Element {
    let ctx: CommandContext = use_context();
    let group_id = id.clone();

    // Register group
    use_hook({
        let reg = GroupRegistration {
            id: group_id.clone(),
            heading: heading.clone(),
            force_mount,
        };
        let ctx_copy = ctx;
        move || {
            ctx_copy.register_group(reg);
        }
    });

    let group_id_drop = group_id.clone();
    use_drop(move || {
        ctx.unregister_group(&group_id_drop);
    });

    // Provide GroupId context for child items
    use_context_provider({
        let gid = group_id.clone();
        move || GroupId(gid)
    });

    let is_visible = ctx.is_group_visible(&group_id);
    let heading_id = format!("cmdk-group-heading-{}-{group_id}", ctx.instance_id);

    rsx! {
        div {
            class: class.unwrap_or_default(),
            "data-cmdk-group": "true",
            "data-value": "{group_id}",
            role: "presentation",
            "data-hidden": if !is_visible { "true" },
            style: if !is_visible { "display:none;" },

            if let Some(ref h) = heading {
                div {
                    id: "{heading_id}",
                    "data-cmdk-group-heading": "true",
                    "aria-hidden": "true",
                    "{h}"
                }
            }

            div {
                role: "group",
                "aria-labelledby": if heading.is_some() { heading_id.clone() },
                {children}
            }
        }
    }
}

/// A page container for multi-step command navigation.
///
/// Items nested inside a `CommandPage` are only visible when that page is the
/// active page on the navigation stack. Use `use_command_pages()` to push/pop pages.
///
/// # Example
///
/// ```rust,ignore
/// CommandPage { id: "exercises",
///     CommandItem { id: "squat", label: "Squat", "Squat" }
///     CommandItem { id: "bench", label: "Bench Press", "Bench Press" }
/// }
/// ```
#[component]
pub fn CommandPage(
    children: Element,
    #[props(into)] id: String,
    #[props(default)] title: Option<String>,
    #[props(default)] class: Option<String>,
) -> Element {
    let ctx: CommandContext = use_context();
    let page_id = id.clone();

    // Register page on mount
    use_hook({
        let reg = PageRegistration {
            id: page_id.clone(),
            title: title.clone(),
        };
        let ctx_copy = ctx;
        move || {
            ctx_copy.register_page(reg);
        }
    });

    // Unregister on drop
    let page_id_drop = page_id.clone();
    use_drop(move || {
        ctx.unregister_page(&page_id_drop);
    });

    // Provide PageId context for child items
    use_context_provider({
        let pid = page_id.clone();
        move || PageId(pid)
    });

    let is_active = ctx.is_page_active(&page_id);

    rsx! {
        div {
            class: class.unwrap_or_default(),
            "data-cmdk-page": "true",
            "data-value": "{page_id}",
            "data-hidden": if !is_active { "true" },
            style: if !is_active { "display:none;" },
            {children}
        }
    }
}

/// Shown when there are no matching results.
#[component]
pub fn CommandEmpty(children: Element, #[props(default)] class: Option<String>) -> Element {
    let ctx: CommandContext = use_context();
    let count = (ctx.filtered_count)();

    if count > 0 {
        return rsx! {};
    }

    rsx! {
        div {
            class: class.unwrap_or_default(),
            "data-cmdk-empty": "true",
            role: "status",
            "aria-live": "polite",
            "aria-atomic": "true",
            {children}
        }
    }
}

/// Visual separator between items or groups.
///
/// When `group_before` or `group_after` is set, the separator auto-hides
/// if either referenced group has no visible items. When both are `None`,
/// the separator renders unconditionally (backward compatible).
///
/// Set `always_render = true` to override the auto-hide logic and always
/// render the separator regardless of adjacent group visibility.
#[component]
pub fn CommandSeparator(
    #[props(default)] class: Option<String>,
    /// Group ID before this separator. Hides if this group has no visible items.
    #[props(default)]
    group_before: Option<String>,
    /// Group ID after this separator. Hides if this group has no visible items.
    #[props(default)]
    group_after: Option<String>,
    /// When `true`, always renders regardless of adjacent group visibility.
    /// Overrides the auto-hide behavior driven by `group_before`/`group_after`.
    #[props(default = false)]
    always_render: bool,
) -> Element {
    // Auto-hide when either adjacent group is hidden (unless always_render overrides)
    if !always_render && (group_before.is_some() || group_after.is_some()) {
        let ctx: CommandContext = use_context();
        let before_hidden = group_before
            .as_deref()
            .is_some_and(|g| !ctx.is_group_visible(g));
        let after_hidden = group_after
            .as_deref()
            .is_some_and(|g| !ctx.is_group_visible(g));
        if before_hidden || after_hidden {
            return rsx! {};
        }
    }

    rsx! {
        div {
            class: class.unwrap_or_default(),
            "data-cmdk-separator": "true",
            role: "separator",
        }
    }
}

/// Loading indicator shown when is_loading is true.
///
/// When `progress` is `None` (default), renders `role="status"` with `aria-busy="true"`.
/// When `progress` is `Some(value)`, renders `role="progressbar"` with `aria-valuenow`,
/// `aria-valuemin="0"`, and `aria-valuemax="100"`. Values are clamped to `0.0–100.0`.
#[component]
pub fn CommandLoading(
    children: Element,
    #[props(default)] class: Option<String>,
    /// Progress value (0.0–100.0). When `Some`, renders `role="progressbar"` with
    /// ARIA value attributes. When `None` (default), renders `role="status"` with
    /// `aria-busy="true"`.
    #[props(default)]
    progress: Option<f32>,
) -> Element {
    let ctx: CommandContext = use_context();
    let loading = (ctx.is_loading)();

    if !loading {
        return rsx! {};
    }

    let clamped = progress.map(|p| p.clamp(0.0, 100.0));
    let is_progressbar = clamped.is_some();

    rsx! {
        div {
            class: class.unwrap_or_default(),
            "data-cmdk-loading": "true",
            role: if is_progressbar { "progressbar" } else { "status" },
            "aria-busy": if is_progressbar { "" } else { "true" },
            "aria-label": if is_progressbar { "" } else { "Loading" },
            "aria-valuenow": if let Some(v) = clamped { v.to_string() } else { String::new() },
            "aria-valuemin": if is_progressbar { "0".to_string() } else { String::new() },
            "aria-valuemax": if is_progressbar { "100".to_string() } else { String::new() },
            {children}
        }
    }
}

/// Displays a keyboard shortcut hint. Hidden from screen readers.
#[component]
pub fn CommandShortcut(children: Element, #[props(default)] class: Option<String>) -> Element {
    rsx! {
        kbd {
            class: class.unwrap_or_default(),
            "data-cmdk-shortcut": "true",
            "aria-hidden": "true",
            {children}
        }
    }
}

// ---------------------------------------------------------------------------
// CommandHighlight — fuzzy match highlighting
// ---------------------------------------------------------------------------

/// Helper function to render a label with matched characters in `<mark>` elements.
fn render_highlighted_label(label: &str, indices: Option<&Vec<u32>>, mark_class: &str) -> Element {
    let Some(indices) = indices else {
        // No match data — render plain text
        return rsx! { "{label}" };
    };

    let index_set: HashSet<u32> = indices.iter().copied().collect();

    // Build spans: consecutive matched chars grouped into <mark>,
    // consecutive unmatched chars grouped into plain text
    let chars: Vec<char> = label.chars().collect();
    let mut fragments = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        let is_match = index_set.contains(&(i as u32));
        let start = i;
        while i < chars.len() && index_set.contains(&(i as u32)) == is_match {
            i += 1;
        }
        let segment: String = chars[start..i].iter().collect();
        fragments.push((segment, is_match));
    }

    rsx! {
        for (idx, (text, highlighted)) in fragments.iter().enumerate() {
            if *highlighted {
                mark {
                    key: "{idx}",
                    class: "{mark_class}",
                    "data-cmdk-match": "true",
                    "{text}"
                }
            } else {
                span {
                    key: "{idx}",
                    "{text}"
                }
            }
        }
    }
}

/// Renders a label with fuzzy-matched characters highlighted.
///
/// Reads the item's `match_indices` from `CommandContext::scored_items`
/// and wraps matched characters in `<mark>` elements.
///
/// Must be used inside a `CommandRoot` tree.
///
/// ## Auto-read from context (P-029)
///
/// When placed directly inside a `CommandItem`, the item ID is automatically
/// threaded via context — you do not need to pass an explicit `id` prop.
/// The explicit prop takes precedence when provided.
///
/// You may also pass `match_indices` directly to override the context lookup.
///
/// # Example
///
/// ```rust,ignore
/// // Explicit id (original usage, still works):
/// CommandItem { id: "settings", label: "Settings",
///     CommandHighlight { id: "settings", label: "Settings", class: "highlight" }
/// }
///
/// // Auto-read from parent CommandItem context (no id prop needed):
/// CommandItem { id: "settings", label: "Settings",
///     CommandHighlight { label: "Settings", class: "highlight" }
/// }
/// ```
#[component]
pub fn CommandHighlight(
    /// The item ID to look up match indices for.
    /// When omitted, falls back to the parent `CommandItem`'s ID via context.
    #[props(default, into)]
    id: Option<String>,
    /// The label text to render with highlights.
    #[props(into)]
    label: String,
    /// CSS class for the container span.
    #[props(default)]
    class: Option<String>,
    /// CSS class applied to matched `<mark>` elements.
    #[props(default)]
    mark_class: Option<String>,
    /// Explicit match positions. When provided, takes precedence over the
    /// context lookup from `CommandContext::scored_items`.
    #[props(default)]
    match_indices: Option<Vec<u32>>,
) -> Element {
    let item_id_ctx = try_use_context::<ItemId>();
    let ctx = try_use_context::<CommandContext>();

    // Resolve item ID: explicit prop wins, then fall back to parent CommandItem context.
    let resolved_id = id.or_else(|| item_id_ctx.map(|c| c.0));

    // Resolve match_indices: explicit prop wins, then context lookup.
    let resolved_indices: Option<Vec<u32>> = if let Some(indices) = match_indices {
        // Explicit prop supplied — use it directly.
        Some(indices)
    } else if let (Some(iid), Some(ctx)) = (&resolved_id, &ctx) {
        // Fall back to scored_items lookup via context.
        ctx.scored_items
            .read()
            .iter()
            .find(|s| &s.id == iid)
            .and_then(|s| s.match_indices.clone())
    } else {
        None
    };

    let mark_cls = mark_class.unwrap_or_default();

    rsx! {
        span {
            class: class.unwrap_or_default(),
            "data-cmdk-highlight": "true",
            {render_highlighted_label(&label, resolved_indices.as_ref(), &mark_cls)}
        }
    }
}

// ---------------------------------------------------------------------------
// CommandPreview — preview pane
// ---------------------------------------------------------------------------

/// A slot for rendering preview content based on the active item.
///
/// Place alongside `CommandList` inside a `CommandRoot`. The consumer
/// accesses the active item through `CommandContext` directly
/// (via `use_context::<CommandContext>()`).
///
/// # Example
///
/// ```rust,ignore
/// CommandRoot { on_select: handler,
///     CommandInput { placeholder: "Search exercises..." }
///     div { class: "flex",
///         CommandList { /* items */ }
///         CommandPreview { class: "preview-pane",
///             // children render based on active item
///         }
///     }
/// }
/// ```
#[component]
pub fn CommandPreview(
    /// Content to render in the preview area.
    children: Element,
    /// CSS class for the preview container.
    #[props(default)]
    class: Option<String>,
) -> Element {
    let ctx: CommandContext = use_context();
    let active = ctx.active_item.read().clone();

    rsx! {
        div {
            class: class.unwrap_or_default(),
            "data-cmdk-preview": "true",
            "data-cmdk-has-active": active.is_some(),
            {children}
        }
    }
}

// ---------------------------------------------------------------------------
// CommandModeIndicator — mode pill
// ---------------------------------------------------------------------------

/// Renders the active mode as a headless indicator pill.
///
/// Shows the mode's label when a prefix activates a mode.
/// Hidden when no mode is active.
///
/// # Example
///
/// ```rust,ignore
/// CommandInput {
///     CommandModeIndicator { class: "mode-pill" }
/// }
/// ```
#[component]
pub fn CommandModeIndicator(#[props(default)] class: Option<String>) -> Element {
    let ctx: CommandContext = use_context();
    let mode = ctx.active_mode.read();

    match &*mode {
        Some(m) => {
            let mode_id = m.id.clone();
            let label = m.label.clone();
            rsx! {
                span {
                    class: class.unwrap_or_default(),
                    "data-cmdk-mode": "true",
                    "data-cmdk-mode-id": "{mode_id}",
                    "{label}"
                }
            }
        }
        None => rsx! {},
    }
}

/// Modal dialog wrapper with focus trap, backdrop, and focus restore.
#[component]
pub fn CommandDialog(
    children: Element,
    open: Signal<bool>,
    on_open_change: Option<EventHandler<bool>>,
    #[props(default)] label: Option<String>,
    #[props(default)] class: Option<String>,
    #[props(default)] overlay_class: Option<String>,
    #[props(default)] content_class: Option<String>,
    /// P-015: Fired when the dialog's open animation ends (animationend event).
    /// Only fires on wasm32 targets. Skipped when prefers-reduced-motion.
    on_open_animation_end: Option<EventHandler<()>>,
    /// P-015: Fired when the dialog's close animation ends.
    /// Note: deferred unmount (keeping dialog in DOM during close animation) is Wave 6.
    on_close_animation_end: Option<EventHandler<()>>,
    /// Duration in milliseconds to keep the dialog mounted after closing,
    /// to allow CSS exit animations to play. 0 = immediate unmount (default).
    /// Ignored when `prefers_reduced_motion()` returns `true`.
    #[props(default = 0)]
    animation_duration_ms: u32,
) -> Element {
    // Provide the palette handle via context so children can close the dialog
    use_context_provider(|| CommandPaletteHandle { open });

    let is_open = open();

    // Focus trap: refocus input when tab guards receive focus
    let ctx = try_use_context::<CommandContext>();

    let refocus_input = move |_: FocusEvent| {
        if let Some(ctx) = ctx
            && let Some(ref el) = *ctx.input_element.read()
        {
            let el = el.clone();
            spawn(async move {
                let _ = el.set_focus(true).await;
            });
        }
    };

    // Handle backdrop click
    let on_backdrop_click = move |_: MouseEvent| {
        let mut o = open;
        o.set(false);
        if let Some(ref handler) = on_open_change {
            handler.call(false);
        }
    };

    // P-028: Save focus on open, restore on close.
    // wasm32: uses web_sys to read document.activeElement.id and restore by getElementById.
    // non-wasm: falls back to document::eval for desktop/mobile compatibility.
    use_effect(move || {
        if is_open {
            // P-028: Save the DOM id of the previously focused element
            #[cfg(target_arch = "wasm32")]
            {
                // REVIEW(web_sys): no Dioxus 0.7 API to query currently-focused element
                if let Some(ctx) = ctx
                    && let Some(window) = web_sys::window()
                    && let Some(doc) = window.document()
                    && let Some(active) = doc.active_element()
                {
                    let id_val = active.id();
                    let stored_id = if id_val.is_empty() {
                        None
                    } else {
                        Some(id_val)
                    };
                    let mut fb = ctx.focused_before_id;
                    fb.set(stored_id);
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                // Desktop/mobile: use eval as fallback (no native focused-element API)
                document::eval("window.__cmdk_prev_focused = document.activeElement");
            }
            // Focus the command input
            if let Some(ref ctx) = ctx
                && let Some(ref el) = *ctx.input_element.read()
            {
                let el = el.clone();
                spawn(async move {
                    let _ = el.set_focus(true).await;
                });
            }
        } else {
            // P-028: Restore focus
            #[cfg(target_arch = "wasm32")]
            {
                // REVIEW(web_sys): focus restore requires web_sys on wasm32
                if let Some(ctx) = ctx {
                    let saved_id = ctx.focused_before_id.peek().clone();
                    if let Some(id) = saved_id {
                        if let Some(window) = web_sys::window()
                            && let Some(doc) = window.document()
                            && let Some(el) = doc.get_element_by_id(&id)
                        {
                            use wasm_bindgen::JsCast;
                            if let Ok(html_el) = el.dyn_into::<web_sys::HtmlElement>() {
                                let _ = html_el.focus();
                            }
                        }
                        let mut fb = ctx.focused_before_id;
                        fb.set(None);
                    }
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                // Restore focus on non-wasm via eval
                document::eval(
                    "window.__cmdk_prev_focused?.focus(); window.__cmdk_prev_focused = null;",
                );
            }
        }
    });

    // Handle Escape in the dialog
    let on_dialog_keydown = move |event: KeyboardEvent| {
        if event.key() == Key::Escape {
            event.prevent_default();
            let mut o = open;
            o.set(false);
            if let Some(ref handler) = on_open_change {
                handler.call(false);
            }
        }
    };

    // P-015: Track enter animation state for data-entering attribute.
    // Starts true on open, cleared after one tick. Skipped for prefers-reduced-motion.
    let dialog_entering = use_signal(|| false);

    // P-015 deferred: Deferred unmount — keep dialog in DOM during close animation.
    let mut should_render = use_signal(|| is_open);
    let dur = animation_duration_ms;

    use_effect(move || {
        let open_now = open();
        if open_now {
            // Opening: render immediately, trigger enter animation
            should_render.set(true);
            if !crate::helpers::prefers_reduced_motion() {
                let mut e = dialog_entering;
                e.set(true);
                spawn(async move {
                    e.set(false);
                });
            }
            if let Some(ref handler) = on_open_animation_end {
                let h = *handler;
                spawn(async move {
                    h.call(());
                });
            }
        } else {
            // Closing: defer unmount if animation_duration_ms > 0
            if let Some(ref handler) = on_close_animation_end {
                let h = *handler;
                spawn(async move {
                    h.call(());
                });
            }
            #[cfg(target_arch = "wasm32")]
            {
                if dur > 0 && !crate::helpers::prefers_reduced_motion() {
                    let mut sr = should_render;
                    spawn(async move {
                        gloo_timers::future::TimeoutFuture::new(dur).await;
                        sr.set(false);
                    });
                } else {
                    should_render.set(false);
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = dur;
                should_render.set(false);
            }
        }
    });

    if !should_render() {
        return rsx! {};
    }

    rsx! {
        // Backdrop overlay
        div {
            class: overlay_class.unwrap_or_default(),
            "data-cmdk-overlay": "true",
            style: "position:fixed;inset:0;z-index:9998;",
            onclick: on_backdrop_click,
        }

        // Dialog container
        div {
            class: class.unwrap_or_default(),
            "data-cmdk-dialog": "true",
            // P-015: Present on first open frame for CSS enter animations.
            // Removed after one tick. Skipped when prefers-reduced-motion.
            "data-entering": if dialog_entering() { "true" } else { "" },
            // P-015 deferred: "open" while open prop is true, "closed" during deferred unmount.
            "data-state": if open() { "open" } else { "closed" },
            role: "dialog",
            "aria-modal": "true",
            "aria-label": label.as_deref().unwrap_or("Command palette"),
            style: "position:fixed;z-index:9999;",
            tabindex: "-1",
            onkeydown: on_dialog_keydown,

            // Tab guard (start)
            div {
                tabindex: "0",
                "aria-hidden": "true",
                style: "position:fixed;top:0;left:0;width:1px;height:0;padding:0;margin:-1px;overflow:hidden;clip:rect(0,0,0,0);white-space:nowrap;border-width:0;",
                onfocus: refocus_input,
            }

            div {
                class: content_class.unwrap_or_default(),
                "data-cmdk-dialog-content": "true",
                {children}
            }

            // Tab guard (end)
            div {
                tabindex: "0",
                "aria-hidden": "true",
                style: "position:fixed;top:0;left:0;width:1px;height:0;padding:0;margin:-1px;overflow:hidden;clip:rect(0,0,0,0);white-space:nowrap;border-width:0;",
                onfocus: refocus_input,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// CommandSheet — mobile bottom-sheet wrapper
// ---------------------------------------------------------------------------

/// Context provided by CommandSheet so CommandList can coordinate scroll locking.
#[derive(Clone)]
#[allow(dead_code)]
pub(crate) struct SheetDragContext {
    pub drag_state: Rc<RefCell<DragState>>,
    pub scroll_lock_timeout: u32,
}

/// Unique DOM ID for the sheet element.
#[allow(dead_code)]
fn make_sheet_id(instance_id: u32) -> String {
    format!("cmdk-sheet-{instance_id}")
}

/// Mobile bottom-sheet wrapper for the command palette.
///
/// Renders a draggable sheet that slides up from the bottom. Supports snap
/// points, velocity-based flick-to-dismiss, and integrates with the same
/// `CommandRoot` / `CommandInput` / `CommandList` children used in `CommandDialog`.
///
/// # Example
///
/// ```rust,ignore
/// let sheet = use_command_palette(false);
///
/// rsx! {
///     CommandSheet {
///         open: sheet.open,
///         snap_points: vec![0.5, 1.0],
///         CommandRoot {
///             on_select: move |v: String| { sheet.hide(); },
///             CommandInput { placeholder: "Search..." }
///             CommandList {
///                 CommandEmpty { "No results." }
///             }
///         }
///     }
/// }
/// ```
#[component]
pub fn CommandSheet(
    children: Element,
    /// Controlled open state.
    open: Signal<bool>,
    /// Fired when the sheet opens or closes.
    on_open_change: Option<EventHandler<bool>>,
    /// ARIA label for the sheet dialog.
    #[props(default)]
    label: Option<String>,
    /// CSS class for the sheet container.
    #[props(default)]
    class: Option<String>,
    /// CSS class for the backdrop overlay.
    #[props(default)]
    overlay_class: Option<String>,
    /// CSS class for the content wrapper inside the sheet.
    #[props(default)]
    content_class: Option<String>,
    /// Snap point ratios (0.0–1.0). E.g. `vec![0.5, 1.0]` = half and full.
    #[props(default)]
    snap_points: Option<Vec<f32>>,
    /// Ratio (0.0–1.0) of sheet height that must be dragged down to dismiss.
    #[props(default = 0.5)]
    close_threshold: f32,
    /// Milliseconds after a scroll event before drag gestures re-enable.
    #[props(default = 500)]
    scroll_lock_timeout: u32,
    /// Whether the sheet can be dismissed by gesture / escape / backdrop click.
    #[props(default = true)]
    dismissible: bool,
    /// Restrict drag initiation to the handle element only.
    #[props(default = false)]
    handle_only: bool,
    /// Fired when the sheet settles on a new snap point.
    on_snap_point_change: Option<EventHandler<usize>>,
    /// Focus the command input when the sheet opens.
    #[props(default = false)]
    autofocus_on_open: bool,
    /// Show an accessible close button in the sheet header. Defaults to `true`.
    #[props(default = true)]
    show_close_button: bool,
    /// ARIA label for the close button.
    #[props(default = "Close".to_string())]
    close_button_label: String,
    /// P-015: Fired when the sheet's open animation ends.
    /// One-tick approximation; full animationend wiring available on wasm32.
    on_open_animation_end: Option<EventHandler<()>>,
    /// P-015: Fired when the sheet's close animation ends.
    /// Note: deferred unmount (keeping sheet in DOM during close animation) is Wave 6.
    on_close_animation_end: Option<EventHandler<()>>,
    /// Duration in milliseconds to keep the sheet mounted after closing,
    /// to allow CSS exit animations to play. 0 = immediate unmount (default).
    /// Ignored when `prefers_reduced_motion()` returns `true`.
    #[props(default = 0)]
    animation_duration_ms: u32,
) -> Element {
    let instance_id = use_hook(crate::helpers::next_instance_id);
    let sheet_dom_id = make_sheet_id(instance_id);

    // Provide palette handle via context (same pattern as CommandDialog)
    use_context_provider(|| CommandPaletteHandle { open });

    let is_open = open();

    // P-005: Respect prefers-reduced-motion — skip sheet transitions
    let transition_css = if crate::helpers::prefers_reduced_motion() {
        "transform 0s"
    } else {
        "transform 0.3s cubic-bezier(0.32, 0.72, 0, 1)"
    };

    // Effective snap points — default to [1.0] (fully open)
    let effective_snaps = snap_points.clone().unwrap_or_else(|| vec![1.0]);

    // Reactive signal for "is a drag in progress" — used by RSX to skip transform
    let is_dragging = use_signal(|| false);

    // Current snap point index (reactive for settled state)
    let current_snap_idx = use_signal(|| effective_snaps.len().saturating_sub(1));

    // Non-reactive mutable drag state (Rc<RefCell> pattern from context.rs)
    let drag_state = use_hook(|| Rc::new(RefCell::new(DragState::default())));

    // Provide SheetDragContext so CommandList can coordinate scroll locking
    let drag_ctx = SheetDragContext {
        drag_state: drag_state.clone(),
        scroll_lock_timeout,
    };
    use_context_provider(move || drag_ctx.clone());

    // Focus trap: refocus input on tab guard focus
    let ctx = try_use_context::<CommandContext>();

    let refocus_input = move |_: FocusEvent| {
        if let Some(ctx) = ctx
            && let Some(ref el) = *ctx.input_element.read()
        {
            let el = el.clone();
            spawn(async move {
                let _ = el.set_focus(true).await;
            });
        }
    };

    // Dismiss helper
    let dismiss = move || {
        if !dismissible {
            return;
        }
        let mut o = open;
        o.set(false);
        if let Some(ref handler) = on_open_change {
            handler.call(false);
        }
    };

    // Backdrop click
    let on_backdrop_click = move |_: MouseEvent| {
        dismiss();
    };

    // Escape key
    let on_sheet_keydown = move |event: KeyboardEvent| {
        if event.key() == Key::Escape {
            event.prevent_default();
            dismiss();
        }
    };

    // P-028: Save/restore focus on open/close.
    // wasm32: Stores HtmlElement ref in Rc<RefCell> (reliable even without id attribute)
    // and also stores the element's id into ctx.focused_before_id (for observability/tests).
    // non-wasm: falls back to document::eval.
    #[cfg(target_arch = "wasm32")]
    let saved_focus: Rc<RefCell<Option<web_sys::HtmlElement>>> =
        use_hook(|| Rc::new(RefCell::new(None)));

    use_effect(move || {
        if is_open {
            // P-028: Save currently focused element
            #[cfg(target_arch = "wasm32")]
            {
                use wasm_bindgen::JsCast;
                if let Some(window) = web_sys::window()
                    && let Some(doc) = window.document()
                    && let Some(active) = doc.active_element()
                    && let Ok(html_el) = active.dyn_into::<web_sys::HtmlElement>()
                {
                    // Store id in context signal for observability
                    // REVIEW(web_sys): no Dioxus 0.7 API to query currently-focused element
                    if let Some(ref ctx_ref) = ctx {
                        let id_val = html_el.id();
                        let stored_id = if id_val.is_empty() {
                            None
                        } else {
                            Some(id_val)
                        };
                        let mut fb = ctx_ref.focused_before_id;
                        fb.set(stored_id);
                    }
                    *saved_focus.borrow_mut() = Some(html_el);
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            document::eval("window.__cmdk_prev_focused = document.activeElement");

            if autofocus_on_open
                && let Some(ref ctx) = ctx
                && let Some(ref el) = *ctx.input_element.read()
            {
                let el = el.clone();
                spawn(async move {
                    let _ = el.set_focus(true).await;
                });
            }
        } else {
            // P-028: Restore focus to previously focused element
            #[cfg(target_arch = "wasm32")]
            {
                // REVIEW(web_sys): focus restore requires web_sys on wasm32
                if let Some(el) = saved_focus.borrow_mut().take() {
                    let _ = el.focus();
                }
                // Clear the context signal
                if let Some(ref ctx_ref) = ctx {
                    let mut fb = ctx_ref.focused_before_id;
                    fb.set(None);
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            document::eval(
                "window.__cmdk_prev_focused?.focus(); window.__cmdk_prev_focused = null;",
            );
        }
    });

    // Cache sheet element on mount (for imperative style updates during drag)
    let onmounted = {
        let sheet_dom_id_clone = sheet_dom_id.clone();
        let drag_state_mount = drag_state.clone();
        move |_: MountedEvent| {
            // Suppress unused-variable warnings on non-wasm
            let _ = &sheet_dom_id_clone;
            let _ = &drag_state_mount;
            #[cfg(target_arch = "wasm32")]
            {
                if let Some(window) = web_sys::window()
                    && let Some(document) = window.document()
                    && let Some(el) = document.get_element_by_id(&sheet_dom_id_clone)
                {
                    use wasm_bindgen::JsCast;
                    if let Ok(html_el) = el.dyn_into::<web_sys::HtmlElement>() {
                        let rect = html_el.get_bounding_client_rect();
                        let mut state = drag_state_mount.borrow_mut();
                        state.sheet_height = rect.height();
                        state.sheet_element = Some(html_el);
                    }
                }
            }
        }
    };

    // ── Drag start logic (shared by sheet and handle pointerdown) ──

    // Flag in DragState to track if drag started from the handle
    // (used for handle_only enforcement)

    let make_pointerdown_handler = {
        let drag_state = drag_state.clone();
        let effective_snaps = effective_snaps.clone();
        let mut is_dragging_sig = is_dragging;

        move || {
            let drag_state = drag_state.clone();
            let effective_snaps = effective_snaps.clone();
            move |event: PointerEvent| {
                let mut state = drag_state.borrow_mut();

                // Multi-touch guard
                if state.is_dragging {
                    return;
                }

                // Check scroll lock (wasm-only timestamp)
                #[cfg(target_arch = "wasm32")]
                {
                    let now = crate::helpers::now_ms();
                    if now < state.scroll_locked_until {
                        return;
                    }
                }

                let pointer_id = event.pointer_id();
                state.is_dragging = true;
                state.pointer_id = pointer_id;
                state.start_y = event.client_coordinates().y;
                state.velocity_buffer.clear();

                // Compute base translate from current snap
                let offsets = sheet_math::snap_offsets(&effective_snaps, state.sheet_height);
                let snap_idx = *current_snap_idx.peek();
                state.base_translate_y = offsets.get(snap_idx).copied().unwrap_or(0.0);
                state.current_translate_y = state.base_translate_y;

                // Set pointer capture on the sheet element (wasm only)
                #[cfg(target_arch = "wasm32")]
                if let Some(ref el) = state.sheet_element {
                    let _ = el.set_pointer_capture(pointer_id);
                }

                drop(state);
                is_dragging_sig.set(true);
            }
        }
    };

    // Create handlers: sheet-level and handle-level
    let onpointerdown_sheet = make_pointerdown_handler();
    let onpointerdown_handle = make_pointerdown_handler();

    // Move handler — always on the sheet
    let onpointermove = {
        let drag_state = drag_state.clone();
        move |event: PointerEvent| {
            let mut state = drag_state.borrow_mut();
            if !state.is_dragging {
                return;
            }
            if event.pointer_id() != state.pointer_id {
                return;
            }

            let client_y = event.client_coordinates().y;
            let delta = client_y - state.start_y;
            // Only allow dragging down from base, clamp to prevent dragging above top.
            let new_translate = (state.base_translate_y + delta).max(0.0);
            state.current_translate_y = new_translate;

            // Record velocity sample
            #[cfg(target_arch = "wasm32")]
            {
                let now = crate::helpers::now_ms();
                state.velocity_buffer.push_back((delta, now));
                if state.velocity_buffer.len() > 6 {
                    state.velocity_buffer.pop_front();
                }
            }

            // Imperatively update transform — zero re-renders (wasm only)
            #[cfg(target_arch = "wasm32")]
            if let Some(ref el) = state.sheet_element {
                let _ = el
                    .style()
                    .set_property("transform", &format!("translateY({new_translate}px)"));
                // Disable transition during drag for immediate response
                let _ = el.style().set_property("transition", "none");
            }
        }
    };

    // Up handler — always on the sheet
    let onpointerup = {
        let drag_state = drag_state.clone();
        let effective_snaps = effective_snaps.clone();
        let mut is_dragging_sig = is_dragging;

        move |event: PointerEvent| {
            let mut state = drag_state.borrow_mut();
            if !state.is_dragging {
                return;
            }
            if event.pointer_id() != state.pointer_id {
                return;
            }

            state.is_dragging = false;

            // Release pointer capture (wasm only)
            #[cfg(target_arch = "wasm32")]
            if let Some(ref el) = state.sheet_element {
                let _ = el.release_pointer_capture(state.pointer_id);
            }

            let translate = state.current_translate_y;
            let height = state.sheet_height;
            let offsets = sheet_math::snap_offsets(&effective_snaps, height);

            // Compute velocity from buffer
            let samples: Vec<(f64, f64)> = state.velocity_buffer.iter().copied().collect();
            let velocity = sheet_math::compute_velocity(&samples);

            // Check dismiss threshold
            if dismissible && sheet_math::should_dismiss(translate, height, close_threshold) {
                // Re-enable transition for animate-out (wasm only)
                #[cfg(target_arch = "wasm32")]
                if let Some(ref el) = state.sheet_element {
                    let _ = el.style().set_property("transition", transition_css);
                    let _ = el
                        .style()
                        .set_property("transform", &format!("translateY({height}px)"));
                }
                drop(state);
                is_dragging_sig.set(false);
                let mut o = open;
                o.set(false);
                if let Some(ref handler) = on_open_change {
                    handler.call(false);
                }
                return;
            }

            // Snap to nearest point (with velocity bias)
            let target_idx = sheet_math::snap_with_velocity(translate, velocity, &offsets, 0.5);
            let target_offset = offsets.get(target_idx).copied().unwrap_or(0.0);

            // Animate to snap position (wasm only)
            #[cfg(target_arch = "wasm32")]
            if let Some(ref el) = state.sheet_element {
                let _ = el.style().set_property("transition", transition_css);
                let _ = el
                    .style()
                    .set_property("transform", &format!("translateY({target_offset}px)"));
            }

            state.current_translate_y = target_offset;
            state.base_translate_y = target_offset;
            drop(state);

            is_dragging_sig.set(false);

            // Notify snap point change
            let prev_idx = *current_snap_idx.peek();
            if target_idx != prev_idx {
                let mut snap = current_snap_idx;
                snap.set(target_idx);
                if let Some(ref handler) = on_snap_point_change {
                    handler.call(target_idx);
                }
            }
        }
    };

    // Compute the settled transform style (when not dragging, RSX owns the style)
    let settled_transform = if !is_dragging() {
        let offsets = sheet_math::snap_offsets(&effective_snaps, {
            let state = drag_state.borrow();
            state.sheet_height
        });
        let idx = current_snap_idx();
        let offset = offsets.get(idx).copied().unwrap_or(0.0);
        format!("translateY({offset}px)")
    } else {
        // During drag, web-sys owns the transform imperatively — don't fight it
        String::new()
    };

    // P-015: Track enter animation state for the sheet.
    let sheet_entering = use_signal(|| false);

    // P-015 deferred: Deferred unmount — keep sheet in DOM during close animation.
    let mut should_render = use_signal(|| is_open);
    let dur = animation_duration_ms;

    use_effect(move || {
        let open_now = open();
        if open_now {
            // Opening: render immediately, trigger enter animation
            should_render.set(true);
            if !crate::helpers::prefers_reduced_motion() {
                let mut e = sheet_entering;
                e.set(true);
                spawn(async move {
                    e.set(false);
                });
            }
            if let Some(ref handler) = on_open_animation_end {
                let h = *handler;
                spawn(async move {
                    h.call(());
                });
            }
        } else {
            // Closing: defer unmount if animation_duration_ms > 0
            if let Some(ref handler) = on_close_animation_end {
                let h = *handler;
                spawn(async move {
                    h.call(());
                });
            }
            #[cfg(target_arch = "wasm32")]
            {
                if dur > 0 && !crate::helpers::prefers_reduced_motion() {
                    let mut sr = should_render;
                    spawn(async move {
                        gloo_timers::future::TimeoutFuture::new(dur).await;
                        sr.set(false);
                    });
                } else {
                    should_render.set(false);
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = dur;
                should_render.set(false);
            }
        }
    });

    if !should_render() {
        return rsx! {};
    }

    let sheet_style = if is_dragging() {
        // During drag: position styles only, transform set imperatively
        "position:fixed;bottom:0;left:0;right:0;z-index:9999;touch-action:none;".to_string()
    } else {
        format!(
            "position:fixed;bottom:0;left:0;right:0;z-index:9999;touch-action:none;\
             transform:{settled_transform};\
             transition:{transition_css};"
        )
    };

    if handle_only {
        // handle_only: drag starts from the handle, move/up on the sheet
        rsx! {
            div {
                class: overlay_class.unwrap_or_default(),
                "data-cmdk-sheet-overlay": "true",
                style: "position:fixed;inset:0;z-index:9998;",
                onclick: on_backdrop_click,
            }
            div {
                id: sheet_dom_id,
                class: class.unwrap_or_default(),
                "data-cmdk-sheet": "true",
                // P-015: Present on first open frame for CSS enter animations.
                "data-entering": if sheet_entering() { "true" } else { "" },
                // P-015 deferred: "open" while open prop is true, "closed" during deferred unmount.
                "data-state": if open() { "open" } else { "closed" },
                role: "dialog",
                "aria-modal": "true",
                "aria-label": label.as_deref().unwrap_or("Command palette"),
                tabindex: "-1",
                style: sheet_style,
                onmounted,
                onkeydown: on_sheet_keydown,
                onpointermove,
                onpointerup,

                div {
                    "data-cmdk-sheet-handle": "true",
                    "aria-hidden": "true",
                    style: "display:flex;justify-content:center;padding:8px 0;cursor:grab;touch-action:none;",
                    onpointerdown: onpointerdown_handle,
                    div {
                        style: "width:36px;height:4px;border-radius:2px;background:rgba(128,128,128,0.4);",
                    }
                }

                if show_close_button {
                    button {
                        r#type: "button",
                        "aria-label": "{close_button_label}",
                        "data-cmdk-sheet-close": "",
                        onclick: move |_| { dismiss(); },
                        "\u{D7}"
                    }
                }

                div {
                    tabindex: "0",
                    "aria-hidden": "true",
                    style: "position:fixed;top:0;left:0;width:1px;height:0;padding:0;margin:-1px;overflow:hidden;clip:rect(0,0,0,0);white-space:nowrap;border-width:0;",
                    onfocus: refocus_input,
                }

                div {
                    class: content_class.unwrap_or_default(),
                    "data-cmdk-sheet-content": "true",
                    {children}
                }

                div {
                    tabindex: "0",
                    "aria-hidden": "true",
                    style: "position:fixed;top:0;left:0;width:1px;height:0;padding:0;margin:-1px;overflow:hidden;clip:rect(0,0,0,0);white-space:nowrap;border-width:0;",
                    onfocus: refocus_input,
                }
            }
        }
    } else {
        // Default: drag from anywhere on the sheet
        rsx! {
            div {
                class: overlay_class.unwrap_or_default(),
                "data-cmdk-sheet-overlay": "true",
                style: "position:fixed;inset:0;z-index:9998;",
                onclick: on_backdrop_click,
            }
            div {
                id: sheet_dom_id,
                class: class.unwrap_or_default(),
                "data-cmdk-sheet": "true",
                // P-015: Present on first open frame for CSS enter animations.
                "data-entering": if sheet_entering() { "true" } else { "" },
                // P-015 deferred: "open" while open prop is true, "closed" during deferred unmount.
                "data-state": if open() { "open" } else { "closed" },
                role: "dialog",
                "aria-modal": "true",
                "aria-label": label.as_deref().unwrap_or("Command palette"),
                tabindex: "-1",
                style: sheet_style,
                onmounted,
                onkeydown: on_sheet_keydown,
                onpointerdown: onpointerdown_sheet,
                onpointermove,
                onpointerup,

                div {
                    "data-cmdk-sheet-handle": "true",
                    "aria-hidden": "true",
                    style: "display:flex;justify-content:center;padding:8px 0;cursor:grab;",
                    div {
                        style: "width:36px;height:4px;border-radius:2px;background:rgba(128,128,128,0.4);",
                    }
                }

                if show_close_button {
                    button {
                        r#type: "button",
                        "aria-label": "{close_button_label}",
                        "data-cmdk-sheet-close": "",
                        onclick: move |_| { dismiss(); },
                        "\u{D7}"
                    }
                }

                div {
                    tabindex: "0",
                    "aria-hidden": "true",
                    style: "position:fixed;top:0;left:0;width:1px;height:0;padding:0;margin:-1px;overflow:hidden;clip:rect(0,0,0,0);white-space:nowrap;border-width:0;",
                    onfocus: refocus_input,
                }

                div {
                    class: content_class.unwrap_or_default(),
                    "data-cmdk-sheet-content": "true",
                    {children}
                }

                div {
                    tabindex: "0",
                    "aria-hidden": "true",
                    style: "position:fixed;top:0;left:0;width:1px;height:0;padding:0;margin:-1px;overflow:hidden;clip:rect(0,0,0,0);white-space:nowrap;border-width:0;",
                    onfocus: refocus_input,
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// CommandPalette — adaptive wrapper
// ---------------------------------------------------------------------------

/// Adaptive command palette that auto-detects mobile and renders either
/// `CommandDialog` (desktop) or `CommandSheet` (mobile).
///
/// Accepts the **union** of `CommandDialog` + `CommandSheet` props.
/// Sheet-specific props (`snap_points`, `close_threshold`, etc.) are silently
/// ignored when rendering as a dialog.
///
/// # Mode resolution
///
/// | `mode` | Behavior |
/// |--------|----------|
/// | `Auto` (default) | Uses `use_is_mobile()` media query detection |
/// | `Dialog` | Always renders `CommandDialog` |
/// | `Sheet` | Always renders `CommandSheet` |
///
/// # Example
///
/// ```rust,ignore
/// let palette = use_adaptive_palette();
///
/// rsx! {
///     CommandPalette { open: palette.open,
///         CommandRoot {
///             CommandInput { placeholder: "Search..." }
///             CommandList { CommandEmpty { "No results." } }
///         }
///     }
/// }
/// ```
#[component]
pub fn CommandPalette(
    children: Element,
    /// Controlled open state.
    open: Signal<bool>,
    /// Fired when the palette opens or closes.
    on_open_change: Option<EventHandler<bool>>,
    /// ARIA label for the dialog/sheet.
    #[props(default)]
    label: Option<String>,
    /// CSS class for the container.
    #[props(default)]
    class: Option<String>,
    /// CSS class for the backdrop overlay.
    #[props(default)]
    overlay_class: Option<String>,
    /// CSS class for the content wrapper.
    #[props(default)]
    content_class: Option<String>,
    /// Rendering mode. Defaults to `Auto` (media query detection).
    #[props(default)]
    mode: PaletteMode,
    // ── Sheet-specific props (ignored in dialog mode) ──
    /// Snap point ratios (0.0–1.0). Sheet only.
    #[props(default)]
    snap_points: Option<Vec<f32>>,
    /// Ratio (0.0–1.0) of sheet height that must be dragged to dismiss. Sheet only.
    #[props(default = 0.5)]
    close_threshold: f32,
    /// Milliseconds after scroll before drag re-enables. Sheet only.
    #[props(default = 500)]
    scroll_lock_timeout: u32,
    /// Whether the sheet can be dismissed by gesture/escape/backdrop. Sheet only.
    #[props(default = true)]
    dismissible: bool,
    /// Restrict drag to the handle element only. Sheet only.
    #[props(default = false)]
    handle_only: bool,
    /// Fired when the sheet settles on a snap point. Sheet only.
    on_snap_point_change: Option<EventHandler<usize>>,
    /// Focus the input when the sheet opens. Sheet only.
    #[props(default = false)]
    autofocus_on_open: bool,
) -> Element {
    let is_mobile = use_is_mobile();

    let use_sheet = match mode {
        PaletteMode::Auto => is_mobile(),
        PaletteMode::Dialog => false,
        PaletteMode::Sheet => true,
    };

    if use_sheet {
        rsx! {
            CommandSheet {
                open,
                on_open_change,
                label,
                class,
                overlay_class,
                content_class,
                snap_points,
                close_threshold,
                scroll_lock_timeout,
                dismissible,
                handle_only,
                on_snap_point_change,
                autofocus_on_open,
                {children}
            }
        }
    } else {
        rsx! {
            CommandDialog {
                open,
                on_open_change,
                label,
                class,
                overlay_class,
                content_class,
                {children}
            }
        }
    }
}

// ── P-039: CommandActionPanel ────────────────────────────────────────────────

/// Container for the action panel shown when the user opens actions for an item.
///
/// Renders only when an action panel is active. Provides `role="group"` and
/// appropriate ARIA labelling.
///
/// # Example
/// ```rust,ignore
/// CommandItem {
///     id: "deploy",
///     label: "Deploy",
///     actions: rsx! {
///         CommandActionPanel {
///             CommandAction { id: "deploy-prod", label: "Deploy to Production", on_action: |id| {} }
///             CommandAction { id: "deploy-staging", label: "Deploy to Staging", on_action: |id| {} }
///         }
///     }
/// }
/// ```
#[component]
pub fn CommandActionPanel(
    children: Element,
    /// Accessible label for the action panel group.
    #[props(default = "Actions".to_string())]
    label: String,
) -> Element {
    let ctx = use_context::<CommandContext>();
    let is_open = ctx.read_action_panel().is_some();

    if !is_open {
        return rsx! {};
    }

    rsx! {
        div {
            role: "group",
            "aria-label": label,
            "data-cmdk-action-panel": "",
            {children}
        }
    }
}

// ── P-039: CommandAction ─────────────────────────────────────────────────────

/// A single action item within a [`CommandActionPanel`].
///
/// Self-registers into the command context on mount and unregisters on drop.
/// Renders with `role="option"` and `aria-selected` reflecting the active state.
#[component]
pub fn CommandAction(
    /// Unique identifier for this action.
    #[props(into)]
    id: String,
    /// Display label for this action.
    #[props(into)]
    label: String,
    /// Whether this action is disabled.
    #[props(default = false)]
    disabled: bool,
    /// Callback called with the active item's ID when this action is executed.
    #[props(default)]
    on_action: Option<EventHandler<String>>,
) -> Element {
    let mut ctx = use_context::<CommandContext>();
    let action_id = id.clone();

    // Register on mount — lazily initializes the action panel feature
    use_hook({
        let reg = ActionRegistration {
            id: id.clone(),
            label: label.clone(),
            disabled,
            on_action,
        };
        let ctx_hook = ctx;
        move || {
            let mut af = ctx_hook.ensure_action_panel();
            af.items.write().push(reg);
        }
    });

    // Unregister on drop
    {
        let action_id_drop = action_id.clone();
        use_drop(move || {
            if let Some(mut af) = *ctx.action_panel_feature.peek() {
                af.items.write().retain(|r| r.id != action_id_drop);
            }
        });
    }

    // Sync props changes
    {
        let action_id_sync = action_id.clone();
        let label_sync = label.clone();
        use_effect(move || {
            if let Some(mut af) = *ctx.action_panel_feature.read() {
                let mut items = af.items.write();
                if let Some(reg) = items.iter_mut().find(|r| r.id == action_id_sync) {
                    reg.label.clone_from(&label_sync);
                    reg.disabled = disabled;
                }
            }
        });
    }

    let action_id_render = id.clone();
    let panel_state = ctx.read_action_panel();
    let is_active = panel_state
        .as_ref()
        .map(|s| {
            let feat = *ctx.action_panel_feature.read();
            feat.is_some_and(|af| {
                let items = af.items.read();
                items
                    .get(s.active_idx)
                    .is_some_and(|r| r.id == action_id_render)
            })
        })
        .unwrap_or(false);

    let item_id_for_click = panel_state
        .as_ref()
        .map(|s| s.item_id.clone())
        .unwrap_or_default();

    rsx! {
        div {
            role: "option",
            "aria-selected": if is_active { "true" } else { "false" },
            "aria-disabled": if disabled { "true" } else { "false" },
            "data-cmdk-action": "",
            "data-disabled": if disabled { "true" } else { "" },
            onclick: move |_| {
                if !disabled {
                    if let Some(ref handler) = on_action {
                        handler.call(item_id_for_click.clone());
                    }
                    ctx.close_action_panel();
                }
            },
            {label.clone()}
        }
    }
}

// ── P-040: CommandForm ──────────────────────────────────────────────────────

/// Internal context for [`CommandForm`] — local only, not in [`CommandContext`].
#[derive(Clone, Copy)]
struct FormContext {
    values: Signal<std::collections::HashMap<String, crate::types::FormValue>>,
    active_idx: Signal<usize>,
    field_count: Signal<usize>,
}

/// An inline parameter form for multi-field command input.
///
/// Provides local `FormContext` to child [`CommandFormField`] components.
/// Handles Tab/Shift+Tab field navigation, Enter submission, and Escape cancellation.
///
/// **Note:** The form manages its own keyboard events independently of
/// [`CommandInput`]. Use inside a [`CommandPage`] where the user is expected
/// to fill in parameters.
///
/// # Example
/// ```rust,ignore
/// CommandPage { id: "deploy-params",
///     CommandForm {
///         on_submit: move |values| { /* deploy with values */ },
///         CommandFormField { id: "env", label: "Environment", field_type: FormFieldType::Select {
///             options: vec![
///                 SelectOption { value: "prod".to_string(), label: "Production".to_string() },
///                 SelectOption { value: "stg".to_string(), label: "Staging".to_string() },
///             ]
///         }}
///         CommandFormField { id: "tag", label: "Docker Tag", field_type: FormFieldType::Text }
///     }
/// }
/// ```
#[component]
pub fn CommandForm(
    children: Element,
    /// Called when the user submits the form (presses Enter or Tab past the last field).
    #[props(default)]
    on_submit: Option<EventHandler<std::collections::HashMap<String, crate::types::FormValue>>>,
    /// Called when the user cancels (presses Escape).
    #[props(default)]
    on_cancel: Option<EventHandler<()>>,
    /// Optional CSS class.
    #[props(default)]
    class: Option<String>,
) -> Element {
    let form_ctx = use_context_provider(|| FormContext {
        values: Signal::new(std::collections::HashMap::new()),
        active_idx: Signal::new(0),
        field_count: Signal::new(0),
    });

    let on_submit_clone = on_submit;
    let on_cancel_clone = on_cancel;

    let onkeydown = move |event: KeyboardEvent| {
        match event.key() {
            Key::Tab => {
                event.prevent_default();
                let count = (form_ctx.field_count)();
                if count == 0 {
                    return;
                }
                let mut idx = form_ctx.active_idx;
                if event.modifiers().contains(Modifiers::SHIFT) {
                    let cur = idx();
                    idx.set(if cur == 0 { count - 1 } else { cur - 1 });
                } else {
                    let cur = idx();
                    let next = cur + 1;
                    if next >= count {
                        // Tab past last field → submit
                        if let Some(ref handler) = on_submit_clone {
                            handler.call(form_ctx.values.read().clone());
                        }
                    } else {
                        idx.set(next);
                    }
                }
            }
            Key::Enter => {
                event.prevent_default();
                if let Some(ref handler) = on_submit_clone {
                    handler.call(form_ctx.values.read().clone());
                }
            }
            Key::Escape => {
                event.prevent_default();
                if let Some(ref handler) = on_cancel_clone {
                    handler.call(());
                }
            }
            _ => {}
        }
    };

    rsx! {
        div {
            "data-cmdk-form": "",
            class: class.unwrap_or_default(),
            onkeydown,
            {children}
        }
    }
}

// ── P-040: CommandFormField ─────────────────────────────────────────────────

/// A single field within a [`CommandForm`].
///
/// Self-registers with the parent form context on mount (incrementing
/// `field_count`) and unregisters on drop. Renders the appropriate input
/// element based on `field_type`.
#[component]
pub fn CommandFormField(
    /// Unique field identifier within the form.
    #[props(into)]
    id: String,
    /// Human-readable label for the field.
    #[props(into)]
    label: String,
    /// The type and constraints of this field.
    #[props(default)]
    field_type: crate::types::FormFieldType,
    /// Whether this field must have a non-empty value before submission.
    #[props(default = false)]
    required: bool,
    /// Optional CSS class.
    #[props(default)]
    class: Option<String>,
) -> Element {
    let mut form_ctx = use_context::<FormContext>();
    let field_id = id.clone();

    // Register on mount: assign our index, increment field_count
    let my_idx = use_hook({
        let field_id = field_id.clone();
        let field_type = field_type.clone();
        move || {
            let idx = (form_ctx.field_count)();
            form_ctx.field_count.set(idx + 1);
            // Initialize value
            let default_val = match &field_type {
                crate::types::FormFieldType::Text => crate::types::FormValue::Text(String::new()),
                crate::types::FormFieldType::Number { .. } => crate::types::FormValue::Number(0.0),
                crate::types::FormFieldType::Bool => crate::types::FormValue::Bool(false),
                crate::types::FormFieldType::Select { options } => crate::types::FormValue::Select(
                    options.first().map(|o| o.value.clone()).unwrap_or_default(),
                ),
            };
            form_ctx.values.write().insert(field_id, default_val);
            idx
        }
    });

    // Unregister on drop
    {
        let field_id_drop = id.clone();
        use_drop(move || {
            form_ctx.values.write().remove(&field_id_drop);
            let count = (form_ctx.field_count)();
            if count > 0 {
                form_ctx.field_count.set(count - 1);
            }
        });
    }

    let is_active = (form_ctx.active_idx)() == my_idx;
    let is_invalid = required && {
        let vals = form_ctx.values.read();
        match vals.get(&id) {
            Some(crate::types::FormValue::Text(s)) => s.is_empty(),
            Some(crate::types::FormValue::Select(s)) => s.is_empty(),
            _ => false,
        }
    };

    let field_id_input = id.clone();

    // Render the appropriate input based on field_type
    match field_type {
        crate::types::FormFieldType::Text => {
            rsx! {
                div {
                    "data-cmdk-form-field": "",
                    "data-active": if is_active { "true" } else { "" },
                    "data-invalid": if is_invalid { "true" } else { "" },
                    class: class.unwrap_or_default(),
                    label { r#for: id.clone(), {label.clone()} }
                    input {
                        id: id.clone(),
                        r#type: "text",
                        "aria-required": if required { "true" } else { "false" },
                        "aria-invalid": if is_invalid { "true" } else { "false" },
                        oninput: move |evt: FormEvent| {
                            form_ctx.values.write().insert(
                                field_id_input.clone(),
                                crate::types::FormValue::Text(evt.value()),
                            );
                        },
                        onfocus: move |_| { form_ctx.active_idx.set(my_idx); },
                    }
                }
            }
        }
        crate::types::FormFieldType::Number { min, max } => {
            rsx! {
                div {
                    "data-cmdk-form-field": "",
                    "data-active": if is_active { "true" } else { "" },
                    "data-invalid": if is_invalid { "true" } else { "" },
                    class: class.unwrap_or_default(),
                    label { r#for: id.clone(), {label.clone()} }
                    input {
                        id: id.clone(),
                        r#type: "number",
                        min: min.map(|m| m.to_string()).unwrap_or_default(),
                        max: max.map(|m| m.to_string()).unwrap_or_default(),
                        "aria-required": if required { "true" } else { "false" },
                        oninput: move |evt: FormEvent| {
                            if let Ok(n) = evt.value().parse::<f64>() {
                                form_ctx.values.write().insert(
                                    field_id_input.clone(),
                                    crate::types::FormValue::Number(n),
                                );
                            }
                        },
                        onfocus: move |_| { form_ctx.active_idx.set(my_idx); },
                    }
                }
            }
        }
        crate::types::FormFieldType::Bool => {
            rsx! {
                div {
                    "data-cmdk-form-field": "",
                    "data-active": if is_active { "true" } else { "" },
                    "data-invalid": if is_invalid { "true" } else { "" },
                    class: class.unwrap_or_default(),
                    label { r#for: id.clone(), {label.clone()} }
                    input {
                        id: id.clone(),
                        r#type: "checkbox",
                        "aria-required": if required { "true" } else { "false" },
                        oninput: move |evt: FormEvent| {
                            let checked = evt.value() == "true" || evt.value() == "on";
                            form_ctx.values.write().insert(
                                field_id_input.clone(),
                                crate::types::FormValue::Bool(checked),
                            );
                        },
                        onfocus: move |_| { form_ctx.active_idx.set(my_idx); },
                    }
                }
            }
        }
        crate::types::FormFieldType::Select { ref options } => {
            let options = options.clone();
            rsx! {
                div {
                    "data-cmdk-form-field": "",
                    "data-active": if is_active { "true" } else { "" },
                    "data-invalid": if is_invalid { "true" } else { "" },
                    class: class.unwrap_or_default(),
                    label { r#for: id.clone(), {label.clone()} }
                    select {
                        id: id.clone(),
                        "aria-required": if required { "true" } else { "false" },
                        onchange: move |evt: FormEvent| {
                            form_ctx.values.write().insert(
                                field_id_input.clone(),
                                crate::types::FormValue::Select(evt.value()),
                            );
                        },
                        onfocus: move |_| { form_ctx.active_idx.set(my_idx); },
                        for opt in options {
                            option {
                                value: opt.value.clone(),
                                {opt.label.clone()}
                            }
                        }
                    }
                }
            }
        }
    }
}

/// A non-interactive informational callout. Can be placed in the input area
/// (as a sibling to `CommandInput`) or inside `CommandList` (as a decorative row
/// before/after groups). Does **not** register as a `CommandItem` — keyboard
/// navigation never lands on it; `filtered_count` is unaffected.
///
/// Consumer controls visibility via signals. `dismissible=true` renders an
/// unstyled dismiss button (`[data-cmdk-callout-dismiss]`) after children;
/// `on_dismiss` fires on click. Consumer owns visible state.
///
/// # Example
/// ```rust,ignore
/// if show_tip() {
///     CommandCallout {
///         dismissible: true,
///         on_dismiss: move |_| show_tip.set(false),
///         "Try typing '>' for editor commands"
///     }
/// }
/// ```
#[component]
pub fn CommandCallout(
    /// Optional CSS class forwarded to the root element.
    class: Option<String>,
    /// Renders an unstyled dismiss button (`[data-cmdk-callout-dismiss]`) after children.
    /// When clicked, fires `on_dismiss` (if set). Consumer owns visible state.
    #[props(default)]
    dismissible: bool,
    /// Called when the auto-rendered dismiss button is clicked (requires `dismissible=true`).
    on_dismiss: Option<EventHandler<()>>,
    /// Any content: icon, text, links, custom dismiss button.
    children: Element,
) -> Element {
    rsx! {
        div {
            "data-cmdk-callout": "",
            role: "note",
            class: class.unwrap_or_default(),
            {children}
            if dismissible {
                button {
                    "data-cmdk-callout-dismiss": "",
                    "aria-label": "Dismiss",
                    r#type: "button",
                    onclick: move |_| {
                        if let Some(cb) = &on_dismiss {
                            cb.call(());
                        }
                    },
                }
            }
        }
    }
}
