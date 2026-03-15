use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};

use dioxus::prelude::*;

use crate::types::{Extension, ExtensionInfo, PluginCommand};

static INSTANCE_COUNTER: AtomicU32 = AtomicU32::new(0);

fn next_instance_id() -> u32 {
    INSTANCE_COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Central state for the extension registry.
///
/// Holds all registered extensions and their flattened commands via Dioxus
/// [`Signal`]s. Provides methods for registration, lookup, and search.
///
/// Obtain via [`use_extension_context`] from any descendant of a component
/// that called [`use_extensions`](crate::use_extensions).
#[derive(Clone, Copy)]
pub struct ExtensionContext {
    extensions: Signal<Vec<Rc<dyn Extension>>>,
    extension_index: Signal<HashMap<String, usize>>,
    all_commands: Signal<Vec<PluginCommand>>,
    on_register: Signal<Option<EventHandler<ExtensionInfo>>>,
    on_unregister: Signal<Option<EventHandler<String>>>,
    instance_id: u32,
}

impl PartialEq for ExtensionContext {
    fn eq(&self, other: &Self) -> bool {
        self.instance_id == other.instance_id
    }
}

impl ExtensionContext {
    /// Register an extension. If an extension with the same ID already exists,
    /// it is silently replaced (unregistered then registered).
    ///
    /// Calls [`Extension::on_activate`] after registration and fires the
    /// `on_register` callback if set.
    pub fn register(&self, ext: Rc<dyn Extension>) {
        let id = ext.id().to_string();

        // Replace if already registered
        if self.extension_index.read().contains_key(&id) {
            self.unregister(&id);
        }

        let info = ExtensionInfo::from(&ext);

        let mut exts = self.extensions;
        exts.write().push(ext.clone());
        let idx = exts.read().len() - 1;
        let mut ext_index = self.extension_index;
        ext_index.write().insert(id, idx);

        self.rebuild_commands();

        ext.on_activate();

        if let Some(handler) = self.on_register.read().as_ref() {
            handler.call(info);
        }
    }

    /// Unregister an extension by ID.
    ///
    /// Calls [`Extension::on_deactivate`] before removal and fires the
    /// `on_unregister` callback if set. No-op if the ID is not found.
    pub fn unregister(&self, ext_id: &str) {
        let ext = {
            let index = self.extension_index.read();
            let Some(&idx) = index.get(ext_id) else {
                return;
            };
            self.extensions.read()[idx].clone()
        };

        ext.on_deactivate();

        let mut exts = self.extensions;
        exts.write().retain(|e| e.id() != ext_id);

        // Rebuild index after retain
        let mut ext_index = self.extension_index;
        let mut index = ext_index.write();
        index.clear();
        for (i, e) in exts.read().iter().enumerate() {
            index.insert(e.id().to_string(), i);
        }

        self.rebuild_commands();

        if let Some(handler) = self.on_unregister.read().as_ref() {
            handler.call(ext_id.to_string());
        }
    }

    /// List all registered extensions as read-only summaries.
    pub fn extensions(&self) -> Vec<ExtensionInfo> {
        self.extensions
            .read()
            .iter()
            .map(ExtensionInfo::from)
            .collect()
    }

    /// Look up a registered extension by ID. Returns `None` if not found.
    pub fn extension(&self, id: &str) -> Option<Rc<dyn Extension>> {
        let index = self.extension_index.read();
        let &idx = index.get(id)?;
        Some(self.extensions.read()[idx].clone())
    }

    /// Return all commands from all active extensions.
    pub fn commands(&self) -> Vec<PluginCommand> {
        self.all_commands.read().clone()
    }

    /// Return commands contributed by a specific extension.
    pub fn commands_for(&self, ext_id: &str) -> Vec<PluginCommand> {
        let exts = self.extensions.read();
        let index = self.extension_index.read();
        let Some(&idx) = index.get(ext_id) else {
            return Vec::new();
        };
        exts[idx].commands()
    }

    /// Search commands by case-insensitive substring matching against label,
    /// keywords, and group.
    ///
    /// For fuzzy search, integrate with the cmdk crate's nucleo pipeline instead.
    pub fn search_commands(&self, query: &str) -> Vec<PluginCommand> {
        let commands = self.all_commands.read();
        let indices = filter_commands(&commands, query);
        indices.into_iter().map(|i| commands[i].clone()).collect()
    }

    /// Rebuild the flattened command list from all registered extensions.
    fn rebuild_commands(&self) {
        let commands: Vec<PluginCommand> = self
            .extensions
            .read()
            .iter()
            .flat_map(|ext| ext.commands())
            .collect();
        let mut cmds = self.all_commands;
        *cmds.write() = commands;
    }
}

/// Case-insensitive substring filter over commands. Returns indices of matches.
///
/// Matches against label, keywords_cached, and group.
pub(crate) fn filter_commands(commands: &[PluginCommand], query: &str) -> Vec<usize> {
    if query.is_empty() {
        return (0..commands.len()).collect();
    }
    let q = query.to_lowercase();
    commands
        .iter()
        .enumerate()
        .filter(|(_, cmd)| {
            cmd.label.to_lowercase().contains(&q)
                || cmd.keywords_cached.contains(&q)
                || cmd
                    .group
                    .as_ref()
                    .is_some_and(|g| g.to_lowercase().contains(&q))
        })
        .map(|(i, _)| i)
        .collect()
}

/// Retrieve the [`ExtensionContext`] from the current Dioxus scope.
///
/// Panics if called outside a component tree that includes
/// [`use_extensions`](crate::use_extensions).
pub fn use_extension_context() -> ExtensionContext {
    use_context::<ExtensionContext>()
}

/// Initialize the extension context and provide it via Dioxus context.
pub(crate) fn init_extension_context(
    on_register: Option<EventHandler<ExtensionInfo>>,
    on_unregister: Option<EventHandler<String>>,
) -> ExtensionContext {
    let instance_id = use_hook(next_instance_id);

    let ctx = ExtensionContext {
        extensions: use_signal(Vec::new),
        extension_index: use_signal(HashMap::new),
        all_commands: use_signal(Vec::new),
        on_register: use_signal(|| on_register),
        on_unregister: use_signal(|| on_unregister),
        instance_id,
    };

    use_context_provider(|| ctx);
    ctx
}
