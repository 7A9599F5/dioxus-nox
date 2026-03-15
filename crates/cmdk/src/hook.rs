use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::{HashSet, VecDeque};
use std::ops::Deref;
use std::rc::Rc;

use dioxus::prelude::*;
use dioxus_core::use_drop;

use crate::context::{CommandContext, use_command_context};
use crate::shortcut::Hotkey;
use crate::types::{
    AsyncCommandHandle, AsyncItem, ChordShortcut, ChordState, GlobalShortcut, ModeRegistration,
    ScoredItem,
};

// ---------------------------------------------------------------------------
// P-035: Type aliases for closure-holder types to avoid clippy::type_complexity.
// ---------------------------------------------------------------------------

/// Holder type for a stored `KeyboardEvent` closure (wasm32 only).
/// Used to keep a closure alive and allow removal via `removeEventListener`.
#[cfg(target_arch = "wasm32")]
type KeydownClosureHolder =
    Rc<RefCell<Option<wasm_bindgen::prelude::Closure<dyn FnMut(web_sys::KeyboardEvent)>>>>;

/// Holder type for a stored `MediaQueryListEvent` closure (wasm32 only).
#[cfg(target_arch = "wasm32")]
type MqClosureHolder =
    Rc<RefCell<Option<wasm_bindgen::prelude::Closure<dyn FnMut(web_sys::MediaQueryListEvent)>>>>;

/// Holder type for a `MediaQueryList` reference (wasm32 only).
#[cfg(target_arch = "wasm32")]
type MqHolder = Rc<RefCell<Option<web_sys::MediaQueryList>>>;

// ---------------------------------------------------------------------------
// P-034: Cross-platform global keydown hook helper
// ---------------------------------------------------------------------------

/// Set up a global `keydown` listener on `window` with proper cleanup on unmount.
///
/// On **wasm32**: registers `addEventListener("keydown", …)` on `window`,
/// stores the closure in a `use_hook`-allocated holder, and removes the
/// listener via `use_drop` when the calling component unmounts.
///
/// On **non-wasm** targets: this is a no-op. Use Dioxus native event handling
/// (e.g. `onkeydown` on the root element) for keyboard shortcuts on desktop
/// and mobile.
///
/// # Example
///
/// ```rust,ignore
/// use_global_keydown(move |event: web_sys::KeyboardEvent| {
///     if event.key() == "Escape" { /* … */ }
/// });
/// ```
#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
pub(crate) fn use_global_keydown<F>(handler: F)
where
    F: Fn(web_sys::KeyboardEvent) + 'static,
{
    use wasm_bindgen::JsCast;
    use wasm_bindgen::prelude::*;

    let holder: KeydownClosureHolder = use_hook(|| Rc::new(RefCell::new(None)));
    // Wrap the handler in Rc so it can be shared across the use_effect FnMut boundary.
    let handler = Rc::new(handler);

    {
        let ch = holder.clone();
        use_effect(move || {
            let h = handler.clone();
            let closure = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| h(event))
                as Box<dyn FnMut(web_sys::KeyboardEvent)>);
            if let Some(window) = web_sys::window() {
                let _ = window
                    .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref());
            }
            *ch.borrow_mut() = Some(closure);
        });
    }

    {
        let ch = holder.clone();
        use_drop(move || {
            if let Some(cl) = ch.borrow_mut().take()
                && let Some(window) = web_sys::window()
            {
                let _ = window
                    .remove_event_listener_with_callback("keydown", cl.as_ref().unchecked_ref());
            }
        });
    }
}

/// No-op stub for non-wasm targets.
///
/// On desktop and mobile, use Dioxus native keyboard event handling rather
/// than global DOM listeners.
#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub(crate) fn use_global_keydown<F>(_handler: F)
where
    F: Fn(()) + 'static,
{
}

/// Handle returned by `use_command_palette`.
#[derive(Clone, Copy)]
pub struct CommandPaletteHandle {
    pub open: Signal<bool>,
}

impl CommandPaletteHandle {
    pub fn toggle(&self) {
        let mut open = self.open;
        let current = *open.read();
        open.set(!current);
    }

    pub fn show(&self) {
        let mut open = self.open;
        open.set(true);
    }

    pub fn hide(&self) {
        let mut open = self.open;
        open.set(false);
    }
}

