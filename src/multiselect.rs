use std::collections::HashSet;
use std::io;
use std::io::Write;

use console::{Alignment, Key, Term};
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use itertools::Itertools;
use termcolor::{Buffer, WriteColor};

use crate::theme::Theme;
use crate::{DemandOption, ctrlc, theme};

/// Select multiple options from a list
///
/// # Example
/// ```rust
/// use demand::{DemandOption, MultiSelect};
///
/// let multiselect = MultiSelect::new("Toppings")
///   .description("Select your toppings")
///   .min(1)
///   .max(4)
///   .filterable(true)
///   .option(DemandOption::new("Lettuce").selected(true))
///   .option(DemandOption::new("Tomatoes").selected(true))
///   .option(DemandOption::new("Charm Sauce"))
///   .option(DemandOption::new("Jalapenos").label("Jalapeños"))
///   .option(DemandOption::new("Cheese"))
///   .option(DemandOption::new("Vegan Cheese"))
///   .option(DemandOption::new("Nutella"));
/// let toppings = match multiselect.run() {
///   Ok(toppings) => toppings,
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
pub struct MultiSelect<'a, T> {
    /// The title of the selector
    pub title: String,
    /// The colors/style of the selector
    pub theme: &'a Theme,
    /// A description to display after the title
    pub description: String,
    /// The options which can be selected
    pub options: Vec<DemandOption<T>>,
    /// The minimum number of options which must be selected
    pub min: usize,
    /// The maximum number of options which can be selected
    pub max: usize,
    /// Whether the selector can be filtered with a query
    pub filterable: bool,
    /// Whether the selector is currently being filtered
    pub filtering: bool,
    /// A filter query to preset when `filtering` is true
    pub filter: String,
    /// The key used to toggle selection (default: Space)
    pub toggle_key: Key,

    err: Option<String>,
    cursor_x: usize,
    cursor_y: usize,
    cursor: usize,
    last_line_count: usize,
    term: Term,
    pages: usize,
    cur_page: usize,
    capacity: usize,
    fuzzy_matcher: SkimMatcherV2,
}

impl<'a, T> MultiSelect<'a, T> {
    /// Create a new multi select with the given title
    pub fn new<S: Into<String>>(title: S) -> Self {
        let mut ms = MultiSelect {
            title: title.into(),
            description: String::new(),
            options: vec![],
            min: 0,
            max: usize::MAX,
            filterable: false,
            theme: &theme::DEFAULT,
            cursor_x: 0,
            cursor_y: 0,
            err: None,
            cursor: 0,
            last_line_count: 0,
            term: Term::stderr(),
            filter: String::new(),
            filtering: false,
            pages: 0,
            cur_page: 0,
            capacity: 0,
            toggle_key: Key::Char(' '),
            fuzzy_matcher: SkimMatcherV2::default().use_cache(true).smart_case(),
        };
        let max_height = ms.term.size().0 as usize;
        ms.capacity = max_height.max(8) - 6;
        ms
    }

    /// Set the description of the selector
    pub fn description(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }

    /// Add an option to the selector
    pub fn option(mut self, option: DemandOption<T>) -> Self {
        self.options.push(option);
        self.pages = self.get_pages();
        self
    }

    /// Add multiple options to the selector
    pub fn options(mut self, options: Vec<DemandOption<T>>) -> Self {
        for option in options {
            self.options.push(option);
        }
        self.pages = self.get_pages();
        self
    }

    /// Set the minimum number of options which must be selected
    pub fn min(mut self, min: usize) -> Self {
        self.min = min;
        self
    }

    /// Set the maximum number of options which can be selected
    pub fn max(mut self, max: usize) -> Self {
        self.max = max;
        self
    }

    /// Set whether the selector can be filtered with a query
    pub fn filterable(mut self, filterable: bool) -> Self {
        self.filterable = filterable;
        self
    }

    pub fn filtering(mut self, filtering: bool) -> Self {
        self.filtering = filtering;
        self
    }

    pub fn filter(mut self, filter: &str) -> Self {
        self.filter = filter.to_string();
        self.cursor_x = self.filter.chars().count();
        self.pages = self.get_pages();
        self
    }

