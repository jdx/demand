use std::{
    char,
    io::{self, Write},
};

use console::{Key, Term};
use termcolor::{Buffer, WriteColor};

use crate::{theme, Theme};

/// Single line text input
///
/// # Example
/// ```rust
/// use demand::Input;
///
/// let input = Input::new("What's your name?")
///   .description("We'll use this to personalize your experience.")
///   .placeholder("Enter your name");
/// let name = input.run().expect("error running input");
/// ````
pub struct Input<'a> {
    /// The title of the input
    pub title: String,
    /// A description to display after the title
    pub description: String,
    /// A prompt to display after the description
    pub prompt: String,
    /// A placeholder to display in the input
    pub placeholder: String,
    /// Show the input inline
    pub inline: bool,
    /// Whether to mask the input
    pub password: bool,
    /// Input entered by the user
    pub input: String,
    /// Colors/style of the input
    pub theme: &'a Theme,

    cursor: usize,
    height: usize,
    term: Term,
}

impl<'a> Input<'a> {
    /// Creates a new input with the given title.
    pub fn new<S: Into<String>>(title: S) -> Self {
        Self {
            title: title.into(),
            description: String::new(),
            placeholder: String::new(),
            prompt: "> ".to_string(),
            input: String::new(),
            inline: false,
            password: false,
            theme: &*theme::DEFAULT,
            cursor: 0,
            height: 0,
            term: Term::stderr(),
        }
    }

    /// Sets the description of the input.
    /// If the input is inline, it is displayed to the right of the title. Otherwise, it is displayed below the title.
    pub fn description(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }

    /// Sets the inline flag of the input.
    /// If true, the input is displayed inline with the title
    pub fn inline(mut self, inline: bool) -> Self {
        self.inline = inline;
        self
    }

    /// Sets the password flag of the input.
    /// If true, the input is masked with asterisks
    pub fn password(mut self, password: bool) -> Self {
        self.password = password;
        self
    }

    /// Sets the placeholder of the input.
    /// The placeholder is displayed in the input before the user enters any text
    pub fn placeholder(mut self, placeholder: &str) -> Self {
        self.placeholder = placeholder.to_string();
        self
    }

    /// Sets the prompt of the input.
    /// The prompt is displayed after the title and description. If empty, the default prompt `>` is displayed.
    pub fn prompt(mut self, prompt: &str) -> Self {
        self.prompt = prompt.to_string();
        self
    }

    /// Sets the theme of the input
    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Displays the input to the user and returns the response
    pub fn run(mut self) -> io::Result<String> {
        self.term.show_cursor()?;
        loop {
            self.clear()?;
            let output = self.render()?;

            self.height = output.lines().count() - 1;
            self.term.write_all(output.as_bytes())?;
            self.term.flush()?;
            self.set_cursor()?;

            match self.term.read_key()? {
                Key::Char(c) => self.handle_key(c)?,
                Key::Backspace => self.handle_backspace()?,
                Key::ArrowLeft => self.handle_arrow_left()?,
                Key::ArrowRight => self.handle_arrow_right()?,
                Key::Home => self.handle_home()?,
                Key::End => self.handle_end()?,
                Key::Enter => {
                    return self.handle_submit();
                }
                _ => {}
            }
        }
    }

    fn handle_key(&mut self, c: char) -> io::Result<()> {
        self.input.insert(self.cursor, c);
        self.cursor += 1;
        Ok(())
    }

    fn handle_backspace(&mut self) -> io::Result<()> {
        let chars_count = self.input.chars().count();
        if chars_count > 1 {
            self.term.move_cursor_left(1)?;
        }
        if chars_count > 0 && self.cursor > 0 {
            self.input.remove(self.cursor - 1);
        }
        if self.cursor > 0 {
            self.cursor -= 1;
        }
        Ok(())
    }

    fn handle_arrow_left(&mut self) -> io::Result<()> {
        if self.cursor > 0 {
            self.term.move_cursor_left(1)?;
            self.cursor -= 1;
        }
        Ok(())
    }