/// Hook that provides a command palette handle with open/close state
/// and optionally sets up a global Cmd/Ctrl+K keyboard shortcut.
///
/// **Note:** The keyboard shortcut is only functional on wasm targets.
/// On desktop, Dioxus `document::eval` cannot toggle Rust signals from JS,
/// so the shortcut is a no-op. Use your own key-event handler for desktop.
pub fn use_command_palette(setup_shortcut: bool) -> CommandPaletteHandle {
    let open = use_signal(|| false);

    // P-035: Store the closure so it can be properly removed on drop.
    // The holder is allocated once via use_hook (stable across re-renders).
    #[cfg(target_arch = "wasm32")]
    let closure_holder: KeydownClosureHolder = use_hook(|| Rc::new(RefCell::new(None)));

    // Hooks must always be called unconditionally - check flag inside the effect
    #[cfg(target_arch = "wasm32")]
    {
        let ch = closure_holder.clone();
        use_effect(move || {
            if setup_shortcut {
                use wasm_bindgen::JsCast;
                use wasm_bindgen::prelude::*;

                let closure = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
                    if (event.meta_key() || event.ctrl_key()) && event.key() == "k" {
                        event.prevent_default();
                        let mut open = open;
                        let current = *open.peek();
                        open.set(!current);
                    }
                })
                    as Box<dyn FnMut(web_sys::KeyboardEvent)>);

                if let Some(window) = web_sys::window() {
                    let _ = window.add_event_listener_with_callback(
                        "keydown",
                        closure.as_ref().unchecked_ref(),
                    );
                }
                // Store closure instead of forgetting — cleaned up in use_drop below.
                *ch.borrow_mut() = Some(closure);
            }
        });
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use_effect(move || {
            let _ = setup_shortcut;
            // Desktop: keyboard shortcut cannot toggle Rust signals from JS eval.
            // Users should handle Cmd/Ctrl+K in their own key-event handler and
            // call `handle.toggle()` directly.
        });
    }

    // P-035: Remove the keydown listener when the component unmounts.
    #[cfg(target_arch = "wasm32")]
    {
        let ch = closure_holder.clone();
        use_drop(move || {
            if let Some(cl) = ch.borrow_mut().take() {
                use wasm_bindgen::JsCast;
                if let Some(window) = web_sys::window() {
                    let _ = window.remove_event_listener_with_callback(
                        "keydown",
                        cl.as_ref().unchecked_ref(),
                    );
                }
            }
        });
    }

    CommandPaletteHandle { open }
}

/// Retrieve the [`CommandPaletteHandle`] from context, if one was provided
/// by an ancestor [`CommandDialog`](crate::CommandDialog) or
/// [`CommandSheet`](crate::CommandSheet).
///
/// Returns `None` when called outside a dialog/sheet tree.
///
/// ```rust,ignore
/// // Inside a deeply nested component:
/// if let Some(palette) = use_command_palette_handle() {
///     palette.hide();
/// }
/// ```
pub fn use_command_palette_handle() -> Option<CommandPaletteHandle> {
    try_use_context::<CommandPaletteHandle>()
}

/// Creates open/close state for a [`CommandSheet`](crate::CommandSheet).
///
/// This is a convenience alias — it returns the same [`CommandPaletteHandle`]
/// as [`use_command_palette`], but without setting up the Cmd/Ctrl+K shortcut
/// (which isn't appropriate for a mobile sheet).
///
/// ```rust,ignore
/// let sheet = use_command_sheet();
/// rsx! {
///     button { onclick: move |_| sheet.show(), "Open Sheet" }
///     CommandSheet { open: sheet.open,
///         CommandRoot { /* ... */ }
///     }
/// }
/// ```
pub fn use_command_sheet() -> CommandPaletteHandle {
    use_command_palette(false)
}

/// Handle for page navigation within a command palette.
///
/// Obtained via [`use_command_pages`]. Must be called inside a `CommandRoot` tree.
///
/// ```rust,ignore
/// let pages = use_command_pages();
///
/// // In an on_select handler:
/// pages.push("exercises");
///
/// // Later:
/// pages.pop();
/// ```
#[derive(Clone, Copy)]
pub struct CommandPagesHandle {
    ctx: CommandContext,
}

impl CommandPagesHandle {
    /// Push a page onto the navigation stack. Clears search.
    pub fn push(&self, page_id: &str) {
        self.ctx.push_page(page_id);
    }

    /// Push a page with associated data that can be read by the page's components.
    pub fn push_with_data(&self, page_id: &str, data: Rc<dyn Any>) {
        self.ctx.push_page_with_data(page_id, data);
    }

    /// Pop the top page from the stack. Clears search.
    /// Returns the popped page ID, or None if already at root.
    pub fn pop(&self) -> Option<String> {
        self.ctx.pop_page()
    }

    /// Clear the entire page stack (return to root). Clears search.
    pub fn clear(&self) {
        self.ctx.clear_pages();
    }

    /// Current page navigation stack.
    pub fn stack(&self) -> Vec<String> {
        self.ctx.page_stack.read().clone()
    }

    /// Breadcrumbs: `(id, title)` pairs for each page in the stack.
    pub fn breadcrumbs(&self) -> Vec<(String, Option<String>)> {
        let stack = self.ctx.page_stack.read();
        let pages = self.ctx.pages.read();
        stack
            .iter()
            .map(|sid| {
                let title = pages
                    .iter()
                    .find(|p| p.id == *sid)
                    .and_then(|p| p.title.clone());
                (sid.clone(), title)
            })
            .collect()
    }

    /// The currently active page ID, or None if at root.
    pub fn current(&self) -> Option<String> {
        self.ctx.active_page.read().clone()
    }

    /// Whether the page stack is empty (at root).
    pub fn is_root(&self) -> bool {
        self.ctx.page_stack.read().is_empty()
    }

    /// Get the current page's data, downcast to the expected type.
    pub fn data<T: 'static>(&self) -> Option<Rc<T>> {
        self.ctx.get_page_data::<T>()
    }
}

/// Hook that provides page navigation within a command palette.
///
/// Must be called inside a `CommandRoot` component tree (i.e. where
/// `CommandContext` is available via context).
///
/// ```rust,ignore
/// let pages = use_command_pages();
///
/// rsx! {
///     CommandItem {
///         id: "jump-to-exercise",
///         label: "Jump to Exercise",
///         on_select: move |_| { pages.push("exercises"); },
///         "Jump to Exercise"
///     }
/// }
/// ```
pub fn use_command_pages() -> CommandPagesHandle {
    let ctx: CommandContext = use_context();
    CommandPagesHandle { ctx }
}

