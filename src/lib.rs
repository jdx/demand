//! A prompt library for Rust. Based on [huh? for Go](https://github.com/charmbracelet/huh).

pub use confirm::Confirm;
pub use multiselect::MultiSelect;
pub use option::DemandOption;
pub use select::Select;
pub use theme::Theme;

mod confirm;
mod multiselect;
mod option;
mod select;
mod theme;
