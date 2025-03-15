use std::{
    char,
    io::{self, Write},
};

use console::{Key, Term, measure_text_width};
use termcolor::{Buffer, WriteColor};

use crate::ctrlc;
use crate::{Theme, theme};

/// Single line text input
///
/// # Example
/// ```rust
/// use demand::Input;
///
/// let input = Input::new("What's your name?")
///   .description("We'll use this to personalize your experience.")
///   .placeholder("Enter your name");
/// let name = match input.run() {
///   Ok(value) => value,
///   Err(e) => {
///       if e.kind() == std::io::ErrorKind::Interrupted {
///           println!("Input cancelled");
///           return;
///       } else {
///           panic!("Error: {}", e);
///       }
///   }
/// };
/// ```
pub struct Input<'a> {
    /// The title of the input
    pub title: String,
    /// A description to display after the title
    pub description: String,
    /// A prompt to display after the description
    pub prompt: String,
    /// A placeholder to display in the input
    pub placeholder: String,
    /// A list of suggestions to autocomplete from
    pub suggestions: Option<&'a [&'a str]>,
    /// Show the input inline
    pub inline: bool,
    /// Whether to mask the input
    pub password: bool,
    /// Input entered by the user
    pub input: String,
    /// Colors/style of the input
    pub theme: &'a Theme,
    /// Validation function
    pub validation: fn(&str) -> Result<(), &str>,

    // Internal state
    cursor: usize,
    height: usize,
    term: Term,
    err: Option<String>,
    suggestion: Option<String>,
}

const CTRL_U: char = '\u{15}';
const CTRL_W: char = '\u{17}';

const ERR_MSG_HEIGHT: usize = 2;

impl<'a> Input<'a> {
    /// Creates a new input with the given title.
    pub fn new<S: Into<String>>(title: S) -> Self {
        Self {
            title: title.into(),
            description: String::new(),
            prompt: "> ".to_string(),
            placeholder: String::new(),
            suggestions: None,
            input: String::new(),
            inline: false,
            password: false,
            theme: &*theme::DEFAULT,
            validation: |_| Ok(()),

            // Internal state
            cursor: 0,
            height: 0,
            term: Term::stderr(),
            err: None,
            suggestion: None,
        }
    }

    /// Sets the description of the input.
    ///
    /// If the input is inline, it is displayed to the right of the title. Otherwise, it is displayed below the title.
    pub fn description(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }

    /// Sets the inline flag of the input.
    ///
    /// If true, the input is displayed inline with the title
    pub fn inline(mut self, inline: bool) -> Self {
        self.inline = inline;
        self
    }

    /// Sets the password flag of the input.
    ///
    /// If true, the input is masked with asterisks
    pub fn password(mut self, password: bool) -> Self {
        self.password = password;
        self
    }

    /// Sets the placeholder of the input.
    ///
    /// The placeholder is displayed in the input before the user enters any text
    pub fn placeholder(mut self, placeholder: &str) -> Self {
        self.placeholder = placeholder.to_string();
        self
    }