// ---------------------------------------------------------------------------
// Adaptive palette — mobile detection + unified handle
// ---------------------------------------------------------------------------

/// Reactive hook that detects mobile browsers via media queries.
///
/// On wasm32, creates `MediaQueryList` objects for `(pointer: coarse) and (hover: none)`
/// and `(max-width: 768px)`. Both must match for the signal to be `true`.
/// Listens for `"change"` events and updates the signal reactively.
///
/// On non-wasm targets, always returns `false`.
pub fn use_is_mobile() -> Signal<bool> {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        use wasm_bindgen::prelude::*;

        let is_mobile = use_signal(|| {
            // Read initial state from media queries
            if let Some(window) = web_sys::window() {
                let pointer_mq = window
                    .match_media("(pointer: coarse) and (hover: none)")
                    .ok()
                    .flatten();
                let width_mq = window.match_media("(max-width: 768px)").ok().flatten();
                let pointer_matches = pointer_mq.as_ref().is_some_and(|mq| mq.matches());
                let width_matches = width_mq.as_ref().is_some_and(|mq| mq.matches());
                pointer_matches && width_matches
            } else {
                false
            }
        });

        // P-035: Holders for MQ listener closures so they can be removed on drop.
        let pointer_closure_holder: MqClosureHolder = use_hook(|| Rc::new(RefCell::new(None)));
        let width_closure_holder: MqClosureHolder = use_hook(|| Rc::new(RefCell::new(None)));
        // Store MediaQueryList references so the drop handler can call removeEventListener.
        let pointer_mq_holder: MqHolder = use_hook(|| Rc::new(RefCell::new(None)));
        let width_mq_holder: MqHolder = use_hook(|| Rc::new(RefCell::new(None)));

        // Set up reactive listeners (once)
        {
            let pch = pointer_closure_holder.clone();
            let wch = width_closure_holder.clone();
            let pmh = pointer_mq_holder.clone();
            let wmh = width_mq_holder.clone();
            use_effect(move || {
                let Some(window) = web_sys::window() else {
                    return;
                };

                let pointer_mq = window
                    .match_media("(pointer: coarse) and (hover: none)")
                    .ok()
                    .flatten();
                let width_mq = window.match_media("(max-width: 768px)").ok().flatten();

                // Shared evaluation closure
                let evaluate = {
                    let pointer_mq = pointer_mq.clone();
                    let width_mq = width_mq.clone();
                    move || {
                        let pointer = pointer_mq.as_ref().is_some_and(|mq| mq.matches());
                        let width = width_mq.as_ref().is_some_and(|mq| mq.matches());
                        pointer && width
                    }
                };

                // Listener for pointer media query
                if let Some(ref mq) = pointer_mq {
                    let evaluate = evaluate.clone();
                    let mut sig = is_mobile;
                    let closure = Closure::wrap(Box::new(move |_: web_sys::MediaQueryListEvent| {
                        sig.set(evaluate());
                    })
                        as Box<dyn FnMut(web_sys::MediaQueryListEvent)>);
                    let _ = mq.add_event_listener_with_callback(
                        "change",
                        closure.as_ref().unchecked_ref(),
                    );
                    // Store closure and MQ ref for cleanup in use_drop.
                    *pch.borrow_mut() = Some(closure);
                    *pmh.borrow_mut() = Some(mq.clone());
                }

                // Listener for width media query
                if let Some(ref mq) = width_mq {
                    let evaluate = evaluate.clone();
                    let mut sig = is_mobile;
                    let closure = Closure::wrap(Box::new(move |_: web_sys::MediaQueryListEvent| {
                        sig.set(evaluate());
                    })
                        as Box<dyn FnMut(web_sys::MediaQueryListEvent)>);
                    let _ = mq.add_event_listener_with_callback(
                        "change",
                        closure.as_ref().unchecked_ref(),
                    );
                    // Store closure and MQ ref for cleanup in use_drop.
                    *wch.borrow_mut() = Some(closure);
                    *wmh.borrow_mut() = Some(mq.clone());
                }
            });
        }

        // P-035: Remove MQ listeners when the component unmounts.
        {
            let pch = pointer_closure_holder.clone();
            let wch = width_closure_holder.clone();
            let pmh = pointer_mq_holder.clone();
            let wmh = width_mq_holder.clone();
            use_drop(move || {
                use wasm_bindgen::JsCast;
                if let (Some(cl), Some(mq)) = (pch.borrow_mut().take(), pmh.borrow_mut().take()) {
                    let _ = mq
                        .remove_event_listener_with_callback("change", cl.as_ref().unchecked_ref());
                }
                if let (Some(cl), Some(mq)) = (wch.borrow_mut().take(), wmh.borrow_mut().take()) {
                    let _ = mq
                        .remove_event_listener_with_callback("change", cl.as_ref().unchecked_ref());
                }
            });
        }

        is_mobile
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        use_signal(|| false)
    }
}

/// Unified handle for the adaptive command palette.
///
/// Wraps [`CommandPaletteHandle`] and exposes an `is_mobile` signal for
/// conditional rendering based on detected mode.
///
/// Dereferences to `CommandPaletteHandle`, so `.toggle()`, `.show()`, `.hide()`,
/// and `.open` are directly available.
#[derive(Clone, Copy)]
pub struct AdaptivePaletteHandle {
    inner: CommandPaletteHandle,
    /// Reactive signal indicating whether the current device is mobile.
    pub is_mobile: Signal<bool>,
}

