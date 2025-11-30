use std::{
    char,
    io::{self, Write},
};

use console::{Key, Term, measure_text_width};
use termcolor::{Buffer, WriteColor};

use crate::ctrlc;
use crate::{Theme, theme};

/// Trait for implementing autocompletion features for text inputs.
///
/// The `Autocomplete` trait has two provided methods: `get_suggestions` and `get_completion`.
///
/// - `get_suggestions` is called whenever the user's text input is modified, returning a `Vec<String>`.
///   The `Vec<String>` is the list of suggestions that the prompt displays to the user according to their
///   text input. The user can navigate through the list and if they submit while highlighting one of these
///   suggestions, the suggestion is treated as the final answer.
///
/// - `get_completion` is called whenever the user presses the autocompletion hotkey (tab by default),
///   with the current text input and the text of the currently highlighted suggestion, if any, as parameters.
///   This method should return whether any text replacement (an autocompletion) should be made.
///
/// # Example
/// ```rust
/// use demand::{Input, Autocomplete};
///
/// #[derive(Clone)]
/// struct FileExtensionCompleter;
///
/// impl Autocomplete for FileExtensionCompleter {
///     fn get_suggestions(&mut self, input: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
///         let extensions = vec![".rs", ".toml", ".md", ".txt"];
///         Ok(extensions.iter()
///             .filter(|ext| ext.starts_with(input) || input.is_empty())
///             .map(|s| s.to_string())
///             .collect())
///     }
///
///     fn get_completion(
///         &mut self,
///         input: &str,
///         highlighted_suggestion: Option<&str>,
///     ) -> Result<Option<String>, Box<dyn std::error::Error>> {
///         if let Some(suggestion) = highlighted_suggestion {
///             Ok(Some(suggestion.to_string()))
///         } else {
///             Ok(None)
///         }
///     }
/// }
///
/// let input = Input::new("File extension:")
///     .autocomplete(FileExtensionCompleter)
///     .run();
/// ```
pub trait Autocomplete: AutocompleteClone {
    /// List of input suggestions to be displayed to the user upon typing the text input.
    ///
    /// If the user presses the autocompletion hotkey (tab as default) with a suggestion highlighted,
    /// the user's text input will be replaced by the content of the suggestion string.
    fn get_suggestions(&mut self, input: &str) -> Result<Vec<String>, Box<dyn std::error::Error>>;

    /// Standalone autocompletion that can be implemented based solely on the user's input.
    ///
    /// If the user presses the autocompletion hotkey (tab as default) and there are no suggestions
    /// highlighted, this function will be called in an attempt to autocomplete the user's input.
    ///
    /// If the returned value is of the Some variant, the text input will be replaced by the content
    /// of the string.
    fn get_completion(
        &mut self,
        input: &str,
        highlighted_suggestion: Option<&str>,
    ) -> Result<Option<String>, Box<dyn std::error::Error>>;
}

// Helper trait for cloning boxed Autocomplete trait objects
pub trait AutocompleteClone {
    fn clone_box(&self) -> Box<dyn Autocomplete>;
}

impl<T> AutocompleteClone for T
where
    T: 'static + Autocomplete + Clone,
{
    fn clone_box(&self) -> Box<dyn Autocomplete> {
        Box::new(self.clone())
    }
}

impl Clone for Box<dyn Autocomplete> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// No-op autocompleter that provides no suggestions or completions
#[derive(Clone)]
pub struct NoAutocompletion;

impl Autocomplete for NoAutocompletion {
    fn get_suggestions(&mut self, _input: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        Ok(Vec::new())
    }

    fn get_completion(
        &mut self,
        _input: &str,
        _highlighted_suggestion: Option<&str>,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        Ok(None)
    }
}

/// Simple function-based autocompleter
#[derive(Clone)]
pub struct FnAutocomplete<F>
where
    F: Fn(&str) -> Result<Vec<String>, Box<dyn std::error::Error>> + Clone + 'static,
{
    suggester: F,
}

impl<F> Autocomplete for FnAutocomplete<F>
where
    F: Fn(&str) -> Result<Vec<String>, Box<dyn std::error::Error>> + Clone + 'static,
{
    fn get_suggestions(&mut self, input: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        (self.suggester)(input)
    }

    fn get_completion(
        &mut self,
        _input: &str,
        highlighted_suggestion: Option<&str>,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        Ok(highlighted_suggestion.map(|s| s.to_string()))
    }
}

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
    /// A list of suggestions to autocomplete from (legacy, prefer using autocomplete())
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
    pub validation: Box<dyn InputValidator>,

    // Internal state
    cursor: usize,
    height: usize,
    term: Term,
    err: Option<String>,
    suggestion: Option<String>,
    autocompleter: Box<dyn Autocomplete>,
    suggestions_list: Vec<String>,
    selected_suggestion_idx: Option<usize>,
    show_suggestions: bool,
    max_suggestions_display: usize,
    input_line_offset: usize,
    suggestions_scroll_offset: usize,
}

