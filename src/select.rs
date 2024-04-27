use std::io;
use std::io::Write;

use console::{Key, Term};
use termcolor::{Buffer, WriteColor};

use crate::theme::Theme;
use crate::{theme, DemandOption};

/// Select multiple options from a list
///
/// # Example
/// ```rust
/// use demand::{DemandOption, Select};
///
/// let select = Select::new("Toppings")
///   .description("Select your topping")
///   .filterable(true)
///   .option(DemandOption::new("Lettuce"))
///   .option(DemandOption::new("Tomatoes"))
///   .option(DemandOption::new("Charm Sauce"))
///   .option(DemandOption::new("Jalapenos").label("Jalapeños"))
///   .option(DemandOption::new("Cheese"))
///   .option(DemandOption::new("Vegan Cheese"))
///   .option(DemandOption::new("Nutella"));
/// let topping = select.run().expect("error running multi select");
/// ```
pub struct Select<'a, T> {
    /// The title of the selector
    pub title: String,
    /// The colors/style of the selector
    pub theme: &'a Theme,
    /// A description to display above the selector
    pub description: String,
    /// The options which can be selected
    pub options: Vec<DemandOption<T>>,
    /// Whether the selector can be filtered with a query
    pub filterable: bool,

    cursor: usize,
    height: usize,
    term: Term,
    filter: String,
    filtering: bool,
    pages: usize,
    cur_page: usize,
    capacity: usize,
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
            cursor: 0,
            height: 0,
            term: Term::stderr(),
            filter: String::new(),
            filtering: false,
            pages: 0,
            cur_page: 0,
            capacity: 0,
        };
        let max_height = s.term.size().0 as usize;
        s.capacity = max_height.max(8) - 4;
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

    /// Set whether the selector can be filtered with a query
    pub fn filterable(mut self, filterable: bool) -> Self {
        self.filterable = filterable;
        self
    }

    /// Set the theme of the selector
    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Displays the selector to the user and returns their selected options
    pub fn run(mut self) -> io::Result<T> {
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
                    Key::Char('/') if self.filterable => self.handle_start_filtering(),
                    Key::Escape => self.handle_stop_filtering(false),
                    Key::Enter => {
                        self.clear()?;
                        self.term.show_cursor()?;
                        let id = self.visible_options().get(self.cursor).unwrap().id;
                        let selected = self.options.iter().find(|o| o.id == id).unwrap();
                        let output = self.render_success(&selected.label)?;
                        let selected = self.options.into_iter().find(|o| o.id == id).unwrap();
                        self.term.write_all(output.as_bytes())?;
                        return Ok(selected.item);
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
        } else if self.pages > 0 && self.cur_page < self.pages - 1 {
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
        if self.pages > 0 && self.cur_page < self.pages - 1 {
            self.cur_page += 1;
        }
    }

    fn handle_start_filtering(&mut self) {
        self.filtering = true;
    }

    fn handle_stop_filtering(&mut self, save: bool) {
        self.filtering = false;
        self.cur_page = 0;

        let visible_options = self.visible_options();
        if !visible_options.is_empty() {
            self.cursor = self.cursor.min(self.visible_options().len() - 1);
        } else {
            self.cursor = 0;
        }
        if !save {
            self.filter.clear();
            self.pages = self.get_pages();
        }
    }

    fn handle_filter_key(&mut self, c: char) {
        self.filter.push(c);
        self.pages = self.get_pages();
    }

    fn handle_filter_backspace(&mut self) {
        self.filter.pop();
        self.pages = self.get_pages();
    }

    fn get_pages(&self) -> usize {
        if self.filtering {
            ((self.filtered_options().len() as f64) / self.capacity as f64).ceil() as usize
        } else {
            ((self.options.len() as f64) / self.capacity as f64).ceil() as usize
        }
    }

    fn render(&self) -> io::Result<String> {
        let mut out = Buffer::ansi();

        out.set_color(&self.theme.title)?;
        write!(out, " {}", self.title)?;

        writeln!(out)?;
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
            out.set_color(&self.theme.unselected_option)?;
            writeln!(out, " {}", option.label)?;
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
        } else {
            let mut help_keys = vec![("↑/↓/k/j", "up/down")];
            if self.pages > 1 {
                help_keys.push(("←/→/h/l", "prev/next page"));
            }
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
        writeln!(out)?;

        out.reset()?;
        Ok(std::str::from_utf8(out.as_slice()).unwrap().to_string())
    }

    fn render_success(&self, selected: &str) -> io::Result<String> {
        let mut out = Buffer::ansi();
        out.set_color(&self.theme.title)?;
        write!(out, " {}", self.title)?;
        out.set_color(&self.theme.selected_option)?;
        writeln!(out, " {}", selected)?;
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
    use crate::test::without_ansi;

    use super::*;
    use indoc::indoc;

    #[test]
    fn test_render() {
        let select = Select::new("Country")
            .description("Select your Country")
            .option(DemandOption::new("United States").selected(true))
            .option(DemandOption::new("Germany"))
            .option(DemandOption::new("Brazil"))
            .option(DemandOption::new("Canada"))
            .option(DemandOption::new("Mexico"));

        assert_eq!(
            indoc! {
              " Country
               Select your Country
               > United States
                 Germany
                 Brazil
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
              " things
               pick a thing
               > First
                 2

              ↑/↓/k/j up/down • enter confirm
              "
            },
            without_ansi(select.render().unwrap().as_str())
        );
    }
}