impl Deref for AdaptivePaletteHandle {
    type Target = CommandPaletteHandle;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// Unified hook that detects mobile, creates open state, and sets up Cmd/Ctrl+K
/// (desktop only).
///
/// Combines `use_is_mobile()` + `use_command_palette(true)` into a single call.
/// The keyboard shortcut is only registered when the device is not detected as mobile.
///
/// # Example
///
/// ```rust,ignore
/// let palette = use_adaptive_palette();
///
/// rsx! {
///     if !(palette.is_mobile)() {
///         kbd { "Cmd+K" }
///     }
///     CommandPalette { open: palette.open,
///         CommandRoot { CommandInput {} CommandList { /* items */ } }
///     }
/// }
/// ```
pub fn use_adaptive_palette() -> AdaptivePaletteHandle {
    let is_mobile = use_is_mobile();
    let open = use_signal(|| false);

    // P-035: Store the Cmd+K closure for proper cleanup on drop (wasm only).
    #[cfg(target_arch = "wasm32")]
    let adaptive_closure_holder: KeydownClosureHolder = use_hook(|| Rc::new(RefCell::new(None)));

    // Set up Cmd/Ctrl+K shortcut (wasm only, desktop only)
    #[cfg(target_arch = "wasm32")]
    {
        let ch = adaptive_closure_holder.clone();
        use_effect(move || {
            // Only register shortcut if not mobile
            if is_mobile() {
                return;
            }

            use wasm_bindgen::JsCast;
            use wasm_bindgen::prelude::*;

            let closure = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
                if (event.meta_key() || event.ctrl_key()) && event.key() == "k" {
                    event.prevent_default();
                    let mut open = open;
                    let current = *open.peek();
                    open.set(!current);
                }
            }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);

            if let Some(window) = web_sys::window() {
                let _ = window
                    .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref());
            }
            // Store instead of forgetting — cleaned up in use_drop below.
            *ch.borrow_mut() = Some(closure);
        });
    }
    #[cfg(not(target_arch = "wasm32"))]
    use_effect(move || {
        // Desktop: no global shortcut setup (no-op).
        let _ = is_mobile;
        let _ = open;
    });

    // P-035: Remove keydown listener when the component unmounts (wasm only).
    #[cfg(target_arch = "wasm32")]
    {
        let ch = adaptive_closure_holder.clone();
        use_drop(move || {
            if let Some(cl) = ch.borrow_mut().take() {
                use wasm_bindgen::JsCast;
                if let Some(window) = web_sys::window() {
                    let _ = window.remove_event_listener_with_callback(
                        "keydown",
                        cl.as_ref().unchecked_ref(),
                    );
                }
            }
        });
    }

    AdaptivePaletteHandle {
        inner: CommandPaletteHandle { open },
        is_mobile,
    }
}

// ---------------------------------------------------------------------------
// Mode management hook
// ---------------------------------------------------------------------------

/// Handle for managing command palette modes.
#[derive(Clone, Copy)]
pub struct CommandModesHandle {
    ctx: CommandContext,
}

impl CommandModesHandle {
    /// Register a mode.
    pub fn register(&self, reg: ModeRegistration) {
        self.ctx.register_mode(reg);
    }

    /// Unregister a mode by ID.
    pub fn unregister(&self, id: &str) {
        self.ctx.unregister_mode(id);
    }

    /// The currently active mode (reactive).
    pub fn active(&self) -> Option<ModeRegistration> {
        self.ctx.active_mode.read().clone()
    }

    /// The query with mode prefix stripped (reactive).
    pub fn query(&self) -> String {
        self.ctx.mode_query.read().clone()
    }
}

/// Hook for managing command palette modes.
/// Must be called inside a `CommandRoot` tree.
pub fn use_command_modes() -> CommandModesHandle {
    let ctx: CommandContext = use_context();
    CommandModesHandle { ctx }
}

// ---------------------------------------------------------------------------
// Global shortcuts hook
// ---------------------------------------------------------------------------

/// Handle for global keyboard shortcuts outside the command palette.
#[derive(Clone)]
pub struct GlobalShortcutHandle {
    shortcuts: Signal<Vec<GlobalShortcut>>,
    chords: Signal<Vec<ChordShortcut>>,
    chord_state: Signal<ChordState>,
    suspended: Signal<bool>,
}

impl GlobalShortcutHandle {
    /// Register a single-key global shortcut.
    pub fn register(&self, id: &str, hotkey: Hotkey, handler: EventHandler<()>) {
        let mut shortcuts = self.shortcuts;
        shortcuts.write().push(GlobalShortcut {
            id: id.to_string(),
            hotkey,
            handler,
        });
    }

    /// Register a two-key chord shortcut.
    pub fn register_chord(
        &self,
        id: &str,
        first: Hotkey,
        second: Hotkey,
        handler: EventHandler<()>,
        timeout_ms: u32,
    ) {
        let mut chords = self.chords;
        chords.write().push(ChordShortcut {
            id: id.to_string(),
            first,
            second,
            handler,
            timeout_ms,
        });
    }

    /// Unregister a shortcut or chord by ID.
    pub fn unregister(&self, id: &str) {
        let mut shortcuts = self.shortcuts;
        shortcuts.write().retain(|s| s.id != id);
        let mut chords = self.chords;
        chords.write().retain(|c| c.id != id);
    }

