//! Tests for dioxus-nox-modal.

#[test]
fn modal_types_are_accessible() {
    // Verify the types are importable.
    use crate::types::ModalContext;
    let _: std::marker::PhantomData<ModalContext> = std::marker::PhantomData;
}

#[test]
fn modal_handle_type_is_accessible() {
    use crate::types::ModalHandle;
    let _: std::marker::PhantomData<ModalHandle> = std::marker::PhantomData;
}
