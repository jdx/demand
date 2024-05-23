use std::io;
use std::io::Write;

use console::{Key, Term};
use termcolor::{Buffer, WriteColor};

use crate::theme;
use crate::theme::Theme;

#[derive(Clone, Debug, Default, PartialEq)]
/// A button to select in a dialog
pub struct DialogButton {
    /// The text to display for the option
    pub label: String,
    /// The key to press to select the option
    pub key: char,
}

impl DialogButton {
    /// Create a new button with the given label.
    /// The key will be the first character of the label, lowercased.
    pub fn new(label: &str) -> Self {
        let label = label.to_string();
        let key = label.to_lowercase().chars().next().unwrap();
        Self { label, key }
    }
    /// Create a new button with the given label and key.
    pub fn with_key(label: &str, key: char) -> Self {
        let label = label.to_string();
        Self { label, key }
    }
}

/// A dialog to display to the user
///
/// # Example
/// ```rust
/// use demand::Dialog;
/// use demand::DialogButton;
///
/// let dialog = Dialog::new("Are you sure?")
///   .description("This will do a thing.")
///   .buttons(vec![
///      DialogButton::new("Ok"),
///      DialogButton::new("Not sure"),
///      DialogButton::new("Cancel"),
///   ]);
/// let result = dialog.run().expect("error running confirm");
/// ```
pub struct Dialog<'a> {
    /// The title of the selector
    pub title: String,
    /// The colors/style of the selector
    pub theme: &'a Theme,
    /// A description to display above the selector
    pub description: String,
    /// The buttons to display to the user
    pub buttons: Vec<DialogButton>,

    term: Term,
    height: usize,
    selected_button_idx: usize,
}

impl<'a> Dialog<'a> {
    /// Create a new dialog with the given title
    ///
    /// By default, the dialog will have a single "Ok" button and a "Cancel" button.
    pub fn new<S: Into<String>>(title: S) -> Self {
        Self {
            title: title.into(),
            description: String::new(),
            theme: &*theme::DEFAULT,
            term: Term::stderr(),
            buttons: vec![DialogButton::new("Ok"), DialogButton::new("Cancel")],
            height: 0,
            selected_button_idx: 0,
        }
    }

    /// Set the description of the dialog
    pub fn description(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }

    /// Set the buttons of the dialog
    pub fn buttons(mut self, buttons: Vec<DialogButton>) -> Self {
        self.buttons = buttons;
        self
    }

    /// Set the index of the initially selected button.
    ///
    /// The `idx` is the index of the button in the `buttons` vector and is 0-indexed.
    ///
    /// # Errors
    ///
    /// This will panic if there are no buttons to select or if the index is out of bounds.
    pub fn selected_button(mut self, idx: usize) -> Self {
        if self.buttons.is_empty() {
            panic!("No buttons to select");
        }
        if idx >= self.buttons.len() {
            panic!("Selected button index out of bounds");
        }
        self.selected_button_idx = idx;
        self
    }

    /// Set the theme of the dialog
    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Displays the dialog to the user and returns their response.
    ///
    /// The response will be the label of the selected button.
    ///
    /// This will block until the user selects a button or presses one of the submit keys.
    pub fn run(mut self) -> io::Result<String> {
        loop {
            self.clear()?;
            let output = self.render()?;
            self.height = output.lines().count() - 1;
            self.term.write_all(output.as_bytes())?;
            self.term.flush()?;
            match self.term.read_key()? {
                Key::ArrowLeft | Key::Char('h') => self.handle_left(),
                Key::ArrowRight | Key::Char('l') => self.handle_right(),
                Key::Char(c) if self.buttons.iter().any(|b| b.key == c) => {
                    self.selected_button_idx =
                        self.buttons.iter().position(|b| b.key == c).unwrap();
                    return self.handle_submit();
                }
                Key::Enter => {
                    return self.handle_submit();
                }
                _ => {}
            }
        }
    }

    fn handle_submit(mut self) -> io::Result<String> {
        self.clear()?;
        let output = self.render_success()?;
        self.term.write_all(output.as_bytes())?;
        let result = if !self.buttons.is_empty() {
            self.buttons[self.selected_button_idx].label.clone()
        } else {
            "".to_string()
        };
        Ok(result)
    }

    fn handle_left(&mut self) {
        self.selected_button_idx =
            (self.selected_button_idx + self.buttons.len() - 1) % self.buttons.len();
    }

    fn handle_right(&mut self) {
        self.selected_button_idx = (self.selected_button_idx + 1) % self.buttons.len();
    }

    fn render(&self) -> io::Result<String> {
        let mut out = Buffer::ansi();

        out.set_color(&self.theme.title)?;
        writeln!(out, "{}", self.title)?;

        if !self.description.is_empty() {
            out.set_color(&self.theme.description)?;
            write!(out, "{}", self.description)?;
        }

        writeln!(out, "\n")?;

        for (i, button) in self.buttons.iter().enumerate() {
            write!(out, " ")?;
            if self.selected_button_idx == i {
                out.set_color(&self.theme.focused_button)?;
            } else {
                out.set_color(&self.theme.blurred_button)?;
            }
            write!(out, "  {}  ", button.label)?;
            out.reset()?;
        }

        writeln!(out, "\n")?;

        let mut help_keys = vec![("←/→", "toggle")];
        let button_keys = self
            .buttons
            .clone()
            .iter()
            .fold(String::new(), |mut output, button| {
                output.push_str(button.key.to_string().as_str());
                output.push('/');
                output
            });
        let submit_keys = format!("{}enter", button_keys);
        help_keys.push((&submit_keys, "submit"));
        for (i, (key, desc)) in help_keys.iter().enumerate() {
            if i > 0 {
                out.set_color(&self.theme.help_sep)?;
                write!(out, " • ")?;
            }
            out.set_color(&self.theme.help_key)?;
            write!(out, "{}", key)?;
            out.set_color(&self.theme.help_desc)?;
            write!(out, " {}", desc)?;
        }
        writeln!(out)?;

        out.reset()?;
        Ok(std::str::from_utf8(out.as_slice()).unwrap().to_string())
    }

    fn render_success(&self) -> io::Result<String> {
        let mut out = Buffer::ansi();
        out.set_color(&self.theme.title)?;
        write!(out, "{}", self.title)?;
        out.set_color(&self.theme.selected_option)?;
        writeln!(
            out,
            " {}",
            if !self.buttons.is_empty() {
                self.buttons[self.selected_button_idx].label.clone()
            } else {
                "".to_string()
            }
        )?;
        out.reset()?;
        Ok(std::str::from_utf8(out.as_slice()).unwrap().to_string())
    }

    fn clear(&mut self) -> io::Result<()> {
        self.term.clear_to_end_of_screen()?;
        self.term.clear_last_lines(self.height)?;
        self.height = 0;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::without_ansi;
    use indoc::indoc;

    #[test]
    fn test_render() {
        let dialog = Dialog::new("Are you sure?")
            .description("This will do a thing.")
            .buttons(vec![
                DialogButton::new("Ok"),
                DialogButton::new("Not sure"),
                DialogButton::new("Cancel"),
            ]);

        assert_eq!(
            indoc! {
              "Are you sure?
            This will do a thing.

               Ok     Not sure     Cancel  

            ←/→ toggle • o/n/c/enter submit
            "
            },
            without_ansi(dialog.render().unwrap().as_str())
        );
    }
}