    /// Suspend all global shortcuts (e.g., when palette is open).
    pub fn suspend(&self) {
        let mut suspended = self.suspended;
        suspended.set(true);
    }

    /// Resume global shortcuts.
    pub fn resume(&self) {
        let mut suspended = self.suspended;
        suspended.set(false);
    }

    /// Whether a chord is pending (for UI indicator).
    pub fn pending_chord(&self) -> Option<Hotkey> {
        self.chord_state
            .read()
            .pending
            .as_ref()
            .map(|(hk, _)| hk.clone())
    }
}

/// Hook that sets up document-level keyboard shortcut handling.
///
/// Installs a single `keydown` listener on `document`. Auto-suspends when
/// the active element is an input, textarea, or select (unless the key is
/// Escape or has a modifier).
///
/// On non-wasm targets, this is a no-op — use Dioxus's native keyboard
/// event handling instead.
pub fn use_global_shortcuts() -> GlobalShortcutHandle {
    let shortcuts: Signal<Vec<GlobalShortcut>> = use_signal(Vec::new);
    let chords: Signal<Vec<ChordShortcut>> = use_signal(Vec::new);
    let chord_state: Signal<ChordState> = use_signal(ChordState::default);
    let suspended: Signal<bool> = use_signal(|| false);

    // P-035: Store the document keydown closure for proper cleanup on drop (wasm only).
    #[cfg(target_arch = "wasm32")]
    let global_closure_holder: KeydownClosureHolder = use_hook(|| Rc::new(RefCell::new(None)));

    // Install document keydown listener (wasm only)
    #[cfg(target_arch = "wasm32")]
    {
        let ch = global_closure_holder.clone();
        use_effect(move || {
            use keyboard_types::{Key, Modifiers};
            use wasm_bindgen::JsCast;
            use wasm_bindgen::prelude::*;

            let closure = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
                if *suspended.peek() {
                    return;
                }

                // Auto-suspend for text inputs
                if let Some(target) = event.target()
                    && let Ok(element) = target.dyn_into::<web_sys::Element>()
                {
                    let tag = element.tag_name().to_lowercase();
                    if matches!(tag.as_str(), "input" | "textarea" | "select") {
                        // Allow Escape and modifier combos through
                        if event.key() != "Escape"
                            && !event.ctrl_key()
                            && !event.meta_key()
                            && !event.alt_key()
                        {
                            return;
                        }
                    }
                }

                // Parse key and modifiers from web event
                let key_str = event.key();
                let key = match key_str.as_str() {
                    "Enter" => Key::Enter,
                    "Escape" => Key::Escape,
                    "Tab" => Key::Tab,
                    "Backspace" => Key::Backspace,
                    "Delete" => Key::Delete,
                    "ArrowUp" => Key::ArrowUp,
                    "ArrowDown" => Key::ArrowDown,
                    "ArrowLeft" => Key::ArrowLeft,
                    "ArrowRight" => Key::ArrowRight,
                    "Home" => Key::Home,
                    "End" => Key::End,
                    "PageUp" => Key::PageUp,
                    "PageDown" => Key::PageDown,
                    other => {
                        let chars: Vec<char> = other.chars().collect();
                        if chars.len() == 1 {
                            Key::Character(chars[0].to_lowercase().to_string())
                        } else {
                            return; // Unrecognized key
                        }
                    }
                };

                let mut modifiers = Modifiers::empty();
                if event.ctrl_key() {
                    modifiers |= Modifiers::CONTROL;
                }
                if event.shift_key() {
                    modifiers |= Modifiers::SHIFT;
                }
                if event.alt_key() {
                    modifiers |= Modifiers::ALT;
                }
                if event.meta_key() {
                    modifiers |= Modifiers::META;
                }

                // Check chord state
                let mut cs = chord_state;
                let now = crate::helpers::now_ms();
                let current_chord = cs.peek().pending.clone();

                if let Some((first_key, timestamp)) = current_chord {
                    // Check if any chord matches
                    let chord_list = chords.peek();
                    let timeout_expired = chord_list
                        .iter()
                        .all(|c| now - timestamp > c.timeout_ms as f64);

                    if !timeout_expired {
                        let matched = chord_list.iter().find(|c| {
                            c.first.matches(&first_key.key, first_key.modifiers)
                                && c.second.matches(&key, modifiers)
                        });

                        if let Some(chord) = matched {
                            event.prevent_default();
                            cs.set(ChordState { pending: None });
                            chord.handler.call(());
                            return;
                        }
                    }

                    // Clear pending — either timed out or no match
                    cs.set(ChordState { pending: None });
                }

                // Check if this key starts a chord
                let chord_list = chords.peek();
                let starts_chord = chord_list.iter().any(|c| c.first.matches(&key, modifiers));
                if starts_chord {
                    cs.set(ChordState {
                        pending: Some((Hotkey::new(modifiers, key.clone()), now)),
                    });
                    return;
                }

                // Check single-key shortcuts
                let shortcut_list = shortcuts.peek();
                if let Some(shortcut) = shortcut_list
                    .iter()
                    .find(|s| s.hotkey.matches(&key, modifiers))
                {
                    event.prevent_default();
                    shortcut.handler.call(());
                }
            }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);

            if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                let _ = doc
                    .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref());
            }
            // Store instead of forgetting — cleaned up in use_drop below.
            *ch.borrow_mut() = Some(closure);
        });
    }
    #[cfg(not(target_arch = "wasm32"))]
    use_effect(move || {
        // Non-wasm: no global shortcuts setup (use Dioxus native event handling).
        let _ = shortcuts;
        let _ = chords;
        let _ = chord_state;
        let _ = suspended;
    });

    // P-035: Remove the document keydown listener when the component unmounts (wasm only).
    #[cfg(target_arch = "wasm32")]
    {
        let ch = global_closure_holder.clone();
        use_drop(move || {
            if let Some(cl) = ch.borrow_mut().take() {
                use wasm_bindgen::JsCast;
                if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                    let _ = doc.remove_event_listener_with_callback(
                        "keydown",
                        cl.as_ref().unchecked_ref(),
                    );
                }
            }
        });
    }

    GlobalShortcutHandle {
        shortcuts,
        chords,
        chord_state,
        suspended,
    }
}

