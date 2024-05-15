//! A prompt library for Rust. Based on [huh? for Go](https://github.com/charmbracelet/huh).

pub use confirm::Confirm;
pub use dialog::Dialog;
pub use dialog::DialogButton;
pub use input::Input;
pub use list::List;
pub use multiselect::MultiSelect;
pub use option::DemandOption;
pub use select::Select;
pub use spinner::Spinner;
pub use spinner::SpinnerStyle;
pub use theme::Theme;

mod confirm;
mod dialog;
mod input;
mod list;
mod multiselect;
mod option;
mod select;
mod spinner;
mod theme;

#[cfg(test)]
mod test;
