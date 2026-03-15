use std::cell::RefCell;
use std::rc::Rc;

use crate::context::filter_commands;
use crate::types::{Extension, ExtensionInfo, PluginCommand};

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

/// A concrete Extension implementation for testing.
struct TestExtension {
    id: &'static str,
    name: &'static str,
    commands: Vec<PluginCommand>,
    activated: Rc<RefCell<bool>>,
    deactivated: Rc<RefCell<bool>>,
}

impl TestExtension {
    fn new(id: &'static str, name: &'static str) -> Self {
        Self {
            id,
            name,
            commands: Vec::new(),
            activated: Rc::new(RefCell::new(false)),
            deactivated: Rc::new(RefCell::new(false)),
        }
    }

    fn with_commands(mut self, commands: Vec<PluginCommand>) -> Self {
        self.commands = commands;
        self
    }
}

impl Extension for TestExtension {
    fn id(&self) -> &str {
        self.id
    }

    fn name(&self) -> &str {
        self.name
    }

    fn commands(&self) -> Vec<PluginCommand> {
        self.commands.clone()
    }

    fn on_activate(&self) {
        *self.activated.borrow_mut() = true;
    }

    fn on_deactivate(&self) {
        *self.deactivated.borrow_mut() = true;
    }
}

/// Minimal extension with default lifecycle (no-op).
struct MinimalExtension;

impl Extension for MinimalExtension {
    fn id(&self) -> &str {
        "minimal"
    }
    fn name(&self) -> &str {
        "Minimal"
    }
    fn commands(&self) -> Vec<PluginCommand> {
        Vec::new()
    }
}

// ---------------------------------------------------------------------------
// PluginCommand tests
// ---------------------------------------------------------------------------

#[test]
fn test_plugin_command_new() {
    let cmd = PluginCommand::new("cmd-1", "Do Something");
    assert_eq!(cmd.id, "cmd-1");
    assert_eq!(cmd.label, "Do Something");
    assert!(cmd.keywords.is_empty());
    assert!(cmd.keywords_cached.is_empty());
    assert!(cmd.group.is_none());
    assert!(cmd.on_select.is_none());
}

#[test]
fn test_plugin_command_builder() {
    let cmd = PluginCommand::new("cmd-2", "Search Files")
        .with_keywords(vec!["find".into(), "lookup".into()])
        .with_group("Navigation");

    assert_eq!(cmd.id, "cmd-2");
    assert_eq!(cmd.label, "Search Files");
    assert_eq!(cmd.keywords, vec!["find", "lookup"]);
    assert_eq!(cmd.keywords_cached, "find lookup");
    assert_eq!(cmd.group.as_deref(), Some("Navigation"));
}

#[test]
fn test_plugin_command_keywords_cached_lowercase() {
    let cmd = PluginCommand::new("c", "C").with_keywords(vec!["FOO".into(), "Bar".into()]);
    assert_eq!(cmd.keywords_cached, "foo bar");
}

#[test]
fn test_plugin_command_equality() {
    let a = PluginCommand::new("x", "X");
    let b = PluginCommand::new("x", "X");
    assert_eq!(a, b);

    let c = PluginCommand::new("x", "Y");
    assert_ne!(a, c);
}

// ---------------------------------------------------------------------------
// CommandSelectCallback tests
// ---------------------------------------------------------------------------

// Note: EventHandler::new requires a Dioxus runtime, so callback tests that
// construct EventHandler values cannot run as plain unit tests. The
// CommandSelectCallback wrapper is a trivial newtype — its PartialEq (always
// false) and Debug impls are verified by inspection.

// ---------------------------------------------------------------------------
// Extension trait tests
// ---------------------------------------------------------------------------

#[test]
fn test_extension_trait_implementation() {
    let ext = TestExtension::new("test-ext", "Test Extension").with_commands(vec![
        PluginCommand::new("test-ext.a", "Command A"),
        PluginCommand::new("test-ext.b", "Command B"),
    ]);

    assert_eq!(ext.id(), "test-ext");
    assert_eq!(ext.name(), "Test Extension");
    assert_eq!(ext.commands().len(), 2);
    assert_eq!(ext.commands()[0].id, "test-ext.a");
}

#[test]
fn test_extension_default_lifecycle_does_not_panic() {
    let ext = MinimalExtension;
    ext.on_activate();
    ext.on_deactivate();
}

