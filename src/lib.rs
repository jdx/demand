//! A prompt library for Rust. Based on [huh? for Go](https://github.com/charmbracelet/huh).

pub use confirm::Confirm;
pub use dialog::Dialog;
pub use dialog::DialogButton;
pub use input::{
    Autocomplete, AutocompleteClone, FnAutocomplete, Input, InputValidator, NoAutocompletion,
};
pub use list::List;
pub use multiselect::MultiSelect;
pub use option::DemandOption;
pub use select::Select;
pub use spinner::Spinner;
pub use spinner::SpinnerStyle;
pub use theme::Theme;
pub use wizard::{Navigation, Wizard, handle_navigation_key};

mod confirm;
#[cfg_attr(any(windows), path = "ctrlc_stub.rs")]
mod ctrlc;
mod dialog;
mod event;
mod fuzzy;
mod height;
mod input;
mod keys;
mod list;
mod multiselect;
mod option;
mod select;
mod spinner;
mod synchronized_output;
mod theme;
mod tty;
mod wizard;

#[cfg(test)]
mod test;
