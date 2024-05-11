use std::io;

use console::{Key, Term};
use std::io::Write;
use termcolor::{Buffer, WriteColor};

use crate::{theme, Theme};

pub struct List<'a> {
    pub title: String,
    pub description: String,
    pub success_items: usize,
    pub theme: &'a Theme,

    term: Term,
    items: Vec<&'a str>,
    capacity: usize,
    filtering: bool,
    filterable: bool,
    filter: String,
    cur_page: usize,
    height: usize,
    pages: usize,
    scroll: usize,
}

impl<'a> List<'a> {
    pub fn new<S: Into<String>>(title: S) -> Self {
        let mut s = Self {
            title: title.into(),
            description: String::new(),
            theme: &theme::DEFAULT,
            items: Vec::new(),
            term: Term::stderr(),
            capacity: 0,
            filtering: false,
            filterable: false,
            filter: String::new(),
            height: 0,
            cur_page: 0,
            pages: 0,
            success_items: 4,
            scroll: 0,
        };
        let max_height = s.term.size().0 as usize;
        s.capacity = max_height.max(8) - 5;
        s
    }

    pub fn description(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }

    pub fn item(mut self, entry: &'a str) -> Self {
        self.items.push(entry);
        self.pages = self.get_pages();
        self
    }

    pub fn items(mut self, entries: &[&'a str]) -> Self {
        self.items.extend_from_slice(entries);
        self.pages = self.get_pages();
        self
    }

    pub fn success_items(mut self, items: usize) -> Self {
        self.success_items = items;
        self
    }

    pub fn filterable(mut self, filterable: bool) -> Self {
        self.filterable = filterable;
        self
    }

    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = theme;
        self
    }

    pub fn run(mut self) -> Result<(), io::Error> {
        loop {
            self.clear()?;
            let output = self.render()?;
            self.term.write_all(output.as_bytes())?;
            self.term.flush()?;
            self.height = output.lines().count() - 1;
            if self.filtering {
                // self.term.show_cursor()?;
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
                    Key::ArrowUp | Key::Char('k') => self.handle_up(),
                    Key::ArrowDown | Key::Char('j') => self.handle_down(),
                    Key::ArrowLeft | Key::Char('h') => self.handle_left(),
                    Key::ArrowRight | Key::Char('l') => self.handle_right(),
                    Key::Char('/') if self.filterable => self.handle_start_filtering(),
                    Key::Escape => self.handle_stop_filtering(false),
                    Key::Enter => {
                        self.clear()?;
                        self.term.show_cursor()?;
                        let output = self.render_success()?;
                        self.term.write_all(output.as_bytes())?;
                        return Ok(());
                    }
                    _ => {}
                }
            }
        }
    }

    fn handle_up(&mut self) {
        if self.scroll > 0 {
            self.scroll -= 1;
            self.pages = self.get_pages();
        }
    }

    fn handle_down(&mut self) {
        let saturating_sub = self.filtered_entries().len().saturating_sub(self.capacity);
        if self.scroll < saturating_sub {
            self.scroll += 1;
            self.pages = self.get_pages();
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
        if !save {
            self.filter.clear();
            self.pages = self.get_pages();
        }
    }

    fn handle_filter_backspace(&mut self) {
        self.filter.pop();
        self.scroll = 0;
        self.pages = self.get_pages();
    }

    fn handle_filter_key(&mut self, key: char) {
        self.filter.push(key);
        self.scroll = 0;
        self.pages = self.get_pages();
    }

    fn filtered_entries(&self) -> Vec<&&'a str> {
        self.items
            .iter()
            .filter(|e| {
                self.filter.is_empty() || e.to_lowercase().contains(&self.filter.to_lowercase())
            })
            .collect()
    }

    fn get_pages(&self) -> usize {
        if self.filtering {
            ((self.filtered_entries().len() - self.scroll) as f64 / self.capacity as f64).ceil()
                as usize
        } else {
            ((self.items.len() - self.scroll) as f64 / self.capacity as f64).ceil() as usize
        }
    }

    fn visible_entries(&self) -> Vec<&&'a str> {
        let filtered = self.filtered_entries();
        let start = (self.cur_page * self.capacity) + self.scroll;
        filtered
            .into_iter()
            .skip(start)
            .take(self.capacity)
            .collect()
    }

    fn render(&self) -> Result<String, io::Error> {
        let mut out = Buffer::ansi();

        out.set_color(&self.theme.title)?;
        write!(out, "{}", self.title)?;

        writeln!(out)?;
        if !self.description.is_empty() {
            out.set_color(&self.theme.description)?;
            write!(out, "{}", self.description)?;
            writeln!(out)?;
        }
        for entry in self.visible_entries().iter() {
            out.set_color(&self.theme.unselected_option)?;
            writeln!(out, "  {entry}")?;
        }
        if self.pages > 1 {
            out.set_color(&self.theme.description)?;
            writeln!(out, " (page {}/{})", self.cur_page + 1, self.pages)?;
        }
        if self.filtering {
            out.set_color(&self.theme.input_cursor)?;
            write!(out, "/")?;
            out.reset()?;
            write!(out, "{}", self.filter)?;
            out.set_color(&self.theme.real_cursor_color(None))?;
            writeln!(out, " ")?;
        } else if !self.filter.is_empty() {
            out.set_color(&self.theme.description)?;
            write!(out, "/{}", self.filter)?;
        }
        let mut help_keys = vec![("↑/↓/k/j", "up/down")];
        if self.pages > 1 {
            help_keys.push(("←/→/h/l", "prev/next page"));
        }
        if self.filterable {
            if self.filtering {
                help_keys = vec![("esc", "clear filter"), ("enter", "save filter")]
            } else {
                help_keys.push(("/", "filter"));
            }
        }
        if !self.filtering {
            help_keys.push(("enter", "done"));
        }
        for (i, (key, desc)) in help_keys.iter().enumerate() {
            if i > 0 || !self.filter.is_empty() {
                out.set_color(&self.theme.help_sep)?;
                write!(out, " • ")?;
            }
            out.set_color(&self.theme.help_key)?;
            write!(out, "{key}")?;
            out.set_color(&self.theme.help_desc)?;
            write!(out, " {desc}")?;
        }

        writeln!(out)?;

        out.reset()?;
        Ok(std::str::from_utf8(out.as_slice()).unwrap().to_string())
    }

    fn render_success(&self) -> Result<String, io::Error> {
        let mut out = Buffer::ansi();

        out.set_color(&self.theme.title)?;
        write!(out, "{}", self.title)?;

        for entry in self.items.iter().take(self.success_items) {
            out.set_color(&self.theme.unselected_option)?;
            write!(out, "  {entry},")?;
        }
        if self.items.len() > self.success_items {
            write!(out, " ...")?;
        }

        writeln!(out)?;

        out.reset()?;
        Ok(std::str::from_utf8(out.as_slice()).unwrap().to_string())
    }

    fn clear(&mut self) -> Result<(), io::Error> {
        if self.height > 0 {
            self.term.clear_last_lines(self.height)?;
        } else {
            self.term.clear_line()?;
        }
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
        let list = List::new("Foods")
            .description("yummy thingos")
            .item("chips")
            .item("burger")
            .item("sandwich")
            .item("cupcakes");
        assert_eq!(
            indoc! {
                "Foods
                 yummy thingos
                   chips
                   burger
                   sandwich
                   cupcakes
                 ↑/↓/k/j up/down • enter done
                ",

            },
            without_ansi(list.render().unwrap().as_str())
        )
    }
}
