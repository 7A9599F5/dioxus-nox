use std::rc::Rc;

use dioxus::prelude::EventHandler;

/// Callback wrapper for [`PluginCommand::on_select`].
///
/// Wraps an [`EventHandler<String>`] where the string argument is the command ID.
/// `PartialEq` always returns `false` (callbacks cannot be compared), matching
/// the `ItemSelectCallback` pattern from the cmdk crate.
#[derive(Clone)]
pub struct CommandSelectCallback(pub EventHandler<String>);

impl PartialEq for CommandSelectCallback {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl std::fmt::Debug for CommandSelectCallback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CommandSelectCallback(..)")
    }
}

/// A command contributed by an extension.
///
/// Commands are queryable and filterable, making them suitable for integration
/// with a command palette (e.g., cmdk).
#[derive(Clone, Debug, PartialEq)]
pub struct PluginCommand {
    /// Unique identifier for this command.
    pub id: String,
    /// Human-readable label displayed in UIs.
    pub label: String,
    /// Search keywords for discoverability.
    pub keywords: Vec<String>,
    /// Pre-joined lowercase keywords for fast search matching.
    pub keywords_cached: String,
    /// Optional group/category for organizing commands.
    pub group: Option<String>,
    /// Callback invoked when the command is selected. The argument is the command ID.
    pub on_select: Option<CommandSelectCallback>,
}

impl PluginCommand {
    /// Create a new `PluginCommand` with the given id and label.
    ///
    /// Keywords, group, and on_select default to empty/None.
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            keywords: Vec::new(),
            keywords_cached: String::new(),
            group: None,
            on_select: None,
        }
    }

    /// Set the keywords for this command, pre-joining them for search.
    pub fn with_keywords(mut self, keywords: Vec<String>) -> Self {
        self.keywords_cached = keywords.join(" ").to_lowercase();
        self.keywords = keywords;
        self
    }

    /// Set the group for this command.
    pub fn with_group(mut self, group: impl Into<String>) -> Self {
        self.group = Some(group.into());
        self
    }

    /// Set the on_select callback for this command.
    pub fn with_on_select(mut self, handler: EventHandler<String>) -> Self {
        self.on_select = Some(CommandSelectCallback(handler));
        self
    }
}

/// Read-only summary of a registered extension.
#[derive(Clone, Debug, PartialEq)]
pub struct ExtensionInfo {
    /// The extension's unique identifier.
    pub id: String,
    /// The extension's display name.
    pub name: String,
    /// Number of commands contributed by this extension.
    pub command_count: usize,
}

/// Trait implemented by plugin authors to define an extension.
///
/// Extensions have a unique ID, a display name, and contribute commands.
/// The lifecycle methods [`on_activate`](Extension::on_activate) and
/// [`on_deactivate`](Extension::on_deactivate) are called when the extension
/// is registered or unregistered, respectively.
///
/// # Example
///
/// ```rust,ignore
/// use dioxus_nox_extensions::*;
///
/// struct MyExtension;
///
/// impl Extension for MyExtension {
///     fn id(&self) -> &str { "my-ext" }
///     fn name(&self) -> &str { "My Extension" }
///     fn commands(&self) -> Vec<PluginCommand> {
///         vec![PluginCommand::new("my-ext.greet", "Say Hello")]
///     }
/// }
/// ```
pub trait Extension {
    /// Unique identifier for this extension.
    fn id(&self) -> &str;

    /// Human-readable display name.
    fn name(&self) -> &str;

    /// Commands contributed by this extension.
    fn commands(&self) -> Vec<PluginCommand>;

    /// Called when the extension is registered/activated. Default: no-op.
    fn on_activate(&self) {}

    /// Called when the extension is unregistered/deactivated. Default: no-op.
    fn on_deactivate(&self) {}
}

/// Blanket conversion from `Rc<dyn Extension>` to `ExtensionInfo`.
impl From<&Rc<dyn Extension>> for ExtensionInfo {
    fn from(ext: &Rc<dyn Extension>) -> Self {
        Self {
            id: ext.id().to_string(),
            name: ext.name().to_string(),
            command_count: ext.commands().len(),
        }
    }
}