    /// Set the key used to toggle selection (default: Space)
    pub fn toggle_key(mut self, key: Key) -> Self {
        self.toggle_key = key;
        self
    }

    /// Set the theme of the selector
    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Displays the selector to the user and returns their selected options
    ///
    /// This function will block until the user submits the input. If the user cancels the input,
    /// an error of type `io::ErrorKind::Interrupted` is returned.
    pub fn run(mut self) -> io::Result<Vec<T>> {
        let ctrlc_handle = ctrlc::show_cursor_after_ctrlc(&self.term)?;

        self.max = self.max.min(self.options.len());
        self.min = self.min.min(self.max);

        loop {
            let output = self.render()?;
            let line_count = output.lines().count();

            self.reposition_and_write(&output, line_count)?;

            if self.filtering {
                match self.term.read_key()? {
                    Key::ArrowDown => self.handle_down()?,
                    Key::ArrowUp => self.handle_up()?,
                    Key::ArrowLeft => self.handle_left()?,
                    Key::ArrowRight => self.handle_right()?,
                    Key::Enter => self.handle_stop_filtering(true)?,
                    Key::Escape => self.handle_stop_filtering(false)?,
                    Key::Backspace => self.handle_filter_backspace()?,
                    key if key == self.toggle_key => self.handle_toggle(),
                    Key::Char(c) => self.handle_filter_key(c)?,
                    _ => {}
                }
            } else {
                self.term.hide_cursor()?;
                match self.term.read_key()? {
                    Key::ArrowDown | Key::Char('j') => self.handle_down()?,
                    Key::ArrowUp | Key::Char('k') => self.handle_up()?,
                    Key::ArrowLeft | Key::Char('h') => self.handle_left()?,
                    Key::ArrowRight | Key::Char('l') => self.handle_right()?,
                    key if key == self.toggle_key || key == Key::Char('x') => self.handle_toggle(),
                    Key::Char('a') => self.handle_toggle_all(),
                    Key::Char('/') if self.filterable => self.handle_start_filtering(),
                    Key::Escape => {
                        if self.filter.is_empty() {
                            self.cleanup()?;
                            self.term.show_cursor()?;
                            ctrlc_handle.close();
                            return Err(io::Error::new(
                                io::ErrorKind::Interrupted,
                                "user cancelled",
                            ));
                        }
                        self.handle_stop_filtering(false)?
                    }
                    Key::Enter => {
                        let selected = self
                            .options
                            .iter()
                            .filter(|o| o.selected)
                            .map(|o| o.label.to_string())
                            .collect::<Vec<_>>();
                        if selected.len() < self.min {
                            if self.min == 1 {
                                self.err = Some("Please select an option".to_string());
                            } else {
                                self.err =
                                    Some(format!("Please select at least {} options", self.min));
                            }
                            continue;
                        }
                        if selected.len() > self.max {
                            if self.max == 1 {
                                self.err = Some("Please select only one option".to_string());
                            } else {
                                self.err =
                                    Some(format!("Please select at most {} options", self.max));
                            }
                            continue;
                        }
                        self.cleanup()?;
                        self.term.show_cursor()?;
                        ctrlc_handle.close();
                        let output = self.render_success(&selected)?;
                        self.term.write_all(output.as_bytes())?;
                        let selected = self
                            .options
                            .into_iter()
                            .filter(|o| o.selected)
                            .map(|o| o.item)
                            .collect::<Vec<_>>();
                        self.term.clear_to_end_of_screen()?;
                        return Ok(selected);
                    }
                    _ => {}
                }
            }
        }
    }

    fn filtered_options(&self) -> Vec<&DemandOption<T>> {
        self.options
            .iter()
            .filter_map(|opt| {
                if self.filter.is_empty() {
                    Some((0, opt))
                } else {
                    self.fuzzy_matcher
                        .fuzzy_match(&opt.label.to_lowercase(), &self.filter.to_lowercase())
                        .map(|score| (score, opt))
                }
            })
            .sorted_by_key(|(score, _opt)| -1 * *score)
            .map(|(_score, opt)| opt)
            .collect()
    }

    fn visible_options(&self) -> Vec<&DemandOption<T>> {
        let filtered_options = self.filtered_options();
        let start = self.cur_page * self.capacity;
        filtered_options
            .into_iter()
            .skip(start)
            .take(self.capacity)
            .collect()
    }

