//! A prompt library for Rust. Based on [huh? for Go](https://github.com/charmbracelet/huh).

pub use multiselect::MultiSelect;
pub use option::DemandOption;
pub use theme::Theme;

mod multiselect;
mod option;
mod theme;
