use std::{
    char,
    io::{self, Write},
};

use console::{Key, Term};
use termcolor::{Buffer, WriteColor};

use crate::Theme;

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
pub struct Input {
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
    pub theme: Theme,

    height: usize,
    term: Term,
}

impl Input {
    /// Creates a new input with the given title.
    pub fn new<S: Into<String>>(title: S) -> Self {
        Self {
            title: title.into(),
            description: String::new(),
            placeholder: String::new(),
            prompt: String::new(),
            input: String::new(),
            inline: false,
            password: false,
            theme: Theme::default(),
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
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Displays the input to the user and returns the response
    pub fn run(mut self) -> io::Result<String> {
        let output = self.render()?;
        self.height = output.lines().count() - 1;
        self.term.write_all(output.as_bytes())?;
        self.term.flush()?;
        self.term.show_cursor()?;
        self.render_placeholder()?;
        loop {
            match self.term.read_key()? {
                Key::Char(c) => self.handle_key(c)?,
                Key::Backspace => self.handle_backspace()?,
                Key::Enter => {
                    return self.handle_submit();
                }
                _ => {}
            }

            let chars_count = self.input.chars().count();
            if chars_count > 0 {
                self.clear_placeholder()?;
                self.term.clear_chars(chars_count - 1)?;
            } else {
                self.render_placeholder()?;
            }

            let input = self.render_input()?;
            self.term.write_all(input.as_bytes())?;
            self.term.flush()?;
        }
    }

    fn handle_key(&mut self, c: char) -> io::Result<()> {
        self.input.push(c);
        Ok(())
    }

    fn handle_backspace(&mut self) -> io::Result<()> {
        let chars_count = self.input.chars().count();
        if chars_count > 1 {
            self.term.move_cursor_left(1)?;
        }
        if chars_count > 0 {
            self.input.pop();
            self.term.clear_chars(1)?;
        }
        Ok(())
    }

    fn handle_submit(mut self) -> io::Result<String> {
        self.clear()?;
        let output = self.render_success()?;
        self.term.write_all(output.as_bytes())?;
        Ok(self.input)
    }

    fn render(&self) -> io::Result<String> {
        let mut out = Buffer::ansi();

        out.set_color(&self.theme.title)?;
        match self.inline {
            true => write!(out, "{}", self.title)?,
            false => writeln!(out, "{}", self.title)?,
        }

        out.set_color(&self.theme.description)?;
        if !self.description.is_empty() {
            match self.inline {
                true => write!(out, "{}", self.description)?,
                false => writeln!(out, "{}", self.description)?,
            }
        }

        out.set_color(&self.theme.input_prompt)?;
        if !self.prompt.is_empty() {
            match self.inline {
                true => write!(out, "> ")?,
                false => write!(out, "{}", self.prompt)?,
            }
        }

        out.reset()?;
        Ok(std::str::from_utf8(out.as_slice()).unwrap().to_string())
    }

    fn render_placeholder(&mut self) -> io::Result<()> {
        if !self.placeholder.is_empty() {
            let mut out = Buffer::ansi();
            out.set_color(&self.theme.input_placeholder)?;
            out.write_all(self.placeholder.as_bytes())?;
            self.term.write_all(out.as_slice())?;
            self.term
                .move_cursor_left(self.placeholder.chars().count())?;
            self.term.flush()?;
            out.reset()?;
        }
        Ok(())
    }

    fn clear_placeholder(&mut self) -> io::Result<()> {
        if !self.placeholder.is_empty() {
            let placeholder_count = self.placeholder.chars().count();
            self.term.move_cursor_right(placeholder_count)?;
            self.term.clear_chars(placeholder_count)?;
        }
        Ok(())
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
        write!(out, "{}", self.title)?;
        out.set_color(&self.theme.selected_option)?;
        writeln!(out, " {}", &self.render_input()?.to_string())?;
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
