/// Where the preview pane is positioned relative to the list.
///
/// Exposed as a `data-preview-position` attribute on [`preview::Root`](super::preview::Root)
/// so consumers can target it with CSS selectors.
#[derive(Clone, Debug, PartialEq, Default)]
pub enum PreviewPosition {
    /// No positional hint — the consumer controls layout entirely.
    #[default]
    None,
    /// Preview pane is to the right of the list.
    Right,
    /// Preview pane is below the list.
    Bottom,
}

impl PreviewPosition {
    /// Returns the `data-preview-position` attribute value, or `None` when
    /// no positional hint is set.
    pub fn as_data_attr(&self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::Right => Some("right"),
            Self::Bottom => Some("bottom"),
        }
    }
}
