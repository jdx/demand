use std::io;
use std::io::Write;

use console::{Key, Term};
use fuzzy_matcher::skim::SkimMatcherV2;
use itertools::Itertools;
use termcolor::{Buffer, WriteColor};

use crate::theme::Theme;
use crate::{ctrlc, theme};

/// Pick one column for each row of a table
///
/// Every row carries a label and one cell per column. A cell can be
/// empty, meaning that choice isn't available for the row. ↑/↓ move
/// between rows and ←/→ move the row's choice between its available
/// cells. `run` returns every row's item with the index of the column
/// chosen for it. A row with no available cell has nothing to choose,
/// so it is neither shown nor returned.
///
/// The first column is the row's baseline: the summary printed after
/// confirming lists only the rows moved off it.
///
/// # Example
/// ```rust
/// use demand::{GridRow, GridSelect};
///
/// let grid = GridSelect::new("Pick the packages you want to upgrade")
///   .columns(["Current", "Range", "Latest"])
///   .filterable(true)
///   .row(GridRow::new("react").cell("^18.2.0").cell("^18.3.1").cell("^19.0.0"))
///   .row(GridRow::new("chalk").cell("^4.1.2").empty_cell().cell("^5.0.0"))
///   .row(GridRow::new("jest").cell("^27.4.7").cell("^27.5.1"));
/// let choices = match grid.run() {
///   Ok(choices) => choices,
///   Err(e) => {
///       if e.kind() == std::io::ErrorKind::Interrupted {
///           println!("Input cancelled");
///           return;
///       } else {
///           panic!("Error: {}", e);
///       }
///   }
/// };
/// for (item, column) in choices {
///     println!("{item}: {column}");
/// }
/// ```
pub struct GridSelect<'a, T> {
    /// The title of the selector
    pub title: String,
    /// The colors/style of the selector
    pub theme: &'a Theme,
    /// A description to display after the title
    pub description: String,
    /// Headers for the choice columns
    pub columns: Vec<String>,
    /// The rows to choose for
    pub rows: Vec<GridRow<T>>,
    /// Whether the rows can be filtered with a query
    pub filterable: bool,
    /// Whether the rows are currently being filtered
    pub filtering: bool,
    /// A filter query to preset
    pub filter: String,

    /// Position of the focused row among the filtered rows.
    cursor: usize,
    capacity: usize,
    frame: crate::frame::Frame,
    term: Term,
    fuzzy_matcher: SkimMatcherV2,
}

/// A row of a [`GridSelect`]
#[derive(Debug, Clone)]
pub struct GridRow<T> {
    /// The item this row represents
    pub item: T,
    /// Display label for this row
    pub label: String,
    /// One entry per column; `None` when that choice isn't available
    pub cells: Vec<Option<String>>,
    /// Index of the chosen column
    pub selected: usize,
}

impl<T: ToString> GridRow<T> {
    /// Create a new row with the item as the label
    pub fn new(item: T) -> Self {
        let label = item.to_string();
        Self::with_label(label, item)
    }
}

impl<T> GridRow<T> {
    /// Create a new row with a label and item
    pub fn with_label<S: Into<String>>(label: S, item: T) -> Self {
        Self {
            item,
            label: single_line(label.into()),
            cells: vec![],
            selected: 0,
        }
    }

    /// Set the display label for this row
    pub fn label(mut self, label: &str) -> Self {
        self.label = single_line(label.to_string());
        self
    }

    /// Append a choice in the next column
    pub fn cell<S: Into<String>>(mut self, text: S) -> Self {
        self.cells.push(Some(single_line(text.into())));
        self
    }

    /// Append an unavailable choice in the next column
    pub fn empty_cell(mut self) -> Self {
        self.cells.push(None);
        self
    }

    /// Set the initially chosen column
    pub fn selected(mut self, column: usize) -> Self {
        self.selected = column;
        self
    }

    fn is_available(&self, column: usize) -> bool {
        matches!(self.cells.get(column), Some(Some(_)))
    }

    /// Move the choice `dir` columns over, skipping unavailable cells.
    fn step(&mut self, dir: isize) {
        let mut column = self.selected;
        loop {
            let Some(next) = column.checked_add_signed(dir) else {
                return;
            };
            if next >= self.cells.len() {
                return;
            }
            column = next;
            if self.is_available(column) {
                self.selected = column;
                return;
            }
        }
    }