    fn handle_arrow_right(&mut self) -> io::Result<()> {
        if self.cursor < self.input.chars().count() {
            self.term.move_cursor_right(1)?;
            self.cursor += 1;
        }
        Ok(())
    }

    fn handle_home(&mut self) -> io::Result<()> {
        self.cursor = 0;
        Ok(())
    }

    fn handle_end(&mut self) -> io::Result<()> {
        self.cursor = self.input.chars().count();
        Ok(())
    }

    fn handle_submit(mut self) -> io::Result<String> {
        self.clear()?;
        let output = self.render_success()?;
        self.term.write_all(output.as_bytes())?;
        Ok(self.input)
    }

    fn render(&mut self) -> io::Result<String> {
        let mut out = Buffer::ansi();

        out.set_color(&self.theme.title)?;
        match self.inline {
            true => write!(out, "{}", self.title)?,
            false => writeln!(out, " {}", self.title)?,
        }

        out.set_color(&self.theme.description)?;
        if !self.description.is_empty() {
            match self.inline {
                true => write!(out, "{}", self.description)?,
                false => writeln!(out, " {}", self.description)?,
            }
        }

        out.set_color(&self.theme.input_prompt)?;
        if !self.prompt.is_empty() {
            match self.inline {
                true => write!(out, ">")?,
                false => write!(out, " {}", self.prompt)?,
            }
        } else {
            write!(out, " ")?;
        }
        out.reset()?;

        if !self.placeholder.is_empty() && self.input.is_empty() {
            out.set_color(&self.theme.input_placeholder)?;
            write!(out, "{}", &self.placeholder)?;
            self.term
                .move_cursor_left(self.placeholder.chars().count())?;
            out.reset()?;
        }

        write!(out, "{}", &self.render_input()?)?;

        Ok(std::str::from_utf8(out.as_slice()).unwrap().to_string())
    }

    fn render_input(&mut self) -> io::Result<String> {
        let input = match self.password {
            true => self.input.chars().map(|_| '*').collect::<String>(),
            false => self.input.to_string(),
        };
        Ok(input)
    }

    fn render_success(&mut self) -> io::Result<String> {
        let mut out = Buffer::ansi();
        out.set_color(&self.theme.title)?;
        write!(out, " {}", self.title)?;
        out.set_color(&self.theme.selected_option)?;
        writeln!(out, " {}", &self.render_input()?.to_string())?;
        out.reset()?;
        Ok(std::str::from_utf8(out.as_slice()).unwrap().to_string())
    }

    fn set_cursor(&mut self) -> io::Result<()> {
        if !self.placeholder.is_empty() && self.input.is_empty() {
            self.term
                .move_cursor_left(self.placeholder.chars().count())?;
        } else {
            self.term.move_cursor_left(self.input.chars().count())?;
        }
        self.term.move_cursor_right(self.cursor)?;
        Ok(())
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
    use crate::test::without_ansi;

    use super::*;

    #[test]
    fn test_render_title() {
        let mut input = Input::new("Title");

        assert_eq!(
            " Title\n > ",
            without_ansi(input.render().unwrap().as_str())
        );
    }

    #[test]
    fn test_render_description() {
        let mut input = Input::new("Title").description("Description");

        assert_eq!(
            " Title\n Description\n > ",
            without_ansi(input.render().unwrap().as_str())
        );
    }

    #[test]
    fn test_render_prompt() {
        let mut input = Input::new("Title").prompt("$ ");

        assert_eq!(
            " Title\n $ ",
            without_ansi(input.render().unwrap().as_str())
        );
    }

    #[test]
    fn test_render_placeholder() {
        let mut input = Input::new("Title").placeholder("Placeholder");

        assert_eq!(
            " Title\n > Placeholder",
            without_ansi(input.render().unwrap().as_str())
        );
    }

    #[test]
    fn test_render_all() {
        let mut input = Input::new("Title")
            .description("Description")
            .prompt("$ ")
            .placeholder("Placeholder");

        assert_eq!(
            " Title\n Description\n $ Placeholder",
            without_ansi(input.render().unwrap().as_str())
        );
    }
}