    fn handle_down(&mut self) -> Result<(), io::Error> {
        let visible_options = self.visible_options();
        if self.cursor < visible_options.len().max(1) - 1 {
            self.cursor += 1;
        } else if self.pages > 0 && self.cur_page < self.pages - 1 {
            self.cur_page += 1;
            self.cursor = 0;
            self.term.clear_to_end_of_screen()?;
        }
        Ok(())
    }

    fn handle_up(&mut self) -> Result<(), io::Error> {
        if self.cursor > 0 {
            self.cursor -= 1;
        } else if self.cur_page > 0 {
            self.cur_page -= 1;
            self.cursor = self.visible_options().len().max(1) - 1;
            self.term.clear_to_end_of_screen()?;
        }
        Ok(())
    }

    fn handle_left(&mut self) -> Result<(), io::Error> {
        if self.filtering {
            if self.cursor_x > 0 {
                self.cursor_x -= 1;
            }
        } else if self.cur_page > 0 {
            self.cur_page -= 1;
            self.term.clear_to_end_of_screen()?;
        }
        Ok(())
    }

    fn handle_right(&mut self) -> Result<(), io::Error> {
        if self.filtering {
            if self.cursor_x < self.filter.chars().count() {
                self.cursor_x += 1;
            }
        } else if self.pages > 0 && self.cur_page < self.pages - 1 {
            self.cur_page += 1;
            if self.cursor_y > self.visible_options().len() - 1 {
                self.cursor_y = self.visible_options().len() - 1;
            }
            self.term.clear_to_end_of_screen()?;
        }
        Ok(())
    }

    fn handle_toggle(&mut self) {
        self.err = None;
        let visible_options = self.visible_options();
        if visible_options.is_empty() {
            return;
        }
        let cursor = self.cursor.min(visible_options.len().saturating_sub(1));
        let id = visible_options[cursor].id;
        let selected = visible_options[cursor].selected;
        drop(visible_options);
        self.cursor = cursor;
        self.options
            .iter_mut()
            .find(|o| o.id == id)
            .unwrap()
            .selected = !selected;
    }

    fn handle_toggle_all(&mut self) {
        self.err = None;
        let filtered_options = self.filtered_options();
        if filtered_options.is_empty() {
            return;
        }
        let select = !filtered_options.iter().all(|o| o.selected);
        let ids = filtered_options
            .into_iter()
            .map(|o| o.id)
            .collect::<HashSet<_>>();
        for opt in &mut self.options {
            if ids.contains(&opt.id) {
                opt.selected = select;
            }
        }
    }

    fn handle_start_filtering(&mut self) {
        self.err = None;
        self.filtering = true;
    }

    fn handle_stop_filtering(&mut self, save: bool) -> Result<(), io::Error> {
        self.filtering = false;

        let visible_options = self.visible_options();
        if !visible_options.is_empty() {
            self.cursor = self.cursor.min(self.visible_options().len() - 1);
        }
        if !save {
            self.filter.clear();
            self.reset_paging();
        }
        self.term.clear_to_end_of_screen()
    }

    fn handle_filter_key(&mut self, c: char) -> Result<(), io::Error> {
        let idx = self.get_char_idx(&self.filter, self.cursor_x);
        self.filter.insert(idx, c);
        self.cursor_x += 1;
        self.cursor = 0;
        self.cursor_y = 0;
        self.err = None;
        self.reset_paging();
        self.term.clear_to_end_of_screen()
    }

    fn handle_filter_backspace(&mut self) -> Result<(), io::Error> {
        let chars_count = self.filter.chars().count();
        if chars_count > 0 && self.cursor_x > 0 {
            let idx = self.get_char_idx(&self.filter, self.cursor_x - 1);
            self.filter.remove(idx);
        }
        if self.cursor_x > 0 {
            self.cursor_x -= 1;
        }
        self.cursor = 0;
        self.cursor_y = 0;
        self.err = None;
        self.reset_paging();
        self.term.clear_to_end_of_screen()
    }

    fn reset_paging(&mut self) {
        self.cur_page = 0;
        self.pages = self.get_pages();
    }