// ---------------------------------------------------------------------------
// Command history hook
// ---------------------------------------------------------------------------

/// Core history data structure — pure Rust, no signals. Testable standalone.
#[derive(Debug, Clone)]
pub(crate) struct CommandHistoryState {
    pub entries: VecDeque<String>,
    pub capacity: usize,
    /// `None` = not navigating; `Some(0)` = most recent entry.
    pub cursor: Option<usize>,
    pub draft: Option<String>,
}

impl CommandHistoryState {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(capacity),
            capacity,
            cursor: None,
            draft: None,
        }
    }

    /// Push a value, deduplicating (moves existing to end). Evicts oldest when over capacity.
    pub fn push(&mut self, value: &str) {
        self.entries.retain(|e| e != value);
        self.entries.push_back(value.to_string());
        if self.entries.len() > self.capacity {
            self.entries.pop_front();
        }
        self.cursor = None;
    }

    /// Navigate to an older entry. Returns the entry text, or `None` if already at oldest.
    pub fn prev(&mut self) -> Option<String> {
        let len = self.entries.len();
        if len == 0 {
            return None;
        }
        let new_cursor = match self.cursor {
            None => 0,
            Some(c) if c + 1 < len => c + 1,
            Some(_) => return None, // Already at oldest
        };
        self.cursor = Some(new_cursor);
        self.entries.get(len - 1 - new_cursor).cloned()
    }

    /// Navigate to a newer entry. Returns `None` when past the newest (back to draft).
    pub fn next(&mut self) -> Option<String> {
        let len = self.entries.len();
        match self.cursor {
            None => None,
            Some(0) => {
                self.cursor = None;
                None
            }
            Some(c) => {
                let new_cursor = c - 1;
                self.cursor = Some(new_cursor);
                self.entries.get(len - 1 - new_cursor).cloned()
            }
        }
    }

    pub fn reset_navigation(&mut self) {
        self.cursor = None;
    }

    pub fn save_draft(&mut self, query: &str) {
        self.draft = Some(query.to_string());
    }

    pub fn take_draft(&mut self) -> Option<String> {
        self.draft.take()
    }

    pub fn entries_vec(&self) -> Vec<String> {
        self.entries.iter().cloned().collect()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.cursor = None;
    }
}

/// Context provider so `CommandInput` can detect and integrate with history.
#[derive(Clone, Copy)]
pub(crate) struct CommandHistoryContext {
    pub handle: CommandHistoryHandle,
}

/// Handle for the command history feature. Obtained via [`use_command_history`].
///
/// Wraps a reactive `Signal<CommandHistoryState>` for Dioxus integration.
/// Use `Alt+ArrowUp` / `Alt+ArrowDown` in [`CommandInput`](crate::CommandInput)
/// to navigate history (requires `use_command_history` in the same tree).
#[derive(Clone, Copy)]
pub struct CommandHistoryHandle {
    state: Signal<CommandHistoryState>,
}

impl CommandHistoryHandle {
    /// Push a value into history. Moves duplicates to end, evicts oldest when full.
    pub fn push(&self, value: &str) {
        let mut state = self.state;
        state.write().push(value);
    }

    /// Navigate to an older entry. Returns the entry text, or `None` if at the oldest.
    pub fn prev(&self) -> Option<String> {
        let mut state = self.state;
        state.write().prev()
    }

    /// Navigate to a newer entry. Returns `None` when past newest (back to draft).
    pub fn next(&self) -> Option<String> {
        let mut state = self.state;
        state.write().next()
    }

    /// Reset the navigation cursor (stop navigating history).
    pub fn reset_navigation(&self) {
        let mut state = self.state;
        state.write().reset_navigation();
    }

    /// Returns `true` while navigating (cursor is not None).
    pub fn is_navigating(&self) -> bool {
        self.state.read().cursor.is_some()
    }

    /// Save the current search query as a draft to restore when navigation ends.
    pub fn save_draft(&self, query: &str) {
        let mut state = self.state;
        state.write().save_draft(query);
    }

    /// Take (consume) the saved draft, returning it and clearing the stored value.
    pub fn take_draft(&self) -> Option<String> {
        let mut state = self.state;
        state.write().take_draft()
    }

    /// All history entries in insertion order (oldest first).
    pub fn entries(&self) -> Vec<String> {
        self.state.read().entries_vec()
    }

    /// Clear all history entries and reset navigation.
    pub fn clear(&self) {
        let mut state = self.state;
        state.write().clear();
    }
}

