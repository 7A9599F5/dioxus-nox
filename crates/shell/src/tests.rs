#[cfg(test)]
mod tests {
    use crate::{
        ShellLayout,
        breakpoint::{BreakpointConfig, DesktopSidebar, MobileSidebar, SheetSnap, ShellBreakpoint},
    };

    // ── ShellLayout ────────────────────────────────────────────────────────────

    #[test]
    fn shell_layout_horizontal_attr() {
        assert_eq!(ShellLayout::Horizontal.as_data_attr(), "horizontal");
    }

    #[test]
    fn shell_layout_vertical_attr() {
        assert_eq!(ShellLayout::Vertical.as_data_attr(), "vertical");
    }

    #[test]
    fn shell_layout_sidebar_attr() {
        assert_eq!(ShellLayout::Sidebar.as_data_attr(), "sidebar");
    }

    #[test]
    fn shell_layout_display_horizontal() {
        assert_eq!(format!("{}", ShellLayout::Horizontal), "horizontal");
    }

    #[test]
    fn shell_layout_display_vertical() {
        assert_eq!(format!("{}", ShellLayout::Vertical), "vertical");
    }

    #[test]
    fn shell_layout_display_sidebar() {
        assert_eq!(format!("{}", ShellLayout::Sidebar), "sidebar");
    }

    #[test]
    fn shell_layout_default_is_horizontal() {
        assert_eq!(ShellLayout::default(), ShellLayout::Horizontal);
    }

    #[test]
    fn shell_layout_partial_eq() {
        assert_ne!(ShellLayout::Horizontal, ShellLayout::Sidebar);
    }

    #[test]
    fn shell_layout_clone_copy() {
        let a = ShellLayout::Vertical;
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn shell_layout_debug() {
        let _ = format!("{:?}", ShellLayout::Sidebar);
    }

    // ── ShellBreakpoint ────────────────────────────────────────────────────────

    #[test]
    fn shell_breakpoint_as_str_compact() {
        assert_eq!(ShellBreakpoint::Compact.as_str(), "compact");
    }

    #[test]
    fn shell_breakpoint_as_str_medium() {
        assert_eq!(ShellBreakpoint::Medium.as_str(), "medium");
    }

    #[test]
    fn shell_breakpoint_as_str_expanded() {
        assert_eq!(ShellBreakpoint::Expanded.as_str(), "expanded");
    }

    #[test]
    fn shell_breakpoint_is_mobile_compact_true() {
        assert!(ShellBreakpoint::Compact.is_mobile());
    }

    #[test]
    fn shell_breakpoint_is_mobile_medium_false() {
        assert!(!ShellBreakpoint::Medium.is_mobile());
    }

    #[test]
    fn shell_breakpoint_is_mobile_expanded_false() {
        assert!(!ShellBreakpoint::Expanded.is_mobile());
    }

    #[test]
    fn shell_breakpoint_is_compact_compact_true() {
        assert!(ShellBreakpoint::Compact.is_compact());
    }

    #[test]
    fn shell_breakpoint_is_compact_expanded_false() {
        assert!(!ShellBreakpoint::Expanded.is_compact());
    }

    #[test]
    fn shell_breakpoint_default_is_medium() {
        assert_eq!(ShellBreakpoint::default(), ShellBreakpoint::Medium);
    }

    #[test]
    fn shell_breakpoint_clone_copy() {
        let a = ShellBreakpoint::Compact;
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn shell_breakpoint_debug() {
        let _ = format!("{:?}", ShellBreakpoint::Medium);
    }

    // ── BreakpointConfig ───────────────────────────────────────────────────────

    #[test]
    fn breakpoint_config_default_values() {
        let cfg = BreakpointConfig::default();
        assert_eq!(cfg.compact_below, 640.0);
        assert_eq!(cfg.expanded_above, 1024.0);
    }

    #[test]
    fn breakpoint_config_clone_copy() {
        let a = BreakpointConfig::default();
        let b = a;
        assert_eq!(a, b);
    }

    // ── DesktopSidebar ─────────────────────────────────────────────────────────

    #[test]
    fn desktop_sidebar_default_is_full() {
        assert_eq!(DesktopSidebar::default(), DesktopSidebar::Full);
    }

    #[test]
    fn desktop_sidebar_variants_ne() {
        assert_ne!(DesktopSidebar::Full, DesktopSidebar::Rail);
        assert_ne!(DesktopSidebar::Full, DesktopSidebar::Expandable);
        assert_ne!(DesktopSidebar::Rail, DesktopSidebar::Expandable);
    }

    #[test]
    fn desktop_sidebar_clone_copy() {
        let a = DesktopSidebar::Expandable;
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn desktop_sidebar_debug() {
        let _ = format!("{:?}", DesktopSidebar::Rail);
    }

    // ── MobileSidebar ──────────────────────────────────────────────────────────

    #[test]
    fn mobile_sidebar_default_is_drawer() {
        assert_eq!(MobileSidebar::default(), MobileSidebar::Drawer);
    }

    #[test]
    fn mobile_sidebar_variants_ne() {
        assert_ne!(MobileSidebar::Drawer, MobileSidebar::Rail);
        assert_ne!(MobileSidebar::Drawer, MobileSidebar::Hidden);
        assert_ne!(MobileSidebar::Rail, MobileSidebar::Hidden);
    }

    #[test]
    fn mobile_sidebar_clone_copy() {
        let a = MobileSidebar::Rail;
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn mobile_sidebar_debug() {
        let _ = format!("{:?}", MobileSidebar::Drawer);
    }

    // ── SheetSnap ──────────────────────────────────────────────────────────────

    #[test]
    fn sheet_snap_default_is_hidden() {
        assert_eq!(SheetSnap::default(), SheetSnap::Hidden);
    }

    #[test]
    fn sheet_snap_as_str() {
        assert_eq!(SheetSnap::Hidden.as_str(), "hidden");
        assert_eq!(SheetSnap::Peek.as_str(), "peek");
        assert_eq!(SheetSnap::Half.as_str(), "half");
        assert_eq!(SheetSnap::Full.as_str(), "full");
    }

    #[test]
    fn sheet_snap_is_visible() {
        assert!(!SheetSnap::Hidden.is_visible());
        assert!(SheetSnap::Peek.is_visible());
        assert!(SheetSnap::Half.is_visible());
        assert!(SheetSnap::Full.is_visible());
    }

    #[test]
    fn sheet_snap_clone_copy() {
        let a = SheetSnap::Half;
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn sheet_snap_debug() {
        let _ = format!("{:?}", SheetSnap::Full);
    }
}
