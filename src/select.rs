use std::io;
use std::io::Write;

use crate::theme::Theme;
use crate::{ctrlc, theme, DemandOption};
use console::{Alignment, Key, Term};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use itertools::Itertools;
use termcolor::{Buffer, WriteColor};

/// Select a single option from a list
///
/// If multiple options are marked as selected, only the last one will be shown as selected.
///
/// # Example
/// ```rust
/// use demand::{DemandOption, Select};
///
/// let select = Select::new("Toppings")
///   .description("Select your topping")
///   .filterable(true)
///   .option(DemandOption::new("Lettuce"))
///   .option(DemandOption::new("Tomatoes").selected(true))
///   .option(DemandOption::new("Charm Sauce"))
///   .option(DemandOption::new("Jalapenos").label("Jalapeños"))
///   .option(DemandOption::new("Cheese"))
///   .option(DemandOption::new("Vegan Cheese"))
///   .option(DemandOption::new("Nutella"));
/// let topping = match select.run() {
///     Ok(value) => value,
///     Err(e) => {
///         if e.kind() == std::io::ErrorKind::Interrupted {
///             println!("Input cancelled");
///             return;
///         } else {
///             panic!("Error: {}", e);
///         }
///     }
/// };
/// ```
pub struct Select<'a, T> {
    /// The title of the selector
    pub title: String,
    /// The colors/style of the selector
    pub theme: &'a Theme,
    /// A description to display after the title
    pub description: String,
    /// The options which can be selected
    pub options: Vec<DemandOption<T>>,
    /// Whether the selector can be filtered with a query
    pub filterable: bool,

    cursor_x: usize,
    cursor_y: usize,
    term: Term,
    filter: String,
    filtering: bool,
    pages: usize,
    cur_page: usize,
    capacity: usize,
    fuzzy_matcher: SkimMatcherV2,
}

impl<'a, T> Select<'a, T> {
    /// Create a new select with the given title
    pub fn new<S: Into<String>>(title: S) -> Self {
        let mut s = Select {
            title: title.into(),
            description: String::new(),
            options: vec![],
            filterable: false,
            theme: &theme::DEFAULT,
            cursor_x: 0,
            cursor_y: 0,
            term: Term::stderr(),
            filter: String::new(),
            filtering: false,
            pages: 0,
            cur_page: 0,
            capacity: 0,
            fuzzy_matcher: SkimMatcherV2::default().use_cache(true).smart_case(),
        };
        let max_height = s.term.size().0 as usize;
        s.capacity = max_height.max(8) - 6;
        s
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
        self.cursor_y = self.get_selected_option_idx();
        self
    }

    /// Add multiple options to the selector
    pub fn options(mut self, options: Vec<DemandOption<T>>) -> Self {
        for option in options {
            self.options.push(option);
        }
        self.pages = self.get_pages();
        self.cursor_y = self.get_selected_option_idx();
        self
    }

    /// Set whether the selector can be filtered with a query
    pub fn filterable(mut self, filterable: bool) -> Self {
        self.filterable = filterable;
        self
    }