/// Hook that sets up a command history store and integrates with [`CommandInput`].
///
/// Place this inside a `CommandRoot` tree. `CommandInput` will automatically
/// detect history and enable `Alt+ArrowUp` / `Alt+ArrowDown` navigation.
///
/// ```rust,ignore
/// let history = use_command_history(50); // keep last 50 entries
///
/// CommandRoot {
///     on_select: move |val: String| { history.push(&val); },
///     CommandInput {}
///     CommandList { /* items */ }
/// }
/// ```
pub fn use_command_history(capacity: usize) -> CommandHistoryHandle {
    let state: Signal<CommandHistoryState> = use_signal(|| CommandHistoryState::new(capacity));
    let handle = CommandHistoryHandle { state };
    use_context_provider(|| CommandHistoryContext { handle });
    handle
}

// ---------------------------------------------------------------------------
// P-036: use_scored_item — reactive scored state for a specific item
// ---------------------------------------------------------------------------

/// Returns a reactive [`Memo`] containing the scored state for the item with
/// the given `id`, or `None` if the item is not currently visible / has no score.
///
/// This is useful for custom components that need to reactively read fuzzy-match
/// data (score, match positions) for a specific item without subscribing to the
/// entire scored list.
///
/// Must be called inside a `CommandRoot` component tree.
///
/// # Example
///
/// ```rust,ignore
/// use dioxus_nox_cmdk::use_scored_item;
///
/// #[component]
/// fn MyScoreDisplay(id: String) -> Element {
///     let scored = use_scored_item(&id);
///     let score_text = scored.read()
///         .as_ref()
///         .and_then(|s| s.score)
///         .map(|s| s.to_string())
///         .unwrap_or_else(|| "—".to_string());
///     rsx! { span { "Score: {score_text}" } }
/// }
/// ```
pub fn use_scored_item(id: &str) -> Memo<Option<ScoredItem>> {
    let ctx = use_command_context();
    let id = id.to_string();
    use_memo(move || ctx.scored_items.read().iter().find(|s| s.id == id).cloned())
}

// ---------------------------------------------------------------------------
// P-037: Router integration hook (opt-in via `--features router`)
// ---------------------------------------------------------------------------

/// Handle returned by [`use_router_sync`] that keeps the palette search query
/// in sync with the Dioxus router's URL query parameter.
///
/// # Pure-state API
///
/// The `current_query()` and `push_query()` methods work with the underlying
/// `Signal<String>` so they can be tested without a router runtime.
/// The actual URL navigation is performed inside `use_router_sync` via
/// `dioxus_router::prelude::use_navigator`.
///
/// # Example
///
/// ```rust,ignore
/// // Inside a CommandRoot tree (router must be present in the component tree):
/// let router_sync = use_router_sync("q");
/// // The palette search is now mirrored in the URL: ?q=<query>
/// ```
#[cfg(feature = "router")]
pub struct RouterSyncHandle {
    /// The URL query parameter name (e.g. `"q"`).
    pub param_name: &'static str,
    search: Signal<String>,
}

#[cfg(feature = "router")]
impl RouterSyncHandle {
    /// Returns the current query value tracked by this handle.
    pub fn current_query(&self) -> String {
        self.search.read().clone()
    }

    /// Update the tracked query. The hook's `use_effect` will detect the change
    /// and push the new value to the router history.
    pub fn push_query(&mut self, query: &str) {
        self.search.set(query.to_string());
    }
}