const CTRL_U: char = '\u{15}';
const CTRL_W: char = '\u{17}';

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
            validation: Box::new(NoValidation),

            // Internal state
            cursor: 0,
            height: 0,
            term: Term::stderr(),
            err: None,
            suggestion: None,
            autocompleter: Box::new(NoAutocompletion),
            suggestions_list: Vec::new(),
            selected_suggestion_idx: None,
            show_suggestions: false,
            max_suggestions_display: 5,
            input_line_offset: 0,
            suggestions_scroll_offset: 0,
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

    /// Sets the suggestions of the input (legacy method)
    pub fn suggestions(mut self, suggestions: &'a [&'a str]) -> Self {
        self.suggestions = Some(suggestions);
        self
    }

    /// Sets a custom autocompleter for the input.
    ///
    /// The autocompleter will be used to generate suggestions and completions based on user input.
    pub fn autocomplete<A: Autocomplete + 'static>(mut self, autocompleter: A) -> Self {
        self.autocompleter = Box::new(autocompleter);
        self
    }

    /// Sets a function-based autocompleter for the input.
    ///
    /// The function receives the current input and should return a list of suggestions.
    pub fn autocomplete_fn<F>(mut self, suggester: F) -> Self
    where
        F: Fn(&str) -> Result<Vec<String>, Box<dyn std::error::Error>> + Clone + 'static,
    {
        self.autocompleter = Box::new(FnAutocomplete { suggester });
        self
    }

    /// Sets the maximum number of suggestions to display
    pub fn max_suggestions_display(mut self, max: usize) -> Self {
        self.max_suggestions_display = max;
        self
    }

    /// Sets the prompt of the input.
    ///
    /// The prompt is displayed after the title and description. If empty, the default prompt `> ` is displayed.
    pub fn prompt(mut self, prompt: &str) -> Self {
        self.prompt = prompt.to_string();
        self
    }

    /// Sets the default value of the input.
    pub fn default_value(mut self, default_value: impl Into<String>) -> Self {
        self.input = default_value.into();
        self.cursor += self.input.chars().count(); // move cursor to the end of the text
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
    pub fn validation(self, validation: fn(&str) -> Result<(), &str>) -> Self {
        self.validator(FnValidator(validation))
    }

    /// Sets the validator for the input.
    ///
    /// This is similar to the [Input::validation] method, but it's more flexible.
    /// See [InputValidator] for examples
    pub fn validator(mut self, validation: impl InputValidator + 'static) -> Self {
        self.validation = Box::new(validation);
        self
    }

    /// Displays the input to the user and returns the response
    ///
    /// This function will block until the user submits the input. If the user cancels the input,
    /// an error of type `io::ErrorKind::Interrupted` is returned.
    pub fn run(mut self) -> io::Result<String> {
        // If not a TTY (e.g., piped input or non-interactive environment),
        // write a simple prompt and read from stdin
        if !crate::tty::is_tty() {
            let prompt = if !self.prompt.is_empty() {
                &self.prompt
            } else {
                "> "
            };

            crate::tty::write_prompt(&self.title, &self.description, prompt)?;
            self.input = crate::tty::read_line()?;
            self.validate()?;

            if self.err.is_some() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    self.err.unwrap(),
                ));
            }
            return Ok(self.input);
        }

        let ctrlc_handle = ctrlc::show_cursor_after_ctrlc(&self.term)?;

        self.term.hide_cursor()?;
        self.update_suggestions()?;

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
                Key::ArrowUp => self.handle_arrow_up()?,
                Key::ArrowDown => self.handle_arrow_down()?,
                Key::Home => self.handle_home()?,
                Key::End => self.handle_end()?,
                Key::Enter => {
                    self.clear_err()?;
                    self.validate()?;
                    if self.err.is_none() {
                        self.reset_cursor_to_end()?;
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
        }
    }

    fn handle_key(&mut self, c: char) -> io::Result<()> {
        let idx = self.get_char_idx(&self.input, self.cursor);
        self.input.insert(idx, c);
        self.cursor += 1;
        self.update_suggestions()?;
        Ok(())
    }

    fn handle_ctrl_u(&mut self) -> io::Result<()> {
        let idx = self.get_char_idx(&self.input, self.cursor);
        self.input.replace_range(..idx, "");
        self.cursor = 0;
        self.update_suggestions()?;
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
        self.update_suggestions()?;
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
        self.update_suggestions()?;
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

    fn handle_arrow_up(&mut self) -> io::Result<()> {
        if self.show_suggestions && !self.suggestions_list.is_empty() {
            self.selected_suggestion_idx = match self.selected_suggestion_idx {
                Some(idx) if idx > 0 => Some(idx - 1),
                Some(_) => Some(self.suggestions_list.len() - 1),
                None => Some(self.suggestions_list.len() - 1),
            };
            self.update_scroll_offset();
        }
        Ok(())
    }

    fn handle_arrow_down(&mut self) -> io::Result<()> {
        if self.show_suggestions && !self.suggestions_list.is_empty() {
            self.selected_suggestion_idx = match self.selected_suggestion_idx {
                Some(idx) if idx < self.suggestions_list.len() - 1 => Some(idx + 1),
                Some(_) => Some(0),
                None => Some(0),
            };
            self.update_scroll_offset();
        }
        Ok(())
    }

    fn update_scroll_offset(&mut self) {
        if let Some(selected_idx) = self.selected_suggestion_idx {
            if selected_idx >= self.suggestions_scroll_offset + self.max_suggestions_display {
                self.suggestions_scroll_offset = selected_idx - self.max_suggestions_display + 1;
            } else if selected_idx < self.suggestions_scroll_offset {
                self.suggestions_scroll_offset = selected_idx;
            }
        }
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
        let highlighted = self
            .selected_suggestion_idx
            .and_then(|idx| self.suggestions_list.get(idx))
            .map(|s| s.as_str());

        match self.autocompleter.get_completion(&self.input, highlighted) {
            Ok(Some(completion)) => {
                self.input = completion;
                self.cursor = self.input.chars().count();
                self.show_suggestions = false;
                self.selected_suggestion_idx = None;
                self.suggestions_scroll_offset = 0;
                self.update_suggestions()?;
            }
            Ok(None) => {
                if self.suggestion.is_some() {
                    self.input.push_str(self.suggestion.as_ref().unwrap());
                    self.cursor = self.input.chars().count();
                    self.update_suggestions()?;
                }
            }
            Err(_) => {}
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
            true => {
                write!(out, "{}", self.title)?;
            }
            false => {
                writeln!(out, "{}", self.title)?;
            }
        }

        out.set_color(&self.theme.description)?;
        if !self.description.is_empty() {
            match self.inline {
                true => write!(out, " {}", self.description)?,
                false => {
                    writeln!(out, "{}", self.description)?;
                }
            }
        }

        out.set_color(&self.theme.input_prompt)?;
        if !self.prompt.is_empty() {
            write!(out, "{}", self.prompt)?;
        }
        out.reset()?;

        self.input_line_offset = if self.inline {
            0
        } else {
            let mut offset = 1;
            if !self.description.is_empty() {
                offset += 1;
            }
            offset
        };

        self.render_input(&mut out)?;
        writeln!(out)?;

        if self.show_suggestions && !self.suggestions_list.is_empty() {
            let end_idx = (self.suggestions_scroll_offset + self.max_suggestions_display)
                .min(self.suggestions_list.len());

            for (i, suggestion) in self.suggestions_list[self.suggestions_scroll_offset..end_idx]
                .iter()
                .enumerate()
            {
                let actual_idx = i + self.suggestions_scroll_offset;
                if Some(actual_idx) == self.selected_suggestion_idx {
                    out.set_color(&self.theme.selected_option)?;
                    write!(out, " → {}", suggestion)?;
                } else {
                    out.set_color(&self.theme.unselected_option)?;
                    write!(out, "   {}", suggestion)?;
                }
                writeln!(out)?;
            }

            if self.suggestions_list.len() > self.max_suggestions_display {
                out.set_color(&self.theme.description)?;
                let remaining = self.suggestions_list.len() - end_idx;
                if remaining > 0 {
                    writeln!(out, "   ↓ {} more", remaining)?;
                } else if self.suggestions_scroll_offset > 0 {
                    writeln!(out, "   ↑ scroll for more")?;
                }
            }
            out.reset()?;
        }

        if self.err.is_some() {
            out.set_color(&self.theme.error_indicator)?;
            writeln!(out)?;
            writeln!(out)?;
            write!(out, "✗ {}", self.err.as_ref().unwrap())?;
            out.reset()?;
        }

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
            if !suggestion.is_empty() && !self.show_suggestions {
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

    fn update_suggestions(&mut self) -> io::Result<()> {
        match self.autocompleter.get_suggestions(&self.input) {
            Ok(suggestions) => {
                if !suggestions.is_empty() {
                    self.suggestions_list = suggestions;
                    self.show_suggestions = true;
                    if self.selected_suggestion_idx.is_none() {
                        self.selected_suggestion_idx = Some(0);
                    } else if let Some(idx) = self.selected_suggestion_idx
                        && idx >= self.suggestions_list.len()
                    {
                        self.selected_suggestion_idx = Some(0);
                        self.suggestions_scroll_offset = 0;
                    }
                } else {
                    self.show_suggestions = false;
                    self.selected_suggestion_idx = None;
                    self.suggestions_list.clear();
                    self.suggestions_scroll_offset = 0;
                }
            }
            Err(_) => {
                self.show_suggestions = false;
                self.selected_suggestion_idx = None;
                self.suggestions_list.clear();
                self.suggestions_scroll_offset = 0;
            }
        }

        self.suggest()?;
        Ok(())
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
        self.err = self.validation.check(&self.input).err();
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
        let lines_below_input = self.height - self.input_line_offset;
        if lines_below_input > 0 {
            self.term.move_cursor_up(lines_below_input)?;
        }

        self.term.move_cursor_left(usize::MAX)?;

        let mut offset = 0;
        if self.inline {
            offset += self.title.chars().count();
            if !self.description.is_empty() {
                offset += 1;
                offset += self.description.chars().count();
            }
        }
        offset += self.prompt.chars().count();
        offset += self.cursor;

        self.term.move_cursor_right(offset)?;

        Ok(())
    }

    fn reset_cursor_to_end(&mut self) -> io::Result<()> {
        let lines_below_input = self.height - self.input_line_offset;
        if lines_below_input > 0 {
            self.term.move_cursor_down(lines_below_input)?;
        }
        Ok(())
    }

    fn clear_err(&mut self) -> io::Result<()> {
        if self.err.is_some() {
            self.err = None;
        }
        Ok(())
    }

    fn clear(&mut self) -> io::Result<()> {
        if self.height > 0 {
            self.reset_cursor_to_end()?;
            self.term.clear_last_lines(self.height)?;
        }
        self.height = 0;
        Ok(())
    }
}

/// Input validator trait
///
/// ## Examples
///
/// Simple validation with a function pointer
///
/// ```rust
/// use demand::Input;
///
/// fn not_empty(s: &str) -> Result<(), &'static str> {
///      if s.is_empty() {
///          return Err("Name cannot be empty");
///      }
///      Ok(())
/// }
///
/// let input = Input::new("What's your name?")
///     .validation(not_empty);
/// // input.run() would block waiting for user input
/// ```
///
/// Dynamic validation
///
/// ```rust
/// use demand::{Input, InputValidator};
///
/// struct NameValidation {
///     max_length: usize,
/// }
///
/// impl InputValidator for NameValidation {
///     fn check(&self, input: &str) -> Result<(), String> {
///         if input.len() > self.max_length {
///             return Err(format!(
///                 "Name must be at most {} characters, got {}",
///                 self.max_length,
///                 input.len()
///             ));
///         }
///         Ok(())
///     }
/// }
///
/// let input = Input::new("What's your name?")
///     .validator(NameValidation { max_length: 50 });
/// // input.run() would block waiting for user input
/// ```
pub trait InputValidator {
    fn check(&self, input: &str) -> Result<(), String>;
}

/// No validation
///
/// Every input is accepted
pub struct NoValidation;
impl InputValidator for NoValidation {
    fn check(&self, _input: &str) -> Result<(), String> {
        Ok(())
    }
}

pub struct FnValidator(fn(&str) -> Result<(), &str>);
impl InputValidator for FnValidator {
    fn check(&self, input: &str) -> Result<(), String> {
        (self.0)(input).map_err(str::to_string)
    }
}

impl<F, Err> InputValidator for F
where
    F: Fn(&str) -> Result<(), Err>,
    Err: ToString,
{
    fn check(&self, input: &str) -> Result<(), String> {
        self(input).map_err(|err| err.to_string())
    }
}

impl InputValidator for fn(&str) -> Result<(), &str> {
    fn check(&self, input: &str) -> Result<(), String> {
        self(input).map_err(str::to_string)
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
            "Title\nDescription\n>  \n\n\n✗ Name cannot be empty",
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
            "Title? Description.>  \n\n\n✗ Name cannot be empty",
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