    /// Make sure the chosen column holds an available cell, falling back
    /// to the first one that does.
    fn normalize(&mut self) {
        if !self.is_available(self.selected)
            && let Some(column) = (0..self.cells.len()).find(|&c| self.is_available(c))
        {
            self.selected = column;
        }
    }
}

/// Rows are laid out one terminal line each, so line breaks in labels and
/// cells are shown as spaces.
fn single_line(text: String) -> String {
    if text.contains(['\n', '\r']) {
        text.replace("\r\n", " ").replace(['\n', '\r'], " ")
    } else {
        text
    }
}

impl<'a, T> GridSelect<'a, T> {
    /// Create a new grid select with the given title
    pub fn new<S: Into<String>>(title: S) -> Self {
        GridSelect {
            title: title.into(),
            theme: &theme::DEFAULT,
            description: String::new(),
            columns: vec![],
            rows: vec![],
            filterable: false,
            filtering: false,
            filter: String::new(),
            cursor: 0,
            capacity: 1,
            frame: Default::default(),
            term: Term::stderr(),
            fuzzy_matcher: SkimMatcherV2::default().use_cache(true).smart_case(),
        }
    }

    /// Set the description of the selector
    pub fn description(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }

    /// Set the headers of the choice columns
    pub fn columns<I, S>(mut self, columns: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.columns = columns.into_iter().map(Into::into).collect();
        self
    }

    /// Add a row to the selector
    pub fn row(mut self, row: GridRow<T>) -> Self {
        self.rows.push(row);
        self
    }

    /// Add multiple rows to the selector
    pub fn rows(mut self, rows: Vec<GridRow<T>>) -> Self {
        self.rows.extend(rows);
        self
    }

    /// Set whether the rows can be filtered with a query
    pub fn filterable(mut self, filterable: bool) -> Self {
        self.filterable = filterable;
        self
    }

    /// Set the theme of the selector
    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Displays the selector to the user and returns every row's item
    /// with the index of the column chosen for it, in the order the rows
    /// were added
    ///
    /// This function will block until the user submits the input. If the user cancels the input,
    /// an error of type `io::ErrorKind::Interrupted` is returned.
    pub fn run(mut self) -> io::Result<Vec<(T, usize)>> {
        let ctrlc_handle = ctrlc::show_cursor_after_ctrlc(&self.term)?;
        let mut events = crate::event::EventReader::new()?;
        let mut reset_viewport = false;

        self.prepare();

        loop {
            let (term_rows, term_cols) = self.term.size();
            self.capacity = self.capacity_for(term_rows as usize, term_cols as usize);
            let term = self.term.clone();
            crate::synchronized_output::run(&term, || {
                if reset_viewport {
                    self.term.clear_screen()?;
                    self.frame.forget();
                    reset_viewport = false;
                }
                self.redraw()
            })?;

            let Some(key) = events.read_key(&self.term)? else {
                reset_viewport = true;
                continue;
            };
            if !self.filtering {
                self.term.hide_cursor()?;
            }
            match key {
                Key::ArrowDown => self.handle_down(),
                Key::ArrowUp => self.handle_up(),
                Key::ArrowLeft => self.handle_step(-1),
                Key::ArrowRight => self.handle_step(1),
                Key::PageDown => self.handle_page(1),
                Key::PageUp => self.handle_page(-1),
                Key::Enter => {
                    ctrlc_handle.close();
                    return self.finish();
                }
                Key::Escape if self.filtering || !self.filter.is_empty() => {
                    self.handle_clear_filter()
                }
                Key::Escape => {
                    self.frame.clear(&self.term)?;
                    self.term.show_cursor()?;
                    ctrlc_handle.close();
                    return Err(io::Error::new(io::ErrorKind::Interrupted, "user cancelled"));
                }
                Key::Backspace if self.filtering => self.handle_filter_backspace(),
                Key::Char(c) if self.filtering => self.handle_filter_key(c),
                Key::Char('j') => self.handle_down(),
                Key::Char('k') => self.handle_up(),
                Key::Char('h') => self.handle_step(-1),
                Key::Char('l') => self.handle_step(1),
                Key::Char('/') if self.filterable => self.filtering = true,
                _ => {}
            }
        }
    }

