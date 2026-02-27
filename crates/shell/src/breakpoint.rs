use dioxus::prelude::*;

/// Viewport size category detected via `matchMedia` listeners.
///
/// The default value (before any JS event fires) is [`Medium`][ShellBreakpoint::Medium].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ShellBreakpoint {
    /// Viewport width < `compact_below` (default 640 px). Typical phones.
    Compact,
    /// Viewport width between `compact_below` and `expanded_above`. Typical tablets.
    #[default]
    Medium,
    /// Viewport width >= `expanded_above` (default 1024 px). Typical desktops.
    Expanded,
}

impl ShellBreakpoint {
    /// Returns the lowercase string used for the `data-shell-breakpoint` attribute.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Compact  => "compact",
            Self::Medium   => "medium",
            Self::Expanded => "expanded",
        }
    }

    /// `true` when the viewport is in the compact (phone) range.
    pub fn is_compact(&self) -> bool {
        *self == Self::Compact
    }

    /// Alias for [`is_compact`][Self::is_compact] — reflects mobile-first terminology.
    pub fn is_mobile(&self) -> bool {
        self.is_compact()
    }
}

/// How the sidebar behaves on compact (mobile) viewports.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MobileSidebar {
    /// Fixed-position overlay drawer (shadcn Sheet pattern). Default.
    #[default]
    Drawer,
    /// Icon-only narrow strip alongside main content (Flutter NavigationRail style).
    Rail,
    /// Sidebar is removed entirely; consumers provide alternative navigation.
    Hidden,
}

/// How the sidebar behaves on desktop (non-compact) viewports.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DesktopSidebar {
    /// Full-width sidebar. `toggle_sidebar` collapses it to zero width. Default.
    #[default]
    Full,
    /// Permanent narrow icon-only rail (~56 px). Width never changes; `toggle_sidebar` is a no-op.
    Rail,
    /// Rail that expands into a full sidebar on toggle.
    ///
    /// `sidebar_visible = true` → full width; `false` → rail width.
    /// `toggle_sidebar` switches between the two states.
    Expandable,
}

/// Snap state for a persistent bottom sheet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SheetSnap {
    /// Sheet is hidden (0 height). Default.
    #[default]
    Hidden,
    /// Sheet peeks at the bottom (~15–25% height).
    Peek,
    /// Sheet is half-height (~50%).
    Half,
    /// Sheet is fully expanded (~90%).
    Full,
}

impl SheetSnap {
    /// Returns the lowercase string used for the `data-shell-sheet-state` attribute.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Hidden => "hidden",
            Self::Peek   => "peek",
            Self::Half   => "half",
            Self::Full   => "full",
        }
    }

    /// `true` when the sheet is visible at any snap point.
    pub fn is_visible(&self) -> bool {
        *self != Self::Hidden
    }
}

/// Grouped breakpoint thresholds for [`AppShell`][crate::AppShell] and
/// [`use_shell_breakpoint`].
///
/// Pass a custom value to override the defaults:
/// ```rust,ignore
/// AppShell {
///     breakpoints: BreakpointConfig { compact_below: 480.0, expanded_above: 1280.0 },
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BreakpointConfig {
    /// Viewport width (px) below which the layout is considered compact. Default: 640.
    pub compact_below: f64,
    /// Viewport width (px) at or above which the layout is considered expanded. Default: 1024.
    pub expanded_above: f64,
}

impl Default for BreakpointConfig {
    fn default() -> Self {
        Self { compact_below: 640.0, expanded_above: 1024.0 }
    }
}

/// Detects the viewport breakpoint via `matchMedia` listeners fed through
/// [`document::eval`].
///
/// Returns a [`ReadSignal`] that updates reactively on resize events.
/// The initial value is [`ShellBreakpoint::Medium`] until the first JS event
/// fires on the next microtask after mount. When the eval channel closes
/// (unsupported backends), the value falls back to `Medium`.
///
/// **Requires a JavaScript engine.** Supported on all WebView targets:
/// web WASM, Wry desktop, iOS WKWebView, and Android WebView.
pub fn use_shell_breakpoint(compact_below: f64, expanded_above: f64) -> ReadSignal<ShellBreakpoint> {
    use_shell_breakpoint_runtime(compact_below, expanded_above)
}

/// Private composite hook — encapsulates eval-channel logic so
/// `use_shell_breakpoint` can remain a thin, stable public entry point.
fn use_shell_breakpoint_runtime(compact_below: f64, expanded_above: f64) -> ReadSignal<ShellBreakpoint> {
    let mut bp = use_signal(|| ShellBreakpoint::Medium);
    use_effect(move || {
        spawn(async move {
            let compact_max = compact_below - 1.0;
            let js = format!(r#"
                function getBp() {{
                    const w = window.innerWidth;
                    if (w < {compact_below}) return "compact";
                    if (w >= {expanded_above}) return "expanded";
                    return "medium";
                }}
                dioxus.send(getBp());
                const mqlC = window.matchMedia("(max-width: {compact_max}px)");
                const mqlE = window.matchMedia("(min-width: {expanded_above}px)");
                function onChange() {{ dioxus.send(getBp()); }}
                mqlC.addEventListener("change", onChange);
                mqlE.addEventListener("change", onChange);
            "#);
            let mut eval = document::eval(&js);
            while let Ok(v) = eval.recv::<String>().await {
                bp.set(match v.as_str() {
                    "compact" => ShellBreakpoint::Compact,
                    "medium"  => ShellBreakpoint::Medium,
                    _         => ShellBreakpoint::Expanded,
                });
            }
            // Eval channel closed — deterministic fallback for unsupported backends.
            bp.set(ShellBreakpoint::Medium);
        });
    });
    bp.into()
}
