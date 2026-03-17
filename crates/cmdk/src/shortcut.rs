use keyboard_types::{Key, Modifiers};

/// Errors that can occur when parsing a hotkey string.
#[derive(Debug, Clone, PartialEq)]
pub enum HotkeyParseError {
    /// Input string was empty.
    EmptyInput,
    /// A modifier token was not recognized (e.g. "ctrll").
    UnknownModifier(String),
    /// The key portion was not a recognized key name.
    UnknownKey(String),
    /// No key was provided after the modifier(s) (e.g. "ctrl+").
    MissingKey,
}

impl std::fmt::Display for HotkeyParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyInput => write!(f, "hotkey string is empty"),
            Self::UnknownModifier(m) => write!(f, "unknown modifier: {m}"),
            Self::UnknownKey(k) => write!(f, "unknown key: {k}"),
            Self::MissingKey => write!(f, "no key provided after modifiers"),
        }
    }
}

impl std::error::Error for HotkeyParseError {}

/// A keyboard shortcut binding (e.g. Ctrl+N, Meta+Shift+K).
///
/// Used with [`CommandItem`](crate::CommandItem)'s `shortcut` prop to execute
/// an item when the matching key combination is pressed while the palette is open.
///
/// # Examples
///
/// ```
/// use dioxus_nox_cmdk::Hotkey;
/// use keyboard_types::{Key, Modifiers};
///
/// // Typed constructor
/// let hotkey = Hotkey::new(Modifiers::CONTROL, Key::Character("n".into()));
///
/// // Parse from string
/// let hotkey = Hotkey::parse("ctrl+n").unwrap();
/// assert!(Hotkey::parse("invalid++key").is_err());
/// ```
#[derive(Clone, Debug)]
pub struct Hotkey {
    pub modifiers: Modifiers,
    pub key: Key,
}

/// Mask for the four modifier keys we care about when matching shortcuts.
const MODIFIER_MASK: Modifiers = Modifiers::SHIFT
    .union(Modifiers::CONTROL)
    .union(Modifiers::ALT)
    .union(Modifiers::META);

impl Hotkey {
    /// Create a hotkey from typed modifier flags and a key.
    pub fn new(modifiers: Modifiers, key: Key) -> Self {
        Self { modifiers, key }
    }

    /// Parse a hotkey string like `"ctrl+n"`, `"meta+shift+k"`, or `"alt+enter"`.
    ///
    /// Returns an error if the string is empty, contains an unrecognized modifier
    /// or key name, or is missing a key after the modifiers.
    /// Modifier names are case-insensitive: `ctrl`, `shift`, `alt`, `meta`/`cmd`/`super`.
    pub fn parse(s: &str) -> Result<Self, HotkeyParseError> {
        if s.is_empty() {
            return Err(HotkeyParseError::EmptyInput);
        }

        let mut modifiers = Modifiers::empty();
        let parts: Vec<&str> = s.split('+').collect();

        if parts.is_empty() {
            return Err(HotkeyParseError::EmptyInput);
        }

        // All parts except the last are modifiers; the last is the key.
        let (modifier_parts, key_part) = parts.split_at(parts.len() - 1);

        for part in modifier_parts {
            match part.trim().to_lowercase().as_str() {
                "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
                "shift" => modifiers |= Modifiers::SHIFT,
                "alt" | "option" => modifiers |= Modifiers::ALT,
                "meta" | "cmd" | "command" | "super" | "win" => modifiers |= Modifiers::META,
                _ => return Err(HotkeyParseError::UnknownModifier(part.trim().to_string())),
            }
        }

        let key = parse_key(key_part[0].trim())?;
        Ok(Self { modifiers, key })
    }

    /// Parse a hotkey string, returning `None` on failure.
    /// Convenience wrapper for code that doesn't need error details.
    pub fn try_parse(s: &str) -> Option<Self> {
        Self::parse(s).ok()
    }

    /// Check whether a raw `web_sys::KeyboardEvent`'s fields match this hotkey.
    ///
    /// Takes the `key` string and individual modifier booleans from the raw event,
    /// avoiding the need to construct `keyboard_types` types in the raw listener.
    /// Character keys are compared case-insensitively.
    pub fn matches_raw(&self, key: &str, ctrl: bool, shift: bool, alt: bool, meta: bool) -> bool {
        let mut event_mods = Modifiers::empty();
        if ctrl {
            event_mods |= Modifiers::CONTROL;
        }
        if shift {
            event_mods |= Modifiers::SHIFT;
        }
        if alt {
            event_mods |= Modifiers::ALT;
        }
        if meta {
            event_mods |= Modifiers::META;
        }

        let masked = event_mods & MODIFIER_MASK;
        let expected = self.modifiers & MODIFIER_MASK;
        if masked != expected {
            return false;
        }

        match &self.key {
            Key::Character(a) => a.eq_ignore_ascii_case(key),
            other => {
                // Try to parse the raw key string to compare with non-character keys
                if let Ok(parsed) = parse_key(key) {
                    *other == parsed
                } else {
                    false
                }
            }
        }
    }

    /// Check whether a keyboard event matches this hotkey.
    ///
    /// Character keys are compared case-insensitively. Modifiers are compared
    /// exactly (masked to Shift/Ctrl/Alt/Meta — lock keys are ignored).
    pub fn matches(&self, event_key: &Key, event_modifiers: Modifiers) -> bool {
        let masked = event_modifiers & MODIFIER_MASK;
        let expected = self.modifiers & MODIFIER_MASK;

        if masked != expected {
            return false;
        }

        match (&self.key, event_key) {
            (Key::Character(a), Key::Character(b)) => a.eq_ignore_ascii_case(b),
            (a, b) => a == b,
        }
    }
}

impl PartialEq for Hotkey {
    fn eq(&self, other: &Self) -> bool {
        self.modifiers == other.modifiers && self.key == other.key
    }
}

/// Parse a single key name into a `Key` variant.
fn parse_key(s: &str) -> Result<Key, HotkeyParseError> {
    match s.to_lowercase().as_str() {
        "enter" | "return" => Ok(Key::Enter),
        "escape" | "esc" => Ok(Key::Escape),
        "tab" => Ok(Key::Tab),
        "backspace" => Ok(Key::Backspace),
        "delete" | "del" => Ok(Key::Delete),
        "space" | " " => Ok(Key::Character(" ".into())),
        "arrowup" | "up" => Ok(Key::ArrowUp),
        "arrowdown" | "down" => Ok(Key::ArrowDown),
        "arrowleft" | "left" => Ok(Key::ArrowLeft),
        "arrowright" | "right" => Ok(Key::ArrowRight),
        "home" => Ok(Key::Home),
        "end" => Ok(Key::End),
        "pageup" => Ok(Key::PageUp),
        "pagedown" => Ok(Key::PageDown),
        "" => Err(HotkeyParseError::MissingKey),
        other => {
            // Single character or multi-char key name
            let chars: Vec<char> = other.chars().collect();
            if chars.len() == 1 {
                // Store as lowercase for consistent matching
                Ok(Key::Character(chars[0].to_lowercase().to_string()))
            } else {
                // Multi-char but not a recognized special key
                Err(HotkeyParseError::UnknownKey(other.to_string()))
            }
        }
    }
}