    fn get_pages(&self) -> usize {
        if self.filtering || !self.filter.is_empty() {
            ((self.filtered_options().len() as f64) / self.capacity as f64).ceil() as usize
        } else {
            ((self.options.len() as f64) / self.capacity as f64).ceil() as usize
        }
    }

    fn render(&self) -> io::Result<String> {
        let mut out = Buffer::ansi();

        out.set_color(&self.theme.title)?;
        write!(out, "{}", self.title)?;

        if self.err.is_some() {
            out.set_color(&self.theme.error_indicator)?;
            writeln!(out, " *")?;
        } else {
            writeln!(out)?;
        }
        if !self.description.is_empty() || self.pages > 1 {
            out.set_color(&self.theme.description)?;
            write!(out, "{}", self.description)?;
            writeln!(out)?;
        }
        let max_label_len = self
            .visible_options()
            .iter()
            .map(|o| console::measure_text_width(&o.label))
            .max()
            .unwrap_or(0);
        for (i, option) in self.visible_options().into_iter().enumerate() {
            if self.cursor == i {
                out.set_color(&self.theme.cursor)?;
                write!(out, " >")?;
            } else {
                write!(out, "  ")?;
            }
            if option.selected {
                out.set_color(&self.theme.selected_prefix_fg)?;
                write!(out, "{}", self.theme.selected_prefix)?;
                out.set_color(&self.theme.selected_option)?;
                self.print_option_label(&mut out, option, max_label_len)?;
            } else {
                out.set_color(&self.theme.unselected_prefix_fg)?;
                write!(out, "{}", self.theme.unselected_prefix)?;
                out.set_color(&self.theme.unselected_option)?;
                self.print_option_label(&mut out, option, max_label_len)?;
            }
        }

        if self.pages > 1 {
            out.set_color(&self.theme.description)?;
            writeln!(out, " (page {}/{})", self.cur_page + 1, self.pages)?;
        }

        if self.filtering {
            out.set_color(&self.theme.input_cursor)?;

            write!(out, "/")?;
            out.reset()?;

            let cursor_idx = self.get_char_idx(&self.filter, self.cursor_x);
            write!(out, "{}", &self.filter[..cursor_idx])?;

            if cursor_idx < self.filter.len() {
                out.set_color(&self.theme.real_cursor_color(None))?;
                write!(out, "{}", &self.filter[cursor_idx..cursor_idx + 1])?;
                out.reset()?;
            }
            if cursor_idx + 1 < self.filter.len() {
                out.reset()?;
                write!(out, "{}", &self.filter[cursor_idx + 1..])?;
            }
            if cursor_idx >= self.filter.len() {
                out.set_color(&self.theme.real_cursor_color(None))?;
                write!(out, " ")?;
                out.reset()?;
            }
            writeln!(out)?;
            out.reset()?;
        } else if !self.filter.is_empty() {
            out.set_color(&self.theme.description)?;
            write!(out, "/{}", self.filter)?;
        } else if let Some(err) = &self.err {
            out.set_color(&self.theme.error_indicator)?;
            write!(out, " {err}")?;
        }

        self.print_help_keys(&mut out)?;

        writeln!(out)?;
        out.reset()?;

        Ok(std::str::from_utf8(out.as_slice()).unwrap().to_string())
    }

    fn print_option_label(
        &self,
        out: &mut Buffer,
        option: &DemandOption<T>,
        max_label_len: usize,
    ) -> io::Result<()> {
        if let Some(desc) = &option.description {
            let label = console::pad_str(&option.label, max_label_len, Alignment::Left, None);
            if self.filtering && !self.filter.is_empty() {
                self.highlight_matches(out, &label)?;
            } else {
                write!(out, " {label}")?;
            }
            out.set_color(&self.theme.description)?;
            writeln!(out, "  {desc}")?;
        } else if self.filtering && !self.filter.is_empty() {
            self.highlight_matches(out, &option.label)?;
            writeln!(out)?;
        } else {
            writeln!(out, " {}", option.label)?;
        }
        Ok(())
    }

    fn toggle_key_label(&self) -> String {
        match self.toggle_key {
            Key::Char(' ') => "space".to_string(),
            Key::Tab => "tab".to_string(),
            Key::Char(c) => c.to_string(),
            _ => "?".to_string(),
        }
    }