    fn prepare(&mut self) {
        self.rows
            .retain(|row| row.cells.iter().any(Option::is_some));
        // Columns past the last available cell of every row would only
        // take up space, unless they have a header.
        let width = self
            .rows
            .iter()
            .filter_map(|r| r.cells.iter().rposition(Option::is_some))
            .map(|last| last + 1)
            .max()
            .unwrap_or(0)
            .max(self.columns.len());
        for row in &mut self.rows {
            row.cells.resize(width, None);
            row.normalize();
        }
        self.columns.resize(width, String::new());
        let (term_rows, term_cols) = self.term.size();
        self.capacity = self.capacity_for(term_rows as usize, term_cols as usize);
    }

    fn finish(mut self) -> io::Result<Vec<(T, usize)>> {
        let output = self.render_success()?;
        let term = self.term.clone();
        crate::synchronized_output::run(&term, || {
            self.frame.clear(&self.term)?;
            self.term.show_cursor()?;
            self.term.write_all(output.as_bytes())?;
            self.term.clear_to_end_of_screen()
        })?;
        Ok(self
            .rows
            .into_iter()
            .map(|row| (row.item, row.selected))
            .collect())
    }

    /// How many rows fit on one page of a `term_rows` × `term_cols`
    /// terminal, after the lines drawn around them and any wrapping.
    fn capacity_for(&self, term_rows: usize, term_cols: usize) -> usize {
        let lines = |text: &str| -> usize {
            text.split('\n')
                .map(|line| crate::height::rows_for(line, term_cols))
                .sum()
        };
        let mut reserved = lines(&self.title);
        if !self.description.is_empty() {
            reserved += lines(&self.description);
        }
        let label_width = self
            .rows
            .iter()
            .map(|r| console::measure_text_width(&r.label))
            .max()
            .unwrap_or(0);
        let row_width = 3 + label_width + self.column_widths().iter().map(|w| 2 + w).sum::<usize>();
        // Every row, the header included, is as wide as the widest one.
        let header = lines(&" ".repeat(row_width)).max(1);
        // Assume paging, since whether it's needed depends on the result.
        let help = self
            .help_keys(2)
            .iter()
            .map(|(key, desc)| format!("{key} {desc}"))
            .join(" • ");
        // An applied filter shares the help line; one being typed gets its own.
        let help = if !self.filtering && !self.filter.is_empty() {
            format!("/{} {help}", self.filter)
        } else {
            help
        };
        let page = lines(&format!(" (page {0}/{0})", self.rows.len()));
        let filter = usize::from(self.filtering);
        // header, page indicator, filter, help, and the line the cursor rests on
        let reserved = reserved + header + page + filter + lines(&help) + 1;
        (term_rows.saturating_sub(reserved) / header).max(1)
    }

    /// Indices into `rows` that match the filter, best match first.
    fn filtered(&self) -> Vec<usize> {
        if self.filter.is_empty() {
            return (0..self.rows.len()).collect();
        }
        self.rows
            .iter()
            .enumerate()
            .filter_map(|(i, row)| {
                crate::fuzzy::score(&self.fuzzy_matcher, &row.label, &self.filter)
                    .map(|score| (score, i))
            })
            .sorted_by_key(|(score, _)| -score)
            .map(|(_, i)| i)
            .collect()
    }

    fn pages(&self, filtered: usize) -> usize {
        filtered.div_ceil(self.capacity.max(1))
    }

    fn page(&self) -> usize {
        self.cursor / self.capacity.max(1)
    }

    fn handle_down(&mut self) {
        if self.cursor + 1 < self.filtered().len() {
            self.cursor += 1;
        }
    }

