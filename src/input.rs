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
    /// Whether to mask the input while typing and after submit
    pub password: bool,
    /// Whether to mask the input only after submit (visible while typing)
    pub mask_on_submit: bool,
    /// Input entered by the user
    pub input: String,
    /// Colors/style of the input
    pub theme: &'a Theme,
    /// Validation function
    pub validation: Box<dyn InputValidator>,

    // Internal state
    cursor: usize,
    frame: crate::frame::Frame,
    term: Term,
    err: Option<String>,
    suggestion: Option<String>,
    autocompleter: Box<dyn Autocomplete>,
    suggestions_list: Vec<String>,
    selected_suggestion_idx: Option<usize>,
    show_suggestions: bool,
    max_suggestions_display: usize,
    input_line_offset: usize,
    /// The (row, column) of the caret within the input row, which can
    /// wrap onto several rows.
    caret: (usize, usize),
    /// Physical rows the input row wraps into.
    input_rows: usize,
    /// Rows between where `set_cursor` parked the cursor and the end of
    /// the frame, so `reset_cursor_to_end` can walk back down. Recorded
    /// rather than recomputed: by the time the cursor goes back down, the
    /// next frame has been rendered and the layout fields describe it.
    rows_below_caret: usize,
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
            mask_on_submit: false,
            theme: &*theme::DEFAULT,
            validation: Box::new(NoValidation),

            // Internal state
            cursor: 0,
            frame: Default::default(),
            term: Term::stderr(),
            err: None,
            suggestion: None,
            autocompleter: Box::new(NoAutocompletion),
            suggestions_list: Vec::new(),
            selected_suggestion_idx: None,
            show_suggestions: false,
            max_suggestions_display: 5,
            input_line_offset: 0,
            caret: (0, 0),
            input_rows: 1,
            rows_below_caret: 0,
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
    /// If true, the input is masked with asterisks while typing and after submit
    pub fn password(mut self, password: bool) -> Self {
        self.password = password;
        self
    }

    /// Sets the mask_on_submit flag of the input.
    ///
    /// If true, the input is visible while typing but masked with asterisks after submit.
    /// Note: has no effect when `password` is also `true` (password masking takes precedence).
    pub fn mask_on_submit(mut self, mask_on_submit: bool) -> Self {
        self.mask_on_submit = mask_on_submit;
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

            if let Some(err) = self.err {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, err));
            }
            return Ok(self.input);
        }

        let ctrlc_handle = ctrlc::show_cursor_after_ctrlc(&self.term)?;

        self.term.hide_cursor()?;
        self.update_suggestions()?;

        loop {
            let term = self.term.clone();
            crate::synchronized_output::run(&term, || self.draw())?;

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
                    self.clear()?;
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
                if let Some(suggestion) = &self.suggestion {
                    self.input.push_str(suggestion);
                    self.cursor = self.input.chars().count();
                    self.update_suggestions()?;
                }
            }
            Err(_) => {}
        }
        Ok(())
    }

    fn handle_submit(mut self) -> io::Result<String> {
        let term = self.term.clone();
        crate::synchronized_output::run(&term, || {
            self.clear()?;
            let output = self.render_success()?;
            self.term.write_all(output.as_bytes())
        })?;
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

        // Rows above the input row. Everything written so far is the
        // header plus the prompt, and the prompt shares the input's row —
        // which is exactly what `rendered_height` excludes, so it gives
        // the header's height directly. Counting rows rather than lines
        // matters because `self.height` is in rows: subtracting a logical
        // line count from it would put the caret on the wrong row as soon
        // as a title or description wrapped. Inline mode writes no
        // newline at all and correctly comes out as 0.
        let width = self.term.size().1 as usize;
        let header = std::str::from_utf8(out.as_slice()).unwrap_or_default();
        self.input_line_offset = crate::height::rendered_height(header, width);
        let row_start = header.rfind('\n').map_or(0, |i| i + 1);
        let prefix = header[row_start..].to_string();

        let input = self.render_input(&mut out)?;
        // Lay the row out as the terminal will: an inline title, the
        // prompt and the input all share it, wide characters take two
        // columns, and one that doesn't fit at the edge moves to the next
        // row. The caret is where the char under it starts — or where the
        // cursor block after the input starts. A zero-width char adds no
        // columns, so the caret sits right where it would print.
        let written = std::str::from_utf8(out.as_slice()).unwrap_or_default();
        self.input_rows = crate::height::rows_for(&written[row_start..], width);
        let caret_idx = self.get_char_idx(&input, self.cursor);
        let under_caret = input[caret_idx..].chars().next().unwrap_or(' ');
        let through_caret = format!("{prefix}{}{under_caret}", &input[..caret_idx]);
        let (row, end) = crate::height::cursor_after(&through_caret, width);
        let start = end.saturating_sub(console::measure_text_width(&under_caret.to_string()));
        self.caret = if row < self.input_rows {
            (row, start)
        } else {
            // Nothing is drawn after a caret at the very edge of the last
            // row, so the terminal never wraps onto the row it'd be on.
            (self.input_rows - 1, width.saturating_sub(1))
        };
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

        if let Some(err) = &self.err {
            out.set_color(&self.theme.error_indicator)?;
            writeln!(out)?;
            writeln!(out)?;
            write!(out, "✗ {err}")?;
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
            let split = first_char_len(&self.placeholder);
            write!(out, "{}", &self.placeholder[..split])?;
            if self.placeholder.len() > split {
                out.set_color(&self.theme.input_placeholder)?;
                write!(out, "{}", &self.placeholder[split..])?;
                out.reset()?;
            }
            return Ok(input);
        }

        let cursor_idx = self.get_char_idx(&input, self.cursor);
        write!(out, "{}", &input[..cursor_idx])?;

        let after_cursor = cursor_idx + first_char_len(&input[cursor_idx..]);
        if cursor_idx < input.len() {
            out.set_color(&self.theme.real_cursor_color(None))?;
            write!(out, "{}", &input[cursor_idx..after_cursor])?;
            out.reset()?;
        }
        if after_cursor < input.len() {
            out.reset()?;
            write!(out, "{}", &input[after_cursor..])?;
        }

        if let Some(suggestion) = &self.suggestion {
            if !suggestion.is_empty() && !self.show_suggestions {
                if cursor_idx >= input.len() {
                    out.set_color(
                        &self
                            .theme
                            .real_cursor_color(Some(&self.theme.input_placeholder)),
                    )?;
                    let split = first_char_len(suggestion);
                    write!(out, "{}", &suggestion[..split])?;
                    if suggestion.len() > split {
                        out.set_color(&self.theme.input_placeholder)?;
                        write!(out, "{}", &suggestion[split..])?;
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
            if self.password {
                "*".repeat(12)
            } else if self.mask_on_submit {
                "*".repeat(self.input.chars().count())
            } else {
                self.input.to_string()
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

    /// Park the terminal cursor on the caret. When the input row wraps,
    /// the caret can be on any of its rows: cursor movement stops at the
    /// right edge rather than wrapping, so the row has to be picked
    /// explicitly and only the remainder moved across.
    fn set_cursor(&mut self) -> io::Result<()> {
        let (row, col) = self.caret;
        let rows_up = self.frame.height(&self.term) - self.input_line_offset - row;
        self.rows_below_caret = rows_up;
        if rows_up > 0 {
            self.term.move_cursor_up(rows_up)?;
        }
        self.term.move_cursor_left(usize::MAX)?;
        if col > 0 {
            self.term.move_cursor_right(col)?;
        }
        Ok(())
    }

    fn reset_cursor_to_end(&mut self) -> io::Result<()> {
        if self.rows_below_caret > 0 {
            self.term.move_cursor_down(self.rows_below_caret)?;
        }
        self.rows_below_caret = 0;
        Ok(())
    }

    /// Draw a fresh frame over the previous one and park the cursor on the
    /// caret. A frame identical to the one on screen leaves everything,
    /// cursor included, where it is.
    fn draw(&mut self) -> io::Result<()> {
        let output = self.render()?;
        if self.frame.is_current(&output) {
            return Ok(());
        }
        self.reset_cursor_to_end()?;
        self.frame.update(&self.term, output)?;
        self.set_cursor()
    }

    fn clear_err(&mut self) -> io::Result<()> {
        if self.err.is_some() {
            self.err = None;
        }
        Ok(())
    }

    fn clear(&mut self) -> io::Result<()> {
        if !self.frame.is_empty() {
            self.reset_cursor_to_end()?;
            self.frame.clear(&self.term)?;
        }
        Ok(())
    }
}

/// Byte length of the first char of `s`, or 0 if it's empty. Slicing off
/// one *byte* instead panics as soon as that char is multi-byte.
fn first_char_len(s: &str) -> usize {
    s.chars().next().map_or(0, char::len_utf8)
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
    #[cfg(unix)]
    use crate::test::{Parser, capture_term, replay, snapshot};

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

    #[test]
    fn test_render_success_mask_on_submit() {
        let mut input = Input::new("PIN").mask_on_submit(true);
        input.input = "1234".to_string();
        assert_eq!(
            "PIN ****\n",
            without_ansi(input.render_success().unwrap().as_str())
        );
    }

    #[test]
    fn test_render_success_mask_on_submit_empty() {
        let mut input = Input::new("PIN").mask_on_submit(true);
        input.input = "".to_string();
        assert_eq!(
            "PIN \n",
            without_ansi(input.render_success().unwrap().as_str())
        );
    }

    #[test]
    fn test_render_success_password() {
        let mut input = Input::new("Password").password(true);
        input.input = "short".to_string();
        assert_eq!(
            "Password ************\n",
            without_ansi(input.render_success().unwrap().as_str())
        );
    }

    /// `input_line_offset` is subtracted from `self.height`, which counts
    /// physical rows — so it has to count rows too. A title that wraps
    /// occupies two of them, and counting it as one logical line would
    /// put the caret a row above where the input actually is.
    #[test]
    fn input_line_offset_counts_rows_not_lines() {
        let mut input = Input::new("t".repeat(200)).description("d");
        input.render().unwrap();
        let width = input.term.size().1 as usize;
        let title_rows = 200_usize.div_ceil(width);
        assert!(title_rows > 1, "test needs a title wider than the terminal");
        assert_eq!(input.input_line_offset, title_rows + 1);
    }

    /// Inline mode writes no newline before the input, so nothing sits
    /// above it.
    #[test]
    fn inline_input_has_no_rows_above_it() {
        let mut input = Input::new("Name").inline(true);
        input.render().unwrap();
        assert_eq!(input.input_line_offset, 0);
    }

    /// The cursor highlight used to slice one byte off the input, which
    /// panics when the char under the caret is multi-byte.
    #[test]
    fn renders_multibyte_char_under_the_caret() {
        let mut input = Input::new("Name").placeholder("éé");
        input.render().unwrap();
        input.input = "日本語".to_string();
        input.cursor = 1;
        let rendered = input.render().unwrap();
        assert!(without_ansi(&rendered).contains("> 日本語"));
    }

    /// Wide chars take two columns, so the caret has to be measured, not
    /// counted.
    #[test]
    fn caret_on_a_zero_width_char_stays_in_the_input() {
        let mut input = Input::new("Name");
        input.input = "\u{200b}abc".to_string();
        input.cursor = 0;
        input.render().unwrap();
        assert_eq!(input.caret, (0, "> ".len()));
    }

    #[test]
    fn caret_measures_wide_chars() {
        let mut input = Input::new("Name");
        input.input = "日本語".to_string();
        input.cursor = 2;
        input.render().unwrap();
        assert_eq!(input.caret, (0, "> ".len() + 4));
    }

    /// A wide char under the caret that doesn't fit at the end of a row is
    /// drawn at the start of the next one, and so is the caret. Checked
    /// against the terminal emulator, which wraps it for real.
    #[cfg(unix)]
    #[test]
    fn caret_follows_a_wide_char_wrapped_at_the_edge() {
        let (term, buf) = capture_term();
        let mut input = Input::new("Name");
        input.term = term;
        let width = input.term.size().1 as usize;
        // "> " plus enough `x` to leave one column before the edge.
        input.input = format!("{}日本", "x".repeat(width - 3));
        input.cursor = width - 3;
        input.draw().unwrap();
        assert_eq!(input.caret, (1, 0));

        let mut parser = Parser::new(24, width as u16, 0);
        replay(&mut parser, &snapshot(&buf));
        // Title on row 0, the input starts on row 1 and wraps onto row 2.
        assert_eq!(parser.screen().cursor_position(), (2, 0));
    }

    /// Regression for jdx/demand#7 in `Input`: an input row wider than the
    /// terminal wraps. Cursor movement stops at the right edge, so moving
    /// right by the caret's char offset used to pin the cursor to the edge
    /// of the input's first row, and the next clear started from there.
    #[cfg(unix)]
    #[test]
    fn a_wrapped_input_row_keeps_the_caret_and_redraws_cleanly() {
        let (term, buf) = capture_term();
        let mut input = Input::new("Command").description("run what?");
        input.term = term;
        let width = input.term.size().1 as usize;
        input.input = "x".repeat(width + 10);
        input.cursor = input.input.chars().count();

        for _ in 0..3 {
            input.draw().unwrap();
            input.handle_arrow_left().unwrap();
        }

        let mut parser = Parser::new(24, width as u16, 0);
        replay(&mut parser, &snapshot(&buf));
        let screen = parser.screen().contents();
        assert_eq!(
            screen.matches("Command").count(),
            1,
            "prompt drawn more than once:\n{screen}"
        );
        // Title and description take rows 0 and 1; the input starts on
        // row 2 and wraps onto row 3. After two left presses the caret is
        // 8 columns into the wrapped row ("> " + width + 10 - 2).
        assert_eq!(parser.screen().cursor_position(), (3, 10));
    }
    /// jdx/demand#123 for `Input`, where the cursor is parked on the caret
    /// rather than at the end of the frame: typing and moving through a
    /// wrapped row has to leave the screen, and the cursor, exactly where
    /// a fresh draw of the final state would.
    #[cfg(unix)]
    #[test]
    fn incremental_redraws_match_a_full_redraw() {
        let (term, buf) = capture_term();
        let mut input = Input::new("Command").description("run what?");
        input.term = term;
        let width = input.term.size().1;
        input.input = "x".repeat(width as usize - 3);
        input.cursor = input.input.chars().count();

        input.draw().unwrap();
        for c in "yyyy".chars() {
            input.handle_key(c).unwrap();
            input.draw().unwrap();
        }
        input.handle_arrow_left().unwrap();
        input.draw().unwrap();
        input.handle_backspace().unwrap();
        input.draw().unwrap();
        // A no-op key: the frame doesn't change and nothing is written.
        let before = snapshot(&buf).len();
        input.draw().unwrap();
        assert_eq!(snapshot(&buf).len(), before);

        let (fresh_term, fresh_buf) = capture_term();
        let mut fresh = Input::new("Command").description("run what?");
        fresh.input = input.input.clone();
        fresh.cursor = input.cursor;
        fresh.term = fresh_term;
        fresh.draw().unwrap();

        let mut patched = Parser::new(24, width, 0);
        replay(&mut patched, &snapshot(&buf));
        let mut full = Parser::new(24, width, 0);
        replay(&mut full, &snapshot(&fresh_buf));
        assert_eq!(patched.screen().contents(), full.screen().contents());
        assert_eq!(
            patched.screen().cursor_position(),
            full.screen().cursor_position()
        );
    }
}