    fn print_help_keys(&self, out: &mut Buffer) -> io::Result<()> {
        let toggle_label = self.toggle_key_label();
        let mut help_keys: Vec<(String, &str)> = vec![("↑/↓/k/j".to_string(), "up/down")];
        if self.pages > 1 {
            help_keys.push(("←/→/h/l".to_string(), "prev/next page"));
        }
        help_keys.push((format!("x/{}", toggle_label), "toggle"));
        help_keys.push(("a".to_string(), "toggle all"));
        if self.filterable {
            if self.filtering {
                help_keys = vec![
                    ("↑/↓".to_string(), "up/down"),
                    (toggle_label, "toggle"),
                    ("esc".to_string(), "clear filter"),
                    ("enter".to_string(), "save filter"),
                ];
            } else {
                help_keys.push(("/".to_string(), "filter"));
                if !self.filter.is_empty() {
                    help_keys.push(("esc".to_string(), "clear filter"));
                }
            }
        }
        if !self.filtering {
            help_keys.push(("enter".to_string(), "confirm"));
        }
        for (i, (key, desc)) in help_keys.iter().enumerate() {
            if i > 0 || (!self.filtering && !self.filter.is_empty()) {
                out.set_color(&self.theme.help_sep)?;
                write!(out, " • ")?;
            }
            out.set_color(&self.theme.help_key)?;
            write!(out, "{key}")?;
            out.set_color(&self.theme.help_desc)?;
            write!(out, " {desc}")?;
        }
        Ok(())
    }

    fn get_char_idx(&self, input: &str, cursor: usize) -> usize {
        input
            .char_indices()
            .nth(cursor)
            .map(|(i, _)| i)
            .unwrap_or(input.len())
    }

    fn highlight_matches(
        &self,
        out: &mut dyn WriteColor,
        label: &str,
    ) -> Result<(), std::io::Error> {
        let matches = self
            .fuzzy_matcher
            .fuzzy_indices(&label.to_lowercase(), &self.filter.to_lowercase());
        if let Some((_, indices)) = matches {
            for (j, c) in label.chars().enumerate() {
                if indices.contains(&j) {
                    out.set_color(&self.theme.selected_option)?;
                } else {
                    out.set_color(&self.theme.unselected_option)?;
                }
                if j == 0 {
                    write!(out, " ")?;
                }
                write!(out, "{c}")?;
            }
        } else {
            write!(out, " {label}")?;
        }
        Ok(())
    }

    fn render_success(&self, selected: &[String]) -> io::Result<String> {
        let mut out = Buffer::ansi();
        out.set_color(&self.theme.title)?;
        write!(out, "{}", self.title)?;
        out.set_color(&self.theme.selected_option)?;
        writeln!(out, " {}", selected.join(", "))?;
        out.reset()?;
        Ok(std::str::from_utf8(out.as_slice()).unwrap().to_string())
    }

    fn reposition_and_write(&mut self, output: &str, line_count: usize) -> io::Result<()> {
        // The previous render leaves the cursor at the end of its last line
        // (no trailing newline), so row `last_line_count - 1` of that render.
        // Moving up by the full count would land one row above the top.
        if self.last_line_count > 0 {
            self.term.move_cursor_up(self.last_line_count - 1)?;
        }

        self.term.move_cursor_left(usize::MAX)?;

        let mut lines = output.lines().peekable();
        while let Some(line) = lines.next() {
            self.term.move_cursor_left(usize::MAX)?;
            self.term.clear_line()?;

            write!(self.term, "{}", line)?;

            if lines.peek().is_some() {
                writeln!(self.term)?;
            }
        }

        if line_count < self.last_line_count {
            let extra = self.last_line_count - line_count;
            for _ in 0..extra {
                writeln!(self.term)?;
                self.term.move_cursor_left(usize::MAX)?;
                self.term.clear_line()?;
            }
            // Leave the cursor on the last line of the new render so the
            // invariant above (cursor at end of row N-1) still holds.
            self.term.move_cursor_up(extra)?;
        }

        self.last_line_count = line_count;
        self.term.flush()?;
        Ok(())
    }

