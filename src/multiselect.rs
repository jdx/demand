use crate::theme::Theme;
use crate::DemandOption;
use console::{Key, Term};
use std::collections::HashSet;
use std::fmt::Display;
use std::io;
use std::io::Write;
use termcolor::{Buffer, WriteColor};

/// Select multiple options from a list
///
/// # Example
/// ```rust
/// use demand::{DemandOption, MultiSelect};
///
/// let ms = MultiSelect::new("Toppings")
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
///   .option(DemandOption::new("Nutella"));///
/// let toppings = ms.run().expect("error running multi select");
/// ```
pub struct MultiSelect<T: Display> {
    /// The title of the selector
    pub title: String,
    /// The colors/style of the selector
    pub theme: Theme,
    /// A description to display above the selector
    pub description: String,
    /// The options which can be selected
    pub options: Vec<DemandOption<T>>,
    /// The minimum number of options which must be selected
    pub min: usize,
    /// The maximum number of options which can be selected
    pub max: usize,
    /// Whether the selector can be filtered with a query
    pub filterable: bool,
    err: Option<String>,
    cursor: usize,
    height: usize,
    term: Term,
    filter: String,
    filtering: bool,
    pages: usize,
    cur_page: usize,
    capacity: usize,
}

impl<T: Display> MultiSelect<T> {
    /// Create a new multi select with the given title
    pub fn new<S: Into<String>>(title: S) -> Self {
        Self {
            title: title.into(),
            description: String::new(),
            options: vec![],
            min: 0,
            max: usize::MAX,
            filterable: false,
            theme: Theme::default(),
            err: None,
            cursor: 0,
            height: 0,
            term: Term::stderr(),
            filter: String::new(),
            filtering: false,
            pages: 0,
            cur_page: 0,
            capacity: 0,
        }
    }

    /// Set the description of the selector
    pub fn description(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }

