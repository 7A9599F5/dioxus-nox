use dioxus::prelude::*;

/// Tab orientation — determines keyboard navigation direction and ARIA orientation.
#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub enum Orientation {
    #[default]
    Horizontal,
    Vertical,
}

impl Orientation {
    /// Value for `data-tabs-orientation` attribute.
    pub fn as_data_attr(&self) -> &'static str {
        match self {
            Self::Horizontal => "horizontal",
            Self::Vertical => "vertical",
        }
    }

    /// Value for `aria-orientation` attribute.
    pub fn as_aria_attr(&self) -> &'static str {
        self.as_data_attr()
    }
}

/// Controls when a tab panel is activated.
#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub enum ActivationMode {
    /// Tab activates immediately when it receives focus (default).
    #[default]
    Automatic,
    /// Tab activates only on explicit Space/Enter press.
    Manual,
}

/// Shared context for the tabs compound component tree.
///
/// Provided by [`super::tabs::Root`] and consumed by
/// [`super::tabs::List`], [`super::tabs::Trigger`], and [`super::tabs::Content`].
#[derive(Clone, Copy)]
pub struct TabsContext {
    /// Currently active tab value.
    pub(crate) value: Signal<String>,
    /// Controlled signal (if provided by consumer).
    pub(crate) controlled: Option<Signal<String>>,
    /// Change callback.
    pub(crate) on_value_change: Option<EventHandler<String>>,
    /// Layout direction.
    pub(crate) orientation: Orientation,
    /// Activation strategy.
    pub(crate) activation_mode: ActivationMode,
    /// Ordered list of registered (value, disabled) pairs for keyboard navigation.
    pub(crate) tabs: Signal<Vec<TabEntry>>,
}

/// Registration entry for a single tab trigger.
#[derive(Clone, PartialEq, Debug)]
pub struct TabEntry {
    pub value: String,
    pub disabled: bool,
}

impl TabsContext {
    /// Read the current active tab value.
    pub fn active_value(&self) -> String {
        match self.controlled {
            Some(sig) => (sig)(),
            None => (self.value)(),
        }
    }

    /// Activate a tab by value.
    pub fn activate(&mut self, tab_value: &str) {
        if let Some(mut controlled) = self.controlled {
            controlled.set(tab_value.to_string());
        } else {
            self.value.set(tab_value.to_string());
        }
        if let Some(handler) = &self.on_value_change {
            handler.call(tab_value.to_string());
        }
    }

    /// Register a tab trigger. Called on mount.
    pub fn register(&mut self, entry: TabEntry) {
        let mut tabs = self.tabs.write();
        if !tabs.iter().any(|e| e.value == entry.value) {
            tabs.push(entry);
        }
    }

    /// Deregister a tab trigger. Called on unmount.
    pub fn deregister(&mut self, tab_value: &str) {
        let mut tabs = self.tabs.write();
        tabs.retain(|e| e.value != tab_value);
    }

    /// Navigate to the next non-disabled tab (wrapping).
    pub fn next(&self, current: &str) -> Option<String> {
        navigate(&self.tabs.read(), current, Direction::Forward)
    }

    /// Navigate to the previous non-disabled tab (wrapping).
    pub fn prev(&self, current: &str) -> Option<String> {
        navigate(&self.tabs.read(), current, Direction::Backward)
    }

    /// Navigate to the first non-disabled tab.
    pub fn first(&self) -> Option<String> {
        self.tabs
            .read()
            .iter()
            .find(|e| !e.disabled)
            .map(|e| e.value.clone())
    }

    /// Navigate to the last non-disabled tab.
    pub fn last(&self) -> Option<String> {
        self.tabs
            .read()
            .iter()
            .rev()
            .find(|e| !e.disabled)
            .map(|e| e.value.clone())
    }

    /// Returns true if the given tab value is the active tab.
    pub fn is_active(&self, tab_value: &str) -> bool {
        self.active_value() == tab_value
    }

    /// Close a tab and return the next value to activate (if the closed tab was active).
    ///
    /// This does NOT remove the tab from the registration list — the consumer
    /// is responsible for unmounting the Trigger/Content, which triggers deregister.
    pub fn close(&mut self, tab_value: &str) -> Option<String> {
        let tabs = self.tabs.read();
        let active = self.active_value();
        if active != tab_value {
            return Some(active);
        }
        // Closing the active tab — pick neighbour
        let pos = tabs.iter().position(|e| e.value == tab_value)?;
        // Try next non-disabled, then previous non-disabled
        let next = tabs
            .iter()
            .skip(pos + 1)
            .find(|e| !e.disabled && e.value != tab_value)
            .or_else(|| {
                tabs.iter()
                    .take(pos)
                    .rev()
                    .find(|e| !e.disabled && e.value != tab_value)
            })
            .map(|e| e.value.clone());
        drop(tabs);
        if let Some(ref val) = next {
            self.activate(val);
        }
        next
    }
}

enum Direction {
    Forward,
    Backward,
}

fn navigate(tabs: &[TabEntry], current: &str, direction: Direction) -> Option<String> {
    if tabs.is_empty() {
        return None;
    }
    let cur_idx = tabs.iter().position(|e| e.value == current)?;
    let len = tabs.len();

    let step: isize = match direction {
        Direction::Forward => 1,
        Direction::Backward => -1,
    };

    // Walk up to len steps (full cycle) looking for next non-disabled
    for i in 1..=len {
        let idx = ((cur_idx as isize + step * i as isize).rem_euclid(len as isize)) as usize;
        if !tabs[idx].disabled {
            return Some(tabs[idx].value.clone());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries(specs: &[(&str, bool)]) -> Vec<TabEntry> {
        specs
            .iter()
            .map(|(v, d)| TabEntry {
                value: v.to_string(),
                disabled: *d,
            })
            .collect()
    }

    #[test]
    fn navigate_forward_wraps() {
        let tabs = entries(&[("a", false), ("b", false), ("c", false)]);
        assert_eq!(navigate(&tabs, "c", Direction::Forward), Some("a".into()));
    }

    #[test]
    fn navigate_backward_wraps() {
        let tabs = entries(&[("a", false), ("b", false), ("c", false)]);
        assert_eq!(navigate(&tabs, "a", Direction::Backward), Some("c".into()));
    }

    #[test]
    fn navigate_skips_disabled() {
        let tabs = entries(&[("a", false), ("b", true), ("c", false)]);
        assert_eq!(navigate(&tabs, "a", Direction::Forward), Some("c".into()));
    }

    #[test]
    fn navigate_all_disabled_returns_none() {
        let tabs = entries(&[("a", true), ("b", true)]);
        assert_eq!(navigate(&tabs, "a", Direction::Forward), None);
    }

    #[test]
    fn navigate_single_element() {
        let tabs = entries(&[("a", false)]);
        assert_eq!(navigate(&tabs, "a", Direction::Forward), Some("a".into()));
    }

    #[test]
    fn navigate_empty_list() {
        let tabs: Vec<TabEntry> = vec![];
        assert_eq!(navigate(&tabs, "a", Direction::Forward), None);
    }

    #[test]
    fn navigate_current_not_found() {
        let tabs = entries(&[("a", false), ("b", false)]);
        assert_eq!(navigate(&tabs, "x", Direction::Forward), None);
    }

    #[test]
    fn navigate_backward_skips_disabled() {
        let tabs = entries(&[("a", false), ("b", true), ("c", false), ("d", false)]);
        assert_eq!(navigate(&tabs, "c", Direction::Backward), Some("a".into()));
    }
}
