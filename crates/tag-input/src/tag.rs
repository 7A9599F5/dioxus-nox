/// Trait for types that can be used as tags in the tag input.
///
/// Implementors must provide a unique `id` and a display `name`.
pub trait TagLike: Clone + PartialEq + 'static {
    /// Returns the unique identifier for this tag.
    fn id(&self) -> &str;

    /// Returns the display name for this tag.
    fn name(&self) -> &str;

    /// Returns the group/category label for this tag, if any.
    ///
    /// Used by `use_tag_input_grouped()` to organize suggestions into sections.
    /// Returns `None` by default (all items in a single ungrouped section).
    fn group(&self) -> Option<&str> {
        None
    }

    /// Whether this tag is locked (cannot be removed by the user).
    ///
    /// Locked tags are excluded from Backspace/Delete handling in pill mode
    /// and should not render a remove button. Consumers check `tag.is_locked()`
    /// to conditionally hide the remove button in their UI.
    ///
    /// Default: `false`
    fn is_locked(&self) -> bool {
        false
    }
}

/// A simple tag with an `id` and a `name`.
#[derive(Clone, PartialEq, Debug)]
pub struct Tag {
    pub id: String,
    pub name: String,
}

impl Tag {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
        }
    }
}

impl TagLike for Tag {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }
}