/// Hook that keeps the palette search query in sync with the Dioxus router's
/// URL query parameter. Requires the `router` feature flag.
///
/// On mount it reads `?<param>` from the current URL and sets
/// `CommandContext.search`. On each search change it calls
/// `navigator.push(path?param=query)` to update the URL history.
///
/// URL reading on wasm32 uses `web_sys::window().location()`. On non-wasm
/// targets, the initial URL is not read (router history is not accessible
/// without a concrete `Route` type).
///
/// # Example
///
/// ```rust,ignore
/// #[cfg(feature = "router")]
/// let _sync = use_router_sync("q");
/// ```
#[cfg(feature = "router")]
pub fn use_router_sync(param: &'static str) -> RouterSyncHandle {
    use dioxus_router::use_navigator;

    let ctx: crate::context::CommandContext = use_context();
    let navigator = use_navigator();

    // Reactive signal that mirrors ctx.search — synced bi-directionally.
    let search = ctx.search;

    // On mount: read ?param from the current URL query string and populate search.
    // Uses web_sys on wasm32; no-op on other targets (no URL access without Route type).
    {
        use_effect(move || {
            #[cfg(target_arch = "wasm32")]
            {
                if let Some(window) = web_sys::window()
                    && let Ok(location) = window.location().search()
                    && !location.is_empty()
                {
                    // location is like "?q=hello%20world"
                    let qs = location.trim_start_matches('?');
                    for pair in qs.split('&') {
                        if let Some((key, val)) = pair.split_once('=')
                            && key == param
                        {
                            // Percent-decode spaces
                            let decoded = val.replace('+', " ").replace("%20", " ");
                            let mut s = search;
                            s.set(decoded);
                            break;
                        }
                    }
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                // Non-wasm: initial URL sync is a no-op.
                // Users can populate `ctx.search` manually if needed.
                let _ = param;
            }
        });
    }

    // On search change: push new URL to router history.
    // We rebuild the path from window.location on wasm32; on other targets
    // we push "/" as the base path (acceptable for desktop/mobile SSR).
    {
        use_effect(move || {
            let query = search.read().clone();

            // Build path: try web_sys on wasm32, fall back to "/"
            let path = {
                #[cfg(target_arch = "wasm32")]
                {
                    web_sys::window()
                        .and_then(|w| w.location().pathname().ok())
                        .unwrap_or_else(|| "/".to_string())
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    "/".to_string()
                }
            };

            let new_url = if query.is_empty() {
                path
            } else {
                let encoded = query.replace(' ', "+");
                format!("{}?{}={}", path, param, encoded)
            };
            navigator.push(new_url);
        });
    }

    RouterSyncHandle {
        param_name: param,
        search,
    }
}

// ---------------------------------------------------------------------------
// P-038: Async commands hook
// ---------------------------------------------------------------------------

/// Subscribe to the command search query and load items asynchronously.
///
/// The `provider` closure receives the current search string and returns a `Future`
/// resolving to `Result<Vec<AsyncItem>, String>`. Results are debounced by
/// `debounce_ms` milliseconds and registered into the active [`CommandContext`].
///
/// Stale responses (from cancelled debounce intervals) are discarded via a
/// generation counter.
///
/// # Example
/// ```rust,ignore
/// let handle = use_async_commands(|query| async move {
///     fetch_items(&query).await.map_err(|e| e.to_string())
/// }, 300);
/// ```
pub fn use_async_commands<F, Fut>(provider: F, debounce_ms: u32) -> AsyncCommandHandle
where
    F: Fn(String) -> Fut + 'static,
    Fut: std::future::Future<Output = Result<Vec<AsyncItem>, String>> + 'static,
{
    let ctx = use_command_context();

    let is_loading = use_signal(|| false);
    let error: Signal<Option<String>> = use_signal(|| None);
    let refresh_counter = use_signal(|| 0u32);

    // Generation counter to discard stale responses
    let generation: Rc<Cell<u32>> = use_hook(|| Rc::new(Cell::new(0u32)));

    // Track registered item IDs so we can unregister on cleanup
    let registered_ids: Rc<RefCell<HashSet<String>>> =
        use_hook(|| Rc::new(RefCell::new(HashSet::new())));

    // Pending debounce task handle
    let pending_task: Rc<RefCell<Option<dioxus_core::Task>>> =
        use_hook(|| Rc::new(RefCell::new(None)));

    let provider = Rc::new(provider);

    {
        let generation = generation.clone();
        let registered_ids = registered_ids.clone();
        let pending_task = pending_task.clone();
        let provider = provider.clone();
        let mut is_loading = is_loading;
        let mut error = error;
        let mut refresh_counter = refresh_counter;

        use_effect(move || {
            let current_search = ctx.search.read().clone();
            let gen_id = generation.get().wrapping_add(1);
            generation.set(gen_id);

            // Cancel any previous debounce task
            if let Some(t) = pending_task.borrow_mut().take() {
                t.cancel();
            }

            let generation_clone = generation.clone();
            let registered_ids_clone = registered_ids.clone();
            let provider_clone = provider.clone();

            let task = spawn(async move {
                #[cfg(target_arch = "wasm32")]
                gloo_timers::future::TimeoutFuture::new(debounce_ms).await;
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let _ = debounce_ms;
                }

                // Check if we're still the current generation
                if generation_clone.get() != gen_id {
                    return;
                }

                is_loading.set(true);
                error.set(None);

                let result = provider_clone(current_search).await;

                // Check again after await — another effect may have fired
                if generation_clone.get() != gen_id {
                    return;
                }

                is_loading.set(false);
                match result {
                    Ok(items) => {
                        // Diff: compute ids to add and remove
                        let new_ids: HashSet<String> = items.iter().map(|i| i.id.clone()).collect();
                        let old_ids = registered_ids_clone.borrow().clone();

                        // Unregister removed items
                        for id in old_ids.difference(&new_ids) {
                            ctx.unregister_item(id);
                        }

                        // Register new/updated items
                        for item in &items {
                            // If the item already exists, unregister first to replace
                            if old_ids.contains(&item.id) {
                                ctx.unregister_item(&item.id);
                            }
                            let keywords: Vec<String> = item
                                .keywords
                                .as_deref()
                                .map(|k| k.split_whitespace().map(String::from).collect())
                                .unwrap_or_default();
                            let keywords_cached = keywords.join(" ");
                            let reg = crate::types::ItemRegistration {
                                id: item.id.clone(),
                                label: item.label.clone(),
                                keywords,
                                keywords_cached,
                                value: item.value.clone(),
                                group_id: item.group.clone(),
                                disabled: item.disabled,
                                force_mount: false,
                                shortcut: None,
                                page_id: None,
                                hidden: false,
                                boost: 0,
                                mode_id: None,
                                on_select: None,
                            };
                            ctx.register_item(reg);
                        }
                        *registered_ids_clone.borrow_mut() = new_ids;
                        let next = refresh_counter.peek().wrapping_add(1);
                        refresh_counter.set(next);
                    }
                    Err(e) => {
                        error.set(Some(e));
                    }
                }
            });

            *pending_task.borrow_mut() = Some(task);
        });
    }

    // Cleanup: unregister all items on drop
    {
        let registered_ids = registered_ids.clone();
        use_drop(move || {
            let ids = registered_ids.borrow().clone();
            for id in &ids {
                ctx.unregister_item(id);
            }
        });
    }

    AsyncCommandHandle {
        is_loading,
        error,
        refresh_counter,
    }
}