    /// Sets the suggestions of the input
    pub fn suggestions(mut self, suggestions: &'a [&'a str]) -> Self {
        self.suggestions = Some(suggestions);
        self
    }

    /// Sets the prompt of the input.
    ///
    /// The prompt is displayed after the title and description. If empty, the default prompt `> ` is displayed.
    pub fn prompt(mut self, prompt: &str) -> Self {
        self.prompt = prompt.to_string();
        self
    }

    /// Sets the theme of the input
    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Sets the validation for the input.
    ///
    /// If the input is valid, the Result is Ok(()). Otherwise, the Result is Err(&str).
    pub fn validation(mut self, validation: fn(&str) -> Result<(), &str>) -> Self {
        self.validation = validation;
        self
    }

    /// Displays the input to the user and returns the response
    ///
    /// This function will block until the user submits the input. If the user cancels the input,
    /// an error of type `io::ErrorKind::Interrupted` is returned.
    pub fn run(mut self) -> io::Result<String> {
        let ctrlc_handle = ctrlc::show_cursor_after_ctrlc(&self.term)?;

        self.term.hide_cursor()?;
        loop {
            self.clear()?;
            let output = self.render()?;

            self.height = output.lines().count() - 1;
            self.term.write_all(output.as_bytes())?;
            self.term.flush()?;
            self.set_cursor()?;

            let key = self.term.read_key()?;
            match key {
                Key::Char(CTRL_U) => self.handle_ctrl_u()?,
                Key::Char(CTRL_W) => self.handle_ctrl_w()?,
                Key::Char(c) => self.handle_key(c)?,
                Key::Backspace => self.handle_backspace()?,
                Key::ArrowLeft => self.handle_arrow_left()?,
                Key::ArrowRight => self.handle_arrow_right()?,
                Key::Home => self.handle_home()?,
                Key::End => self.handle_end()?,
                Key::Enter => {
                    self.clear_err()?;
                    self.validate()?;
                    if self.err.is_none() {
                        self.term.clear_to_end_of_screen()?;
                        self.term.show_cursor()?;
                        ctrlc_handle.close();
                        return self.handle_submit();
                    }
                }
                Key::Tab => self.handle_tab()?,
                Key::Escape => {
                    self.term.show_cursor()?;
                    ctrlc_handle.close();
                    return Err(io::Error::new(io::ErrorKind::Interrupted, "user cancelled"));
                }
                _ => {}
            }
            if key != Key::Enter {
                self.clear_err()?;
            }
            self.suggest()?;
        }
    }

    fn handle_key(&mut self, c: char) -> io::Result<()> {
        let idx = self.get_char_idx(&self.input, self.cursor);
        self.input.insert(idx, c);
        self.cursor += 1;
        Ok(())
    }

    fn handle_ctrl_u(&mut self) -> io::Result<()> {
        let idx = self.get_char_idx(&self.input, self.cursor);
        self.input.replace_range(..idx, "");
        self.cursor = 0;
        Ok(())
    }

    fn handle_ctrl_w(&mut self) -> io::Result<()> {
        // is masked, delete whole line to not reveal whitespace
        if self.password {
            self.handle_ctrl_u()?;
            return Ok(());
        }
        let idx = self.get_char_idx(&self.input, self.cursor);
        let slice = &self.input[0..idx];
        let offset = slice
            .trim_end_matches(|c: char| c.is_ascii_punctuation() || c.is_ascii_whitespace())
            .char_indices()
            .rfind(|&(_, x)| x.is_ascii_punctuation() || x.is_ascii_whitespace())
            .map(|(i, _)| i)
            .unwrap_or(0);
        let from = match offset > 0 {
            true => offset + 1,
            false => offset,
        };
        let len = measure_text_width(&self.input[from..idx]);

        self.input.replace_range(from..idx, "");

        match offset > 0 {
            true => self.cursor -= len,
            false => self.cursor = 0,
        }
        Ok(())
    }

    fn handle_backspace(&mut self) -> io::Result<()> {
        let chars_count = self.input.chars().count();
        if chars_count > 0 && self.cursor > 0 {
            let idx = self.get_char_idx(&self.input, self.cursor - 1);
            self.input.remove(idx);
        }
        if self.cursor > 0 {
            self.cursor -= 1;
        }
        Ok(())
    }

    fn handle_arrow_left(&mut self) -> io::Result<()> {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
        Ok(())
    }

    fn handle_arrow_right(&mut self) -> io::Result<()> {
        if self.cursor < self.input.chars().count() {
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

    fn handle_tab(&mut self) -> io::Result<()> {
        if self.suggestion.is_some() {
            self.input.push_str(self.suggestion.as_ref().unwrap());
            self.cursor = self.input.chars().count();
        }
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
            false => writeln!(out, "{}", self.title)?,
        }

        out.set_color(&self.theme.description)?;
        if !self.description.is_empty() {
            match self.inline {
                true => write!(out, " {}", self.description)?,
                false => writeln!(out, "{}", self.description)?,
            }
        }

        out.set_color(&self.theme.input_prompt)?;
        if !self.prompt.is_empty() {
            match self.inline {
                true => write!(out, "{}", self.prompt)?,
                false => write!(out, "{}", self.prompt)?,
            }
        }
        out.reset()?;

        self.render_input(&mut out)?;

        if self.err.is_some() {
            out.set_color(&self.theme.error_indicator)?;
            writeln!(out)?;
            writeln!(out)?;
            write!(out, "* {}", self.err.as_ref().unwrap())?;
            out.reset()?;
        }

        writeln!(out)?;
        out.reset()?;

        Ok(std::str::from_utf8(out.as_slice()).unwrap().to_string())
    }

    fn render_input(&mut self, out: &mut Buffer) -> io::Result<String> {
        let input = match self.password {
            true => self.input.chars().map(|_| '*').collect::<String>(),
            false => self.input.to_string(),
        };

        if !self.placeholder.is_empty() && self.input.is_empty() {
            out.set_color(
                &self
                    .theme
                    .real_cursor_color(Some(&self.theme.input_placeholder)),
            )?;
            write!(out, "{}", &self.placeholder[..1])?;
            if self.placeholder.len() > 1 {
                out.set_color(&self.theme.input_placeholder)?;
                write!(out, "{}", &self.placeholder[1..])?;
                out.reset()?;
            }
            return Ok(input);
        }

        let cursor_idx = self.get_char_idx(&input, self.cursor);
        write!(out, "{}", &input[..cursor_idx])?;

        if cursor_idx < input.len() {
            out.set_color(&self.theme.real_cursor_color(None))?;
            write!(out, "{}", &input[cursor_idx..cursor_idx + 1])?;
            out.reset()?;
        }
        if cursor_idx + 1 < input.len() {
            out.reset()?;
            write!(out, "{}", &input[cursor_idx + 1..])?;
        }

        if let Some(suggestion) = &self.suggestion {
            if !suggestion.is_empty() {
                if cursor_idx >= input.len() {
                    out.set_color(
                        &self
                            .theme
                            .real_cursor_color(Some(&self.theme.input_placeholder)),
                    )?;
                    write!(out, "{}", &suggestion[..1])?;
                    if suggestion.len() > 1 {
                        out.set_color(&self.theme.input_placeholder)?;
                        write!(out, "{}", &suggestion[1..])?;
                    }
                } else {
                    out.set_color(&self.theme.input_placeholder)?;
                    write!(out, "{suggestion}")?;
                }
                out.reset()?;
            } else if cursor_idx >= input.len() {
                out.set_color(&self.theme.real_cursor_color(None))?;
                write!(out, " ")?;
                out.reset()?;
            }
        } else if cursor_idx >= input.len() {
            out.set_color(&self.theme.real_cursor_color(None))?;
            write!(out, " ")?;
            out.reset()?;
        }

        Ok(input)
    }

    fn render_success(&mut self) -> io::Result<String> {
        let mut out = Buffer::ansi();
        out.set_color(&self.theme.title)?;
        write!(out, "{}", self.title)?;
        out.set_color(&self.theme.selected_option)?;
        writeln!(
            out,
            " {}",
            match self.password {
                true => (1..13).map(|_| '*').collect::<String>(),
                false => self.input.to_string(),
            }
        )?;
        out.reset()?;
        Ok(std::str::from_utf8(out.as_slice()).unwrap().to_string())
    }

    fn suggest(&mut self) -> io::Result<()> {
        if self.input.is_empty() {
            self.suggestion = None;
            return Ok(());
        }
        if let Some(suggestions) = &self.suggestions {
            self.suggestion = suggestions
                .iter()
                .find(|s| s.to_lowercase().starts_with(&self.input.to_lowercase()))
                .and_then(|s| {
                    let suggestion = s[self.input.len()..].to_string();
                    (!suggestion.is_empty()).then_some(suggestion)
                });
        }
        Ok(())
    }

    fn validate(&mut self) -> io::Result<()> {
        self.err = (self.validation)(&self.input)
            .map_err(|err| err.to_string())
            .err();
        Ok(())
    }

    fn get_char_idx(&self, input: &str, cursor: usize) -> usize {
        input
            .char_indices()
            .nth(cursor)
            .map(|(i, _)| i)
            .unwrap_or(input.len())
    }

    fn set_cursor(&mut self) -> io::Result<()> {
        // if we have a placeholder, move the cursor left to beginning of the input
        if !self.placeholder.is_empty() && self.input.is_empty() {
            self.term
                .move_cursor_left(self.placeholder.chars().count())?;
        } else {
            self.term.move_cursor_left(self.input.chars().count())?;
        }

        // if we have a suggestion, move the cursor left to end of the input
        if self.suggestion.is_some() {
            self.term
                .move_cursor_left(self.suggestion.as_ref().unwrap().chars().count())?;
        }

        // if there is an error, move the cursor up from error message and right to the input
        match self.err {
            Some(_) => {
                let err_count = self.err.as_ref().unwrap().chars().count();
                self.term.move_cursor_left(err_count + 2)?; // 2 for the error prefix
                self.term.move_cursor_up(ERR_MSG_HEIGHT)?;
                let mut offset = 0;
                if self.inline {
                    offset += self.title.chars().count();
                    offset += self.description.chars().count();
                    offset += self.prompt.chars().count();
                } else {
                    offset += self.prompt.chars().count();
                }
                offset += self.cursor;
                self.term.move_cursor_right(offset)?;
            }
            None => self.term.move_cursor_right(self.cursor)?,
        }
        Ok(())
    }

    fn clear_err(&mut self) -> io::Result<()> {
        if self.err.is_some() {
            self.err = None;
            self.term.move_cursor_down(ERR_MSG_HEIGHT)?;
        }
        Ok(())
    }

    fn clear(&mut self) -> io::Result<()> {
        self.term.clear_last_lines(self.height)?;
        self.term.clear_screen()?;
        self.height = 0;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::test::without_ansi;

    use super::*;

    const NON_EMPTY: fn(&str) -> Result<(), &str> = |s| {
        if s.is_empty() {
            return Err("Name cannot be empty");
        }
        Ok(())
    };

    #[test]
    fn test_render() {
        let mut input = Input::new("Title")
            .description("Description")
            .prompt("$ ")
            .placeholder("Placeholder");

        assert_eq!(
            "Title\nDescription\n$ Placeholder\n",
            without_ansi(input.render().unwrap().as_str())
        );
    }

    #[test]
    fn test_render_title() {
        let mut input = Input::new("Title");

        assert_eq!(
            "Title\n>  \n",
            without_ansi(input.render().unwrap().as_str())
        );
    }

    #[test]
    fn test_render_description() {
        let mut input = Input::new("Title").description("Description");

        assert_eq!(
            "Title\nDescription\n>  \n",
            without_ansi(input.render().unwrap().as_str())
        );
    }

    #[test]
    fn test_render_prompt() {
        let mut input = Input::new("Title").prompt("$ ");

        assert_eq!(
            "Title\n$  \n",
            without_ansi(input.render().unwrap().as_str())
        );
    }

    #[test]
    fn test_render_placeholder() {
        let mut input = Input::new("Title").placeholder("Placeholder");

        assert_eq!(
            "Title\n> Placeholder\n",
            without_ansi(input.render().unwrap().as_str())
        );
    }

    #[test]
    fn test_render_inline() {
        let mut input = Input::new("Title?")
            .description("Description.")
            .prompt("Prompt:")
            .placeholder("Placeholder")
            .inline(true);

        assert_eq!(
            "Title? Description.Prompt:Placeholder\n",
            without_ansi(input.render().unwrap().as_str())
        );
    }

    #[test]
    fn test_render_validation() {
        let mut input = Input::new("Title")
            .description("Description")
            .validation(NON_EMPTY);

        input.input = "".to_string();
        input.validate().unwrap();
        assert_eq!(
            "Title\nDescription\n>  \n\n* Name cannot be empty\n",
            without_ansi(input.render().unwrap().as_str())
        );

        input.input = "non empty".to_string();
        input.validate().unwrap();
        assert_eq!(
            "Title\nDescription\n> non empty\n",
            without_ansi(input.render().unwrap().as_str())
        );
    }

    #[test]
    fn test_render_validation_inline() {
        let mut input = Input::new("Title?")
            .description("Description.")
            .inline(true)
            .validation(NON_EMPTY);

        input.input = "".to_string();
        input.validate().unwrap();
        assert_eq!(
            "Title? Description.>  \n\n* Name cannot be empty\n",
            without_ansi(input.render().unwrap().as_str())
        );

        input.input = "non empty".to_string();
        input.validate().unwrap();
        assert_eq!(
            "Title? Description.> non empty\n",
            without_ansi(input.render().unwrap().as_str())
        );
    }
}