    /// Start filtering immediately
    pub fn filtering(mut self, filtering: bool) -> Self {
        self.filtering = filtering;
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
    pub fn run(mut self) -> io::Result<T> {
        let ctrlc_handle = ctrlc::show_cursor_after_ctrlc(&self.term)?;

        loop {
            self.clear()?;
            let output = self.render()?;
            self.term.write_all(output.as_bytes())?;
            self.term.flush()?;
            self.term.hide_cursor()?;
            let enter = |mut select: Select<T>| {
                select.clear()?;
                select.term.show_cursor()?;
                let id = select.visible_options().get(select.cursor_y).unwrap().id;
                let selected = select.options.iter().find(|o| o.id == id).unwrap();
                let output = select.render_success(&selected.label)?;
                let selected = select.options.into_iter().find(|o| o.id == id).unwrap();
                select.term.write_all(output.as_bytes())?;
                select.term.clear_to_end_of_screen()?;
                Ok::<T, io::Error>(selected.item)
            };

            if self.filtering {
                match self.term.read_key()? {
                    Key::ArrowDown => self.handle_down()?,
                    Key::ArrowUp => self.handle_up()?,
                    Key::ArrowLeft => self.handle_left()?,
                    Key::ArrowRight => self.handle_right()?,
                    Key::Enter => return enter(self),
                    Key::Escape => self.handle_stop_filtering(false)?,
                    Key::Backspace => self.handle_filter_backspace()?,
                    Key::Char(c) => self.handle_filter_key(c)?,
                    _ => {}
                }
            } else {
                match self.term.read_key()? {
                    Key::ArrowDown | Key::Char('j') => self.handle_down()?,
                    Key::ArrowUp | Key::Char('k') => self.handle_up()?,
                    Key::ArrowLeft | Key::Char('h') => self.handle_left()?,
                    Key::ArrowRight | Key::Char('l') => self.handle_right()?,
                    Key::Char('/') if self.filterable => self.handle_start_filtering(),
                    Key::Escape => {
                        if self.filter.is_empty() {
                            self.term.show_cursor()?;
                            ctrlc_handle.close();
                            return Err(io::Error::new(
                                io::ErrorKind::Interrupted,
                                "user cancelled",
                            ));
                        }
                        self.handle_stop_filtering(false)?;
                    }
                    Key::Enter => {
                        ctrlc_handle.close();
                        return enter(self);
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
        if self.cursor_y < visible_options.len().max(1) - 1 {
            self.cursor_y += 1;
        } else if self.pages > 0 && self.cur_page < self.pages - 1 {
            self.cur_page += 1;
            self.cursor_y = 0;
            self.term.clear_to_end_of_screen()?;
        }
        Ok(())
    }

    fn handle_up(&mut self) -> Result<(), io::Error> {
        if self.cursor_y > 0 {
            self.cursor_y -= 1;
        } else if self.cur_page > 0 {
            self.cur_page -= 1;
            self.cursor_y = self.visible_options().len().max(1) - 1;
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

    fn handle_start_filtering(&mut self) {
        self.filtering = true;
    }

    fn handle_stop_filtering(&mut self, save: bool) -> Result<(), io::Error> {
        self.filtering = false;
        self.cur_page = 0;

        if !save {
            self.filter.clear();
            self.pages = self.get_pages();
        }
        self.term.clear_to_end_of_screen()
    }

    fn handle_filter_key(&mut self, c: char) -> Result<(), io::Error> {
        let idx = self.get_char_idx(&self.filter, self.cursor_x);
        self.filter.insert(idx, c);
        self.cursor_x += 1;
        self.cursor_y = 0;
        self.cur_page = 0;
        self.pages = self.get_pages();
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
        self.cursor_y = 0;
        self.cur_page = 0;
        self.pages = self.get_pages();
        self.term.clear_to_end_of_screen()
    }

    fn get_pages(&self) -> usize {
        ((self.options.len() as f64) / self.capacity as f64).ceil() as usize
    }

    fn get_selected_option_idx(&mut self) -> usize {
        self.visible_options()
            .iter()
            .rposition(|o| o.selected)
            .unwrap_or(0)
    }

    fn render(&self) -> io::Result<String> {
        let mut out = Buffer::ansi();

        out.set_color(&self.theme.title)?;
        write!(out, "{}", self.title)?;

        writeln!(out)?;
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
        for (i, option) in self.visible_options().iter().enumerate() {
            if self.cursor_y == i {
                out.set_color(&self.theme.cursor)?;
                write!(out, "{}", self.theme.cursor_str)?;
            } else {
                write!(
                    out,
                    "{}",
                    " ".repeat(console::measure_text_width(&self.theme.cursor_str))
                )?;
            }
            out.set_color(&self.theme.unselected_option)?;
            if let Some(desc) = &option.description {
                let label = console::pad_str(&option.label, max_label_len, Alignment::Left, None);
                if self.filtering && !self.filter.is_empty() {
                    self.highlight_matches(&mut out, &label)?;
                } else {
                    write!(out, " {}", label)?;
                }
                out.set_color(&self.theme.description)?;
                writeln!(out, "  {}", desc)?;
            } else if self.filtering && !self.filter.is_empty() {
                self.highlight_matches(&mut out, &option.label)?;
                writeln!(out)?;
            } else {
                writeln!(out, " {}", option.label)?;
            }
        }

        if !self.filtering && self.pages > 1 {
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
        }

        self.print_help_keys(&mut out)?;

        writeln!(out)?;
        out.reset()?;

        Ok(std::str::from_utf8(out.as_slice()).unwrap().to_string())
    }

    fn print_help_keys(&self, out: &mut Buffer) -> io::Result<()> {
        let mut help_keys = vec![("↑/↓/k/j", "up/down")];
        if self.pages > 1 {
            help_keys.push(("←/→/h/l", "prev/next page"));
        }
        if self.filterable {
            if self.filtering {
                help_keys = vec![("esc", "clear filter")];
            } else {
                help_keys.push(("/", "filter"));
                if !self.filter.is_empty() {
                    help_keys.push(("esc", "clear filter"));
                }
            }
        }
        help_keys.push(("enter", "confirm"));
        for (i, (key, desc)) in help_keys.iter().enumerate() {
            if i > 0 || (!self.filtering && !self.filter.is_empty()) {
                out.set_color(&self.theme.help_sep)?;
                write!(out, " • ")?;
            }
            out.set_color(&self.theme.help_key)?;
            write!(out, "{}", key)?;
            out.set_color(&self.theme.help_desc)?;
            write!(out, " {}", desc)?;
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
                write!(out, "{}", c)?;
            }
        } else {
            write!(out, " {}", label)?;
        }
        Ok(())
    }

    fn render_success(&self, selected: &str) -> io::Result<String> {
        let mut out = Buffer::ansi();
        out.set_color(&self.theme.title)?;
        write!(out, "{}", self.title)?;
        out.set_color(&self.theme.selected_option)?;
        writeln!(out, " {}", selected)?;
        out.reset()?;
        Ok(std::str::from_utf8(out.as_slice()).unwrap().to_string())
    }

    fn clear(&mut self) -> io::Result<()> {
        self.term.clear_screen()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::test::without_ansi;

    use super::*;
    use indoc::indoc;

    #[test]
    fn test_render() {
        let select = Select::new("Country")
            .description("Select your Country")
            .option(DemandOption::new("United States"))
            .option(DemandOption::new("Germany"))
            .option(DemandOption::new("Brazil").selected(true))
            .option(DemandOption::new("Canada"))
            .option(DemandOption::new("Mexico"));

        assert_eq!(
            indoc! {
              "Country
            Select your Country
              United States
              Germany
            ❯ Brazil
              Canada
              Mexico
            ↑/↓/k/j up/down • enter confirm
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
        ];
        let select = Select::new("things").description("pick a thing").options(
            things
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    if i == 0 {
                        DemandOption::with_label("First", t).selected(true)
                    } else {
                        DemandOption::new(t.num).item(t)
                    }
                })
                .collect(),
        );
        assert_eq!(
            indoc! {
              "things
            pick a thing
            ❯ First
              2
            ↑/↓/k/j up/down • enter confirm
            "
            },
            without_ansi(select.render().unwrap().as_str())
        );
    }
}