    fn cleanup(&mut self) -> io::Result<()> {
        if self.last_line_count > 0 {
            self.term.clear_last_lines(self.last_line_count)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::test::without_ansi;

    use super::*;
    use indoc::indoc;

    // The redraw regression tests below use `Term::read_write_pair`, which is
    // `#[cfg(unix)]` in the console crate. CI also runs on Windows, where this
    // module simply has no test seam — the bug was reported on Linux/macOS and
    // the fix is platform-agnostic, so the unix-only tests are sufficient.
    #[cfg(unix)]
    use std::fs::{File, OpenOptions};
    #[cfg(unix)]
    use std::os::fd::{AsRawFd, RawFd};
    #[cfg(unix)]
    use std::sync::{Arc, Mutex};

    /// `Term::read_write_pair` is bounded by `Write + Debug + AsRawFd + Send +
    /// 'static`. It only uses the fd for its own `AsRawFd` impl — the actual
    /// I/O goes through `Write::write_all` — so we can satisfy the trait by
    /// holding a throwaway `/dev/null` handle and capture bytes into a shared
    /// `Vec` without any kernel round-trip.
    #[cfg(unix)]
    #[derive(Debug)]
    struct CaptureWriter {
        _fd: File,
        bytes: Arc<Mutex<Vec<u8>>>,
    }

    #[cfg(unix)]
    impl Write for CaptureWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.bytes.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[cfg(unix)]
    impl AsRawFd for CaptureWriter {
        fn as_raw_fd(&self) -> RawFd {
            self._fd.as_raw_fd()
        }
    }

    #[cfg(unix)]
    fn capture_term() -> (Term, Arc<Mutex<Vec<u8>>>) {
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let writer = CaptureWriter {
            _fd: OpenOptions::new().write(true).open("/dev/null").unwrap(),
            bytes: Arc::clone(&bytes),
        };
        let reader = File::open("/dev/null").unwrap();
        (Term::read_write_pair(reader, writer), bytes)
    }

    #[cfg(unix)]
    fn snapshot(buf: &Arc<Mutex<Vec<u8>>>) -> Vec<u8> {
        buf.lock().unwrap().clone()
    }

    #[test]
    fn test_render() {
        let select = MultiSelect::new("Toppings")
            .description("Select your toppings")
            .option(DemandOption::new("Lettuce").selected(true))
            .option(DemandOption::new("Tomatoes").selected(true))
            .option(DemandOption::new("Charm Sauce"))
            .option(DemandOption::new("Jalapenos").label("Jalapeños"))
            .option(DemandOption::new("Cheese"))
            .option(DemandOption::new("Vegan Cheese"))
            .option(DemandOption::new("Nutella"));

        assert_eq!(
            indoc! {
              "Toppings
            Select your toppings
             >[•] Lettuce
              [•] Tomatoes
              [ ] Charm Sauce
              [ ] Jalapeños
              [ ] Cheese
              [ ] Vegan Cheese
              [ ] Nutella
            ↑/↓/k/j up/down • x/space toggle • a toggle all • enter confirm
            "
            },
            without_ansi(select.render().unwrap().as_str())
        );
    }

    #[test]
    fn non_display() {
        struct Thing {
            num: u32,
            _thing: Option<()>,
        }
        let things = [
            Thing {
                num: 1,
                _thing: Some(()),
            },
            Thing {
                num: 2,
                _thing: None,
            },
            Thing {
                num: 3,
                _thing: None,
            },
        ];
        let select = MultiSelect::new("things")
            .description("pick a thing")
            .options(
                things
                    .iter()
                    .enumerate()
                    .map(|(i, t)| {
                        if i == 0 {
                            DemandOption::with_label("First", t)
                        } else {
                            DemandOption::new(t.num).item(t).selected(true)
                        }
                    })
                    .collect(),
            );
        assert_eq!(
            indoc! {
              "things
            pick a thing
             >[ ] First
              [•] 2
              [•] 3
            ↑/↓/k/j up/down • x/space toggle • a toggle all • enter confirm
            "
            },
            without_ansi(select.render().unwrap().as_str())
        );
    }

    /// Regression for the off-by-one in `reposition_and_write` (jdx/demand#129):
    /// a previous version moved the cursor up by `last_line_count` instead of
    /// `last_line_count - 1`, causing the menu to drift up by one row on every
    /// keypress in `mise up -i` and any other multi-iteration redraw.
    ///
    /// Drives the redraw path through a `Term::read_write_pair` whose writer is
    /// a `SharedBuf`, replays the captured bytes through a vt100 emulator, and
    /// asserts that the cursor row is stable across iterations (i.e. the render
    /// origin doesn't drift).
    #[cfg(unix)]
    #[test]
    fn reposition_does_not_drift_upward() {
        let (term, buf) = capture_term();
        let mut ms = MultiSelect::new("Toppings")
            .option(DemandOption::new("Lettuce"))
            .option(DemandOption::new("Tomatoes"))
            .option(DemandOption::new("Cheese"))
            .option(DemandOption::new("Nutella"));
        ms.term = term;

        // Start the vt100 cursor well below the top of the screen, so that an
        // upward off-by-one would actually shift the rendered area visibly
        // rather than getting silently clamped at row 0.
        let mut parser = vt100::Parser::new(40, 120, 0);
        parser.process(&b"\n".repeat(20));

        let mut cursor_rows = Vec::new();
        for _ in 0..6 {
            let before = snapshot(&buf).len();
            let output = ms.render().unwrap();
            let line_count = output.lines().count();
            ms.reposition_and_write(&output, line_count).unwrap();

            let new_bytes = snapshot(&buf)[before..].to_vec();
            parser.process(&new_bytes);
            cursor_rows.push(parser.screen().cursor_position().0);

            // Move the in-menu cursor so the renders aren't byte-identical and
            // the redraw path actually has work to do (clear + rewrite).
            ms.handle_down().unwrap();
        }

        // After iter 1 the vt100 cursor settles at row (start + line_count - 1).
        // With the bug, every subsequent iter pulls it up by another row; with
        // the fix it stays put.
        let first = cursor_rows[0];
        for (i, &row) in cursor_rows.iter().enumerate().skip(1) {
            assert_eq!(
                row, first,
                "iteration {i}: cursor row drifted from {first} to {row} (full trace: {cursor_rows:?})",
            );
        }
    }

    /// Companion to `reposition_does_not_drift_upward`: when the rendered
    /// output shrinks, the new last-line-count must keep the same "cursor at
    /// end of last row" invariant so the next iteration's `move_cursor_up`
    /// still lands on row 0 of the new render.
    #[cfg(unix)]
    #[test]
    fn reposition_handles_shrinking_render() {
        let (term, buf) = capture_term();
        let mut ms = MultiSelect::new("T")
            .option(DemandOption::new("a"))
            .option(DemandOption::new("b"))
            .option(DemandOption::new("c"));
        ms.term = term;

        let mut parser = vt100::Parser::new(40, 120, 0);
        parser.process(&b"\n".repeat(20));

        // First render with all 3 options.
        let before = snapshot(&buf).len();
        let output = ms.render().unwrap();
        let line_count_full = output.lines().count();
        ms.reposition_and_write(&output, line_count_full).unwrap();
        parser.process(&snapshot(&buf)[before..]);
        let row_after_full = parser.screen().cursor_position().0;

        // Drop two options, forcing the next render to shrink.
        ms.options.truncate(1);

        let before = snapshot(&buf).len();
        let output = ms.render().unwrap();
        let line_count = output.lines().count();
        ms.reposition_and_write(&output, line_count).unwrap();
        parser.process(&snapshot(&buf)[before..]);
        let row_after_short = parser.screen().cursor_position().0;

        assert!(line_count < line_count_full, "render must actually shrink");
        // Cursor should land on the last row of the shorter render — i.e. it
        // should have moved up by the difference, not stayed at the old depth.
        assert_eq!(
            row_after_short,
            row_after_full - (line_count_full - line_count) as u16,
            "cursor didn't reposition correctly after shrink"
        );

        // The third render must again be stable — a fresh redraw on top of
        // the shrunken state without further drift.
        let before = snapshot(&buf).len();
        let output = ms.render().unwrap();
        let line_count = output.lines().count();
        ms.reposition_and_write(&output, line_count).unwrap();
        parser.process(&snapshot(&buf)[before..]);
        assert_eq!(parser.screen().cursor_position().0, row_after_short);
    }
}
