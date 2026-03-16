// ── Pure logic tests (no signals needed) ─────────────────────────────────────

#[test]
fn trigger_element_id_format() {
    assert_eq!(crate::components::trigger_element_id("files"), "files-tab");
}

#[test]
fn panel_element_id_format() {
    assert_eq!(crate::components::panel_element_id("files"), "files-panel");
}

#[test]
fn orientation_data_attr() {
    use crate::types::Orientation;
    assert_eq!(Orientation::Horizontal.as_data_attr(), "horizontal");
    assert_eq!(Orientation::Vertical.as_data_attr(), "vertical");
}

#[test]
fn orientation_aria_attr() {
    use crate::types::Orientation;
    assert_eq!(Orientation::Horizontal.as_aria_attr(), "horizontal");
    assert_eq!(Orientation::Vertical.as_aria_attr(), "vertical");
}

#[test]
fn orientation_default_is_horizontal() {
    use crate::types::Orientation;
    assert_eq!(Orientation::default(), Orientation::Horizontal);
}

#[test]
fn activation_mode_default_is_automatic() {
    use crate::types::ActivationMode;
    assert_eq!(ActivationMode::default(), ActivationMode::Automatic);
}
