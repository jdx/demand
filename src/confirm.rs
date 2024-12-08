use std::io;
use std::io::Write;

use console::{Key, Term};
use termcolor::{Buffer, WriteColor};

use crate::theme;
use crate::theme::Theme;

/// Select multiple options from a list
///
/// # Example
/// ```rust
/// use demand::Confirm;
///
/// let confirm = Confirm::new("Are you sure?")
///   .affirmative("Yes!")
///   .negative("No.");
/// let choice = match confirm.run() {
///     Ok(confirm) => confirm,
///     Err(e) => {
///         if e.kind() == std::io::ErrorKind::Interrupted {
///             println!("Dialog cancelled");
///             false
///         } else {
///             panic!("Error: {}", e);
///         }
///     }
/// };
/// ```
pub struct Confirm<'a> {
    /// The title of the selector
    pub title: String,
    /// The colors/style of the selector
    pub theme: &'a Theme,
    /// A description to display above the selector
    pub description: String,
    /// The text to display for the affirmative option
    pub affirmative: String,
    /// The text to display for the negative option
    pub negative: String,
    /// If true, the affirmative option is selected by default
    pub selected: bool,
    term: Term,
    height: usize,
}

impl<'a> Confirm<'a> {
    /// Create a new multi select with the given title
    pub fn new<S: Into<String>>(title: S) -> Self {
        Self {
            title: title.into(),
            description: String::new(),
            theme: &*theme::DEFAULT,
            term: Term::stderr(),
            affirmative: "Yes".to_string(),
            negative: "No".to_string(),
            selected: true,
            height: 0,
        }
    }

    /// Set the description of the selector
    pub fn description(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }

    /// Set the label of the affirmative option
    pub fn affirmative<S: Into<String>>(mut self, affirmative: S) -> Self {
        self.affirmative = affirmative.into();
        self
    }

    /// Set the label of the negative option
    pub fn negative<S: Into<String>>(mut self, negative: S) -> Self {
        self.negative = negative.into();
        self
    }

    /// Set whether the affirmative option is selected by default
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Set the theme of the dialog
    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Displays the dialog to the user and returns their response
    ///
    /// This function will block until the user submits the input. If the user cancels the input,
    /// an error of type `io::ErrorKind::Interrupted` is returned.
    pub fn run(mut self) -> io::Result<bool> {
        let affirmative_char = self.affirmative.to_lowercase().chars().next().unwrap();
        let negative_char = self.negative.to_lowercase().chars().next().unwrap();
        self.term.clear_line()?;
        loop {
            self.clear()?;
            let output = self.render()?;
            self.height = output.lines().count() - 1;
            self.term.write_all(output.as_bytes())?;
            self.term.flush()?;
            match self.term.read_key()? {
                Key::ArrowLeft | Key::Char('h') => self.handle_left(),
                Key::ArrowRight | Key::Char('l') => self.handle_right(),
                Key::Char(c) if c == affirmative_char => {
                    self.selected = true;
                    return self.handle_submit();
                }
                Key::Char(c) if c == negative_char => {
                    self.selected = false;
                    return self.handle_submit();
                }
                Key::Enter => {
                    return self.handle_submit();
                }
                Key::Escape => {
                    self.clear()?;
                    return Err(io::Error::new(io::ErrorKind::Interrupted, "user cancelled"));
                }
                _ => {}
            }
        }
    }

    fn handle_submit(mut self) -> io::Result<bool> {
        self.term.clear_to_end_of_screen()?;
        self.clear()?;
        let output = self.render_success()?;
        self.term.write_all(output.as_bytes())?;
        Ok(self.selected)
    }

    fn handle_left(&mut self) {
        if !self.selected {
            self.selected = true;
        }
    }

    fn handle_right(&mut self) {
        if self.selected {
            self.selected = false;
        }
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

        write!(out, " ")?;
        if self.selected {
            out.set_color(&self.theme.focused_button)?;
        } else {
            out.set_color(&self.theme.blurred_button)?;
        }
        write!(out, "  {}  ", self.affirmative)?;
        out.reset()?;
        write!(out, " ")?;
        if self.selected {
            out.set_color(&self.theme.blurred_button)?;
        } else {
            out.set_color(&self.theme.focused_button)?;
        }
        write!(out, "  {}  ", self.negative)?;
        out.reset()?;
        writeln!(out, "\n")?;

        let mut help_keys = vec![("←/→", "toggle")];
        let affirmative_char = self.affirmative.to_lowercase().chars().next().unwrap();
        let negative_char = self.negative.to_lowercase().chars().next().unwrap();
        let submit_keys = format!("{affirmative_char}/{negative_char}/enter");
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
        if self.selected {
            writeln!(out, " {}", self.affirmative)?;
        } else {
            writeln!(out, " {}", self.negative)?;
        }
        out.reset()?;
        Ok(std::str::from_utf8(out.as_slice()).unwrap().to_string())
    }

    fn clear(&mut self) -> io::Result<()> {
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
        let confirm = Confirm::new("Are you sure?")
            .description("This will do a thing.")
            .affirmative("Yes!")
            .negative("No.");

        assert_eq!(
            indoc! {
              "Are you sure?
             This will do a thing.

                Yes!     No.  

             ←/→ toggle • y/n/enter submit
            "
            },
            without_ansi(confirm.render().unwrap().as_str())
        );
    }
}