    fn handle_up(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    fn handle_page(&mut self, dir: isize) {
        let last = self.filtered().len().saturating_sub(1);
        let target = self.page() as isize + dir;
        if target < 0 {
            self.cursor = 0;
        } else {
            self.cursor = (target as usize * self.capacity).min(last);
        }
    }

    fn handle_step(&mut self, dir: isize) {
        if let Some(&i) = self.filtered().get(self.cursor) {
            self.rows[i].step(dir);
        }
    }

    fn handle_clear_filter(&mut self) {
        self.filtering = false;
        self.filter.clear();
        self.cursor = 0;
    }

    fn handle_filter_key(&mut self, c: char) {
        self.filter.push(c);
        self.cursor = 0;
    }

    fn handle_filter_backspace(&mut self) {
        self.filter.pop();
        self.cursor = 0;
    }

    /// Width of each choice column, measured across every row so paging
    /// and filtering don't shift the columns around.
    fn column_widths(&self) -> Vec<usize> {
        let marker = console::measure_text_width(&self.theme.selected_prefix)
            .max(console::measure_text_width(&self.theme.unselected_prefix));
        self.columns
            .iter()
            .enumerate()
            .map(|(c, header)| {
                let cells = self
                    .rows
                    .iter()
                    .filter_map(|row| row.cells.get(c).and_then(Option::as_deref))
                    .map(|text| marker + 1 + console::measure_text_width(text))
                    .max()
                    .unwrap_or(0);
                cells.max(console::measure_text_width(header))
            })
            .collect()
    }

    fn render(&self) -> io::Result<String> {
        let mut out = Buffer::ansi();

        out.set_color(&self.theme.title)?;
        writeln!(out, "{}", self.title)?;
        let filtered = self.filtered();
        let pages = self.pages(filtered.len());
        if !self.description.is_empty() {
            out.set_color(&self.theme.description)?;
            writeln!(out, "{}", self.description)?;
        }

        let label_width = self
            .rows
            .iter()
            .map(|r| console::measure_text_width(&r.label))
            .max()
            .unwrap_or(0);
        let widths = self.column_widths();

        out.set_color(&self.theme.description)?;
        let mut pad = 3 + label_width;
        for (header, width) in self.columns.iter().zip(&widths) {
            write!(out, "{}{header}", " ".repeat(pad + 2))?;
            pad = width - console::measure_text_width(header);
        }
        writeln!(out)?;

        let start = self.page() * self.capacity;
        for (pos, &i) in filtered.iter().enumerate().skip(start).take(self.capacity) {
            let row = &self.rows[i];
            if pos == self.cursor {
                out.set_color(&self.theme.cursor)?;
                write!(out, " > ")?;
            } else {
                write!(out, "   ")?;
            }
            out.set_color(&self.theme.unselected_option)?;
            if self.filtering && !self.filter.is_empty() {
                self.highlight_matches(&mut out, &row.label)?;
            } else {
                write!(out, "{}", row.label)?;
            }
            // Padding is only written ahead of the next cell, so rows
            // carry no trailing whitespace.
            let mut pad = label_width - console::measure_text_width(&row.label);
            for (c, width) in widths.iter().enumerate() {
                pad += 2;
                let Some(Some(text)) = row.cells.get(c) else {
                    pad += width;
                    continue;
                };
                write!(out, "{}", " ".repeat(pad))?;
                let (prefix, prefix_fg, text_color) = if row.selected == c {
                    (
                        &self.theme.selected_prefix,
                        &self.theme.selected_prefix_fg,
                        &self.theme.selected_option,
                    )
                } else {
                    (
                        &self.theme.unselected_prefix,
                        &self.theme.unselected_prefix_fg,
                        &self.theme.unselected_option,
                    )
                };
                out.set_color(prefix_fg)?;
                write!(out, "{prefix}")?;
                out.set_color(text_color)?;
                write!(out, " {text}")?;
                let used =
                    console::measure_text_width(prefix) + 1 + console::measure_text_width(text);
                pad = width.saturating_sub(used);
            }
            out.reset()?;
            writeln!(out)?;
        }

        if pages > 1 {
            out.set_color(&self.theme.description)?;
            writeln!(out, " (page {}/{})", self.page() + 1, pages)?;
        }

        if self.filtering {
            out.set_color(&self.theme.input_cursor)?;
            write!(out, "/")?;
            out.reset()?;
            write!(out, "{}", self.filter)?;
            out.set_color(&self.theme.real_cursor_color(None))?;
            write!(out, " ")?;
            out.reset()?;
            writeln!(out)?;
        } else if !self.filter.is_empty() {
            out.set_color(&self.theme.description)?;
            write!(out, "/{} ", self.filter)?;
        }

        self.print_help_keys(&mut out, pages)?;

        writeln!(out)?;
        out.reset()?;

        Ok(std::str::from_utf8(out.as_slice()).unwrap().to_string())
    }

    fn help_keys(&self, pages: usize) -> Vec<(&'static str, &'static str)> {
        let mut help_keys = if self.filtering {
            vec![("↑/↓", "up/down"), ("←/→", "choose")]
        } else {
            vec![("↑/↓/k/j", "up/down"), ("←/→/h/l", "choose")]
        };
        if pages > 1 {
            help_keys.push(("pgup/pgdn", "prev/next page"));
        }
        if self.filtering || !self.filter.is_empty() {
            help_keys.push(("esc", "clear filter"));
        } else if self.filterable {
            help_keys.push(("/", "filter"));
        }
        help_keys.push(("enter", "confirm"));
        help_keys
    }

    fn print_help_keys(&self, out: &mut Buffer, pages: usize) -> io::Result<()> {
        for (i, (key, desc)) in self.help_keys(pages).iter().enumerate() {
            if i > 0 {
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

    fn highlight_matches(&self, out: &mut dyn WriteColor, label: &str) -> io::Result<()> {
        let indices =
            crate::fuzzy::indices(&self.fuzzy_matcher, label, &self.filter).unwrap_or_default();
        for (j, c) in label.chars().enumerate() {
            if indices.contains(&j) {
                out.set_color(&self.theme.selected_option)?;
            } else {
                out.set_color(&self.theme.unselected_option)?;
            }
            write!(out, "{c}")?;
        }
        Ok(())
    }

    fn render_success(&self) -> io::Result<String> {
        let mut out = Buffer::ansi();
        out.set_color(&self.theme.title)?;
        write!(out, "{}", self.title)?;
        let changed = self
            .rows
            .iter()
            .filter(|row| row.selected != 0)
            .filter_map(|row| {
                let text = row.cells.get(row.selected)?.as_deref()?;
                Some(format!("{} {text}", row.label))
            })
            .join(", ");
        out.set_color(&self.theme.selected_option)?;
        writeln!(out, " {changed}")?;
        out.reset()?;
        Ok(std::str::from_utf8(out.as_slice()).unwrap().to_string())
    }

    /// Render a frame and draw it over the previous one.
    fn redraw(&mut self) -> io::Result<()> {
        let output = self.render()?;
        self.frame.update(&self.term, output)
    }
}

#[cfg(test)]
mod tests {
    use crate::test::without_ansi;
    #[cfg(unix)]
    use crate::test::{Parser, capture_term, replay, snapshot};

    use super::*;
    use indoc::indoc;

    fn packages() -> GridSelect<'static, &'static str> {
        let mut grid = GridSelect::new("Upgrade")
            .columns(["Current", "Range", "Latest"])
            .row(
                GridRow::new("react")
                    .cell("^18.2.0")
                    .cell("^18.3.1")
                    .cell("^19.0.0"),
            )
            .row(
                GridRow::new("chalk")
                    .cell("^4.1.2")
                    .empty_cell()
                    .cell("^5.0.0"),
            )
            .row(GridRow::new("jest").cell("^27.4.7").cell("^27.5.1"));
        grid.prepare();
        grid
    }

    #[test]
    fn test_render() {
        let grid = packages().description("Pick versions");
        assert_eq!(
            indoc! {
              "Upgrade
            Pick versions
                      Current      Range        Latest
             > react  [•] ^18.2.0  [ ] ^18.3.1  [ ] ^19.0.0
               chalk  [•] ^4.1.2                [ ] ^5.0.0
               jest   [•] ^27.4.7  [ ] ^27.5.1
            ↑/↓/k/j up/down • ←/→/h/l choose • enter confirm
            "
            },
            without_ansi(grid.render().unwrap().as_str())
        );
    }

    #[test]
    fn step_skips_unavailable_cells_and_stops_at_edges() {
        let mut grid = packages();
        grid.cursor = 1;
        grid.handle_step(1);
        assert_eq!(grid.rows[1].selected, 2);
        grid.handle_step(1);
        assert_eq!(grid.rows[1].selected, 2);
        grid.handle_step(-1);
        assert_eq!(grid.rows[1].selected, 0);
        grid.handle_step(-1);
        assert_eq!(grid.rows[1].selected, 0);

        grid.cursor = 2;
        grid.handle_step(1);
        grid.handle_step(1);
        assert_eq!(grid.rows[2].selected, 1, "padded cells are unavailable");
    }

    #[test]
    fn selection_on_unavailable_cell_moves_to_first_available() {
        let mut grid = GridSelect::new("t")
            .row(GridRow::new("a").empty_cell().cell("x").selected(0))
            .row(GridRow::new("b").cell("y").empty_cell().selected(1));
        grid.prepare();
        assert_eq!(grid.rows[0].selected, 1);
        assert_eq!(grid.rows[1].selected, 0);
    }

    #[test]
    fn filter_applies_choices_to_the_matching_row() {
        let mut grid = packages().filterable(true);
        grid.filtering = true;
        for c in "jest".chars() {
            grid.handle_filter_key(c);
        }
        grid.handle_step(1);
        assert_eq!(grid.rows[2].selected, 1);
        assert_eq!(grid.rows[0].selected, 0);
        let rendered = without_ansi(&grid.render().unwrap()).to_string();
        assert!(rendered.contains(" > jest"), "{rendered}");
        assert!(!rendered.contains("react"), "{rendered}");
    }

    #[test]
    fn success_lists_rows_moved_off_the_first_column() {
        let mut grid = packages();
        grid.rows[0].selected = 2;
        grid.rows[2].selected = 1;
        assert_eq!(
            "Upgrade react ^19.0.0, jest ^27.5.1\n",
            without_ansi(&grid.render_success().unwrap())
        );
    }

    #[test]
    fn rows_without_a_choice_are_left_out() {
        let mut grid = GridSelect::new("t")
            .row(GridRow::new("a").empty_cell().empty_cell())
            .row(GridRow::new("b").cell("x"));
        grid.prepare();
        assert_eq!(grid.rows.len(), 1);
        assert_eq!(grid.rows[0].item, "b");
    }

    #[test]
    fn trailing_empty_columns_are_dropped() {
        let mut grid = GridSelect::new("t")
            .columns(["A"])
            .row(GridRow::new("gone").empty_cell().empty_cell().empty_cell())
            .row(GridRow::new("kept").cell("x").cell("y").empty_cell());
        grid.prepare();
        assert_eq!(grid.columns.len(), 2);
        assert_eq!(grid.rows[0].cells.len(), 2);
    }

    #[test]
    fn line_breaks_in_labels_and_cells_become_spaces() {
        let row = GridRow::new("a\nb").cell("1\r\n2");
        assert_eq!(row.label, "a b");
        assert_eq!(row.cells[0].as_deref(), Some("1 2"));
    }

    #[test]
    fn full_page_fits_the_terminal() {
        let mut grid = GridSelect::new("t").description("d").filterable(true).rows(
            (0..50)
                .map(|i| GridRow::new(i.to_string()).cell("a").cell("b"))
                .collect(),
        );
        grid.prepare();
        grid.filtering = true;
        grid.capacity = grid.capacity_for(20, 80);
        let frame = grid.render().unwrap();
        assert!(frame.contains("(page 1/"), "{frame}");
        assert!(
            crate::height::rendered_height(&frame, 80) < 20,
            "{} rows:\n{frame}",
            crate::height::rendered_height(&frame, 80)
        );

        // Rows wider than the terminal wrap, and take two lines each.
        let narrow = grid.capacity_for(20, 10);
        grid.capacity = narrow;
        let frame = grid.render().unwrap();
        assert!(crate::height::rendered_height(&frame, 10) < 20, "{frame}");
    }

    #[test]
    fn paging_follows_the_cursor() {
        let mut grid = GridSelect::new("t").rows(
            (0..10)
                .map(|i| GridRow::new(i.to_string()).cell("a").cell("b"))
                .collect(),
        );
        grid.prepare();
        grid.capacity = 4;
        for _ in 0..5 {
            grid.handle_down();
        }
        assert_eq!(grid.page(), 1);
        let rendered = without_ansi(&grid.render().unwrap()).to_string();
        assert!(rendered.contains(" > 5"), "{rendered}");
        assert!(rendered.contains("(page 2/3)"), "{rendered}");
        grid.handle_page(1);
        assert_eq!(grid.cursor, 8);
        grid.handle_page(1);
        assert_eq!(grid.cursor, 9);
        grid.handle_page(-1);
        assert_eq!(grid.cursor, 4);
    }

    #[cfg(unix)]
    #[test]
    fn redraw_does_not_drift_upward() {
        let (term, buf) = capture_term();
        let mut grid = packages();
        grid.term = term;

        let mut parser = Parser::new(40, 120, 0);
        parser.process(&b"\n".repeat(20));

        let mut cursor_rows = Vec::new();
        for i in 0..6 {
            let before = snapshot(&buf).len();
            grid.redraw().unwrap();
            replay(&mut parser, &snapshot(&buf)[before..]);
            cursor_rows.push(parser.screen().cursor_position().0);
            if i % 2 == 0 {
                grid.handle_down();
            } else {
                grid.handle_step(1);
            }
        }
        let first = cursor_rows[0];
        assert!(
            cursor_rows.iter().all(|&row| row == first),
            "cursor drifted: {cursor_rows:?}"
        );
        assert_eq!(parser.screen().contents().matches("Upgrade").count(), 1);
    }
}