    /// Add an option to the selector
    pub fn option(mut self, option: DemandOption<T>) -> Self {
        self.options.push(option);
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

    /// Set the theme of the selector
    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Displays the selector to the user and returns their selected options
    pub fn run(mut self) -> io::Result<Vec<T>> {
        let max_height = self.term.size().0 as usize;
        self.capacity = max_height.max(8) - 4;
        self.pages = ((self.options.len() as f64) / self.capacity as f64).ceil() as usize;

        self.max = self.max.min(self.options.len());
        self.min = self.min.min(self.max);
        loop {
            self.clear()?;
            let output = self.render()?;
            self.term.write_all(output.as_bytes())?;
            self.term.flush()?;
            self.height = output.lines().count() - 1;
            if self.filtering {
                self.term.show_cursor()?;
                match self.term.read_key()? {
                    Key::Enter => self.handle_stop_filtering(true),
                    Key::Escape => self.handle_stop_filtering(false),
                    Key::Backspace => self.handle_filter_backspace(),
                    Key::Char(c) => self.handle_filter_key(c),
                    _ => {}
                }
            } else {
                self.term.hide_cursor()?;
                match self.term.read_key()? {
                    Key::ArrowDown | Key::Char('j') => self.handle_down(),
                    Key::ArrowUp | Key::Char('k') => self.handle_up(),
                    Key::ArrowLeft | Key::Char('h') => self.handle_left(),
                    Key::ArrowRight | Key::Char('l') => self.handle_right(),
                    Key::Char('x') | Key::Char(' ') => self.handle_toggle(),
                    Key::Char('a') => self.handle_toggle_all(),
                    Key::Char('/') if self.filterable => self.handle_start_filtering(),
                    Key::Escape => self.handle_stop_filtering(false),
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
                        self.clear()?;
                        self.term.show_cursor()?;
                        let output = self.render_success(&selected)?;
                        self.term.write_all(output.as_bytes())?;
                        let selected = self
                            .options
                            .into_iter()
                            .filter(|o| o.selected)
                            .map(|o| o.item)
                            .collect::<Vec<_>>();
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
            .filter(|opt| {
                self.filter.is_empty()
                    || opt
                        .label
                        .to_lowercase()
                        .contains(&self.filter.to_lowercase())
            })
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

    fn handle_down(&mut self) {
        let visible_options = self.visible_options();
        if self.cursor < visible_options.len().max(1) - 1 {
            self.cursor += 1;
        } else if self.cur_page < self.pages - 1 {
            self.cur_page += 1;
            self.cursor = 0;
        }
    }

    fn handle_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        } else if self.cur_page > 0 {
            self.cur_page -= 1;
            self.cursor = self.visible_options().len().max(1) - 1;
        }
    }

    fn handle_left(&mut self) {
        if self.cur_page > 0 {
            self.cur_page -= 1;
        }
    }

    fn handle_right(&mut self) {
        if self.cur_page < self.pages - 1 {
            self.cur_page += 1;
        }
    }

    fn handle_toggle(&mut self) {
        self.err = None;
        let visible_options = self.visible_options();
        if visible_options.is_empty() {
            return;
        }
        let id = visible_options[self.cursor].id;
        let selected = visible_options[self.cursor].selected;
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

    fn handle_stop_filtering(&mut self, save: bool) {
        self.filtering = false;

        let visible_options = self.visible_options();
        if !visible_options.is_empty() {
            self.cursor = self.cursor.min(self.visible_options().len() - 1);
        }
        if !save {
            self.filter.clear();
        }
    }

    fn handle_filter_key(&mut self, c: char) {
        self.err = None;
        self.filter.push(c);
    }

    fn handle_filter_backspace(&mut self) {
        self.err = None;
        self.filter.pop();
    }

    fn render(&self) -> io::Result<String> {
        let mut out = Buffer::ansi();

        out.set_color(&self.theme.title)?;
        write!(out, " {}", self.title)?;

        if self.err.is_some() {
            out.set_color(&self.theme.error_indicator)?;
            writeln!(out, " *")?;
        } else {
            writeln!(out)?;
        }
        if !self.description.is_empty() || self.pages > 1 {
            out.set_color(&self.theme.description)?;
            write!(out, " {}", self.description)?;
            if self.pages > 1 {
                write!(out, " (page {}/{})", self.cur_page + 1, self.pages)?;
            }
            writeln!(out)?;
        }
        for (i, option) in self.visible_options().iter().enumerate() {
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
                writeln!(out, " {}", option.label)?;
            } else {
                out.set_color(&self.theme.unselected_prefix_fg)?;
                write!(out, "{}", self.theme.unselected_prefix)?;
                out.set_color(&self.theme.unselected_option)?;
                writeln!(out, " {}", option.label)?;
            }
        }
        writeln!(out)?;

        if self.filtering {
            out.set_color(&self.theme.input_cursor)?;

            write!(out, "/")?;
            out.reset()?;
            write!(out, "{}", self.filter)?;
        } else if !self.filter.is_empty() {
            out.set_color(&self.theme.description)?;
            write!(out, "/{}", self.filter)?;
        } else if let Some(err) = &self.err {
            out.set_color(&self.theme.error_indicator)?;
            write!(out, " {}", err)?;
        } else {
            let mut help_keys = vec![("↑/↓/k/j", "up/down")];
            if self.pages > 1 {
                help_keys.push(("←/→/h/l", "prev/next page"));
            }
            help_keys.push(("x/space", "toggle"));
            help_keys.push(("a", "toggle all"));
            if self.filterable {
                if self.filtering {
                    help_keys = vec![("esc", "clear filter"), ("enter", "save filter")];
                } else {
                    help_keys.push(("/", "filter"));
                    if !self.filter.is_empty() {
                        help_keys.push(("esc", "clear filter"));
                    }
                }
            }
            if !self.filtering {
                help_keys.push(("enter", "confirm"));
            }
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
        }

        out.reset()?;
        Ok(std::str::from_utf8(out.as_slice()).unwrap().to_string())
    }

    fn render_success(&self, selected: &[String]) -> io::Result<String> {
        let mut out = Buffer::ansi();
        out.set_color(&self.theme.title)?;
        write!(out, " {}", self.title)?;
        out.set_color(&self.theme.selected_option)?;
        writeln!(out, " {}", selected.join(", "))?;
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
