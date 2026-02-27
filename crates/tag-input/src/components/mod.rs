//! Radix-style compound components for tag input.
//!
//! [`Root`] creates a [`TagInputState`](crate::TagInputState) and shares it with
//! descendants via Dioxus context. All other components consume that context — no
//! prop-drilling required.
//!
//! Every component accepts `#[props(extends = GlobalAttributes)]` for attribute
//! spreading and emits a `data-slot` attribute for CSS targeting.
//!
//! # Component hierarchy
//!
//! ```text
//! Root
//! ├── Control
//! │   ├── TagList
//! │   │   └── Tag
//! │   │       ├── TagRemove
//! │   │       └── TagPopover
//! │   ├── Input
//! │   └── AutoComplete
//! ├── Dropdown
//! │   ├── DropdownGroup (optional)
//! │   └── Option
//! ├── Count
//! ├── FormValue
//! └── LiveRegion
//! ```

#![allow(non_snake_case)]

mod auto_complete;
mod control;
mod count;
mod dropdown;
mod dropdown_group;
mod form_value;
mod input;
mod live_region;
mod option;
mod root;
mod tag;
mod tag_list;
mod tag_popover;
mod tag_remove;

pub use auto_complete::AutoComplete;
pub use control::Control;
pub use count::Count;
pub use dropdown::Dropdown;
pub use dropdown_group::DropdownGroup;
pub use form_value::FormValue;
pub use input::Input;
pub use live_region::LiveRegion;
pub use option::Option;
pub use root::Root;
pub use tag::Tag;
pub use tag_list::TagList;
pub use tag_popover::TagPopover;
pub use tag_remove::TagRemove;
