use crate::hook::TagInputState;
use crate::tag::TagLike;
use dioxus::prelude::*;

/// Props for [`FormValue`].
#[derive(Props, Clone, PartialEq)]
pub struct FormValueProps<T: TagLike + 'static> {
    pub name: String,
    #[props(default)]
    _phantom: std::marker::PhantomData<T>,
}

/// Hidden input for native form submission. Serializes selected tag IDs as JSON.
///
/// Renders `<input type="hidden">` with the `form_value` memo as its value.
pub fn FormValue<T: TagLike>(props: FormValueProps<T>) -> Element {
    let ctx = use_context::<TagInputState<T>>();
    let value = ctx.form_value.read();
    rsx! {
        input {
            r#type: "hidden",
            name: "{props.name}",
            value: "{value}",
        }
    }
}