#[test]
fn test_extension_lifecycle_tracking() {
    let ext = TestExtension::new("lc", "Lifecycle");
    let activated = ext.activated.clone();
    let deactivated = ext.deactivated.clone();

    assert!(!*activated.borrow());
    assert!(!*deactivated.borrow());

    ext.on_activate();
    assert!(*activated.borrow());
    assert!(!*deactivated.borrow());

    ext.on_deactivate();
    assert!(*deactivated.borrow());
}

// ---------------------------------------------------------------------------
// ExtensionInfo tests
// ---------------------------------------------------------------------------

#[test]
fn test_extension_info_from_extension() {
    let ext: Rc<dyn Extension> = Rc::new(
        TestExtension::new("info-ext", "Info Extension").with_commands(vec![
            PluginCommand::new("c1", "C1"),
            PluginCommand::new("c2", "C2"),
            PluginCommand::new("c3", "C3"),
        ]),
    );

    let info = ExtensionInfo::from(&ext);
    assert_eq!(info.id, "info-ext");
    assert_eq!(info.name, "Info Extension");
    assert_eq!(info.command_count, 3);
}

#[test]
fn test_extension_info_clone_eq() {
    let info = ExtensionInfo {
        id: "a".into(),
        name: "A".into(),
        command_count: 1,
    };
    let cloned = info.clone();
    assert_eq!(info, cloned);
}

// ---------------------------------------------------------------------------
// filter_commands tests (pure function, no Dioxus runtime needed)
// ---------------------------------------------------------------------------

fn sample_commands() -> Vec<PluginCommand> {
    vec![
        PluginCommand::new("open-file", "Open File")
            .with_keywords(vec!["browse".into(), "load".into()])
            .with_group("File"),
        PluginCommand::new("save-file", "Save File")
            .with_keywords(vec!["write".into(), "export".into()])
            .with_group("File"),
        PluginCommand::new("toggle-theme", "Toggle Theme")
            .with_keywords(vec!["dark".into(), "light".into()])
            .with_group("Appearance"),
        PluginCommand::new("run-tests", "Run Tests")
            .with_keywords(vec!["test".into(), "check".into()])
            .with_group("Development"),
    ]
}

#[test]
fn test_filter_empty_query_returns_all() {
    let cmds = sample_commands();
    let result = filter_commands(&cmds, "");
    assert_eq!(result, vec![0, 1, 2, 3]);
}

#[test]
fn test_filter_by_label() {
    let cmds = sample_commands();
    let result = filter_commands(&cmds, "Open");
    assert_eq!(result, vec![0]);
}

#[test]
fn test_filter_by_label_case_insensitive() {
    let cmds = sample_commands();
    let result = filter_commands(&cmds, "open file");
    assert_eq!(result, vec![0]);
}

#[test]
fn test_filter_by_keyword() {
    let cmds = sample_commands();
    let result = filter_commands(&cmds, "dark");
    assert_eq!(result, vec![2]);
}

#[test]
fn test_filter_by_group() {
    let cmds = sample_commands();
    let result = filter_commands(&cmds, "File");
    // "Open File" matches label, "Save File" matches label, both in group "File"
    assert_eq!(result, vec![0, 1]);
}

#[test]
fn test_filter_by_group_only() {
    let cmds = sample_commands();
    let result = filter_commands(&cmds, "Appearance");
    assert_eq!(result, vec![2]);
}

#[test]
fn test_filter_partial_match() {
    let cmds = sample_commands();
    // "Tog" matches "Toggle Theme" label
    let result = filter_commands(&cmds, "Tog");
    assert_eq!(result, vec![2]);
}

#[test]
fn test_filter_no_match() {
    let cmds = sample_commands();
    let result = filter_commands(&cmds, "nonexistent");
    assert!(result.is_empty());
}

#[test]
fn test_filter_keyword_cached_match() {
    let cmds = sample_commands();
    // "export" is a keyword on "Save File"
    let result = filter_commands(&cmds, "export");
    assert_eq!(result, vec![1]);
}

#[test]
fn test_filter_multiple_matches() {
    let cmds = sample_commands();
    // "test" matches "Run Tests" label and keyword
    let result = filter_commands(&cmds, "test");
    assert_eq!(result, vec![3]);
}

#[test]
fn test_filter_empty_commands() {
    let cmds: Vec<PluginCommand> = Vec::new();
    let result = filter_commands(&cmds, "anything");
    assert!(result.is_empty());
}

#[test]
fn test_filter_empty_commands_empty_query() {
    let cmds: Vec<PluginCommand> = Vec::new();
    let result = filter_commands(&cmds, "");
    assert!(result.is_empty());
}
