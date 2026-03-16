//! Tests for dioxus-nox-master-detail.

#[test]
fn master_detail_context_struct_exists() {
    // Verify the types are importable and usable.
    use crate::types::MasterDetailContext;
    // MasterDetailContext contains detail_open: bool and on_detail_close: EventHandler<()>.
    // We can't construct it outside of a Dioxus runtime due to EventHandler,
    // but we verify the type is publicly accessible.
    let _: fn() -> bool = || {
        let _ty: std::marker::PhantomData<MasterDetailContext> = std::marker::PhantomData;
        true
    };
}
