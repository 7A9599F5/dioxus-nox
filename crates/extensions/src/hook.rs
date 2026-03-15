use std::rc::Rc;

use dioxus::prelude::EventHandler;

use crate::context::{ExtensionContext, init_extension_context};
use crate::types::{Extension, ExtensionInfo};

/// Handle returned by [`use_extensions`], providing register/unregister/query
/// capabilities for the extension system.
///
/// This is the primary API surface for extension management. It wraps
/// [`ExtensionContext`] with a convenience interface.
#[derive(Clone, Copy)]
pub struct ExtensionHandle {
    ctx: ExtensionContext,
}

impl ExtensionHandle {
    /// Register an extension. Calls `on_activate()` and fires the register callback.
    pub fn register(&self, ext: Rc<dyn Extension>) {
        self.ctx.register(ext);
    }

    /// Unregister an extension by ID. Calls `on_deactivate()` and fires the unregister callback.
    pub fn unregister(&self, ext_id: &str) {
        self.ctx.unregister(ext_id);
    }

    /// List all registered extensions as [`ExtensionInfo`] summaries.
    pub fn extensions(&self) -> Vec<ExtensionInfo> {
        self.ctx.extensions()
    }

    /// Check if an extension with the given ID is currently registered.
    pub fn is_registered(&self, ext_id: &str) -> bool {
        self.ctx.extension(ext_id).is_some()
    }

    /// Access the underlying [`ExtensionContext`] for advanced operations
    /// (e.g., command search, per-extension command listing).
    pub fn context(&self) -> ExtensionContext {
        self.ctx
    }
}

/// Initialize the extension system and return a handle.
///
/// Call this once at the top of your component tree. It provides
/// [`ExtensionContext`] via Dioxus context so descendant components can access
/// it with [`use_extension_context`](crate::use_extension_context).
///
/// # Arguments
///
/// * `on_register` — Optional callback fired when an extension is registered.
/// * `on_unregister` — Optional callback fired with the extension ID when unregistered.
///
/// # Example
///
/// ```rust,ignore
/// use dioxus::prelude::*;
/// use dioxus_nox_extensions::*;
/// use std::rc::Rc;
///
/// #[component]
/// fn App() -> Element {
///     let handle = use_extensions(None, None);
///     // handle.register(Rc::new(MyExtension));
///     rsx! { div { "Extensions loaded: {handle.extensions().len()}" } }
/// }
/// ```
pub fn use_extensions(
    on_register: Option<EventHandler<ExtensionInfo>>,
    on_unregister: Option<EventHandler<String>>,
) -> ExtensionHandle {
    let ctx = init_extension_context(on_register, on_unregister);
    ExtensionHandle { ctx }
}
