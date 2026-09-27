use std::borrow::Cow;
use std::fmt::Display;

use crate::height::rendered_rows;
use std::sync::atomic::AtomicUsize;

/// An individual option in a select or multi-select.
#[derive(Debug, Clone)]
pub struct DemandOption<T> {
    /// Unique ID for this option.
    pub(crate) id: usize,
    /// The item this option represents.
    pub item: T,
    /// Display label for this option.
    pub label: String,
    /// Whether this option is initially selected.
    pub selected: bool,
    /// Optional description shown on the side.
    pub description: Option<String>,
}

impl<T: ToString> DemandOption<T> {
    /// Create a new option with the item as the label
    pub fn new(item: T) -> Self {
        static ID: AtomicUsize = AtomicUsize::new(0);
        Self {
            id: ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            label: item.to_string(),
            item,
            selected: false,
            description: None,
        }
    }
}

impl<T> DemandOption<T> {
    /// Create a new option with a label and item
    pub fn with_label<S: Into<String>>(label: S, item: T) -> Self {
        static ID: AtomicUsize = AtomicUsize::new(0);
        Self {
            id: ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            label: label.into(),
            item,
            selected: false,
            description: None,
        }
    }
    pub fn item<I>(self, item: I) -> DemandOption<I> {
        DemandOption {
            id: self.id,
            item,
            label: self.label,
            selected: self.selected,
            description: None,
        }
    }
    /// Set the display label for this option.
    pub fn label(mut self, name: &str) -> Self {
        self.label = name.to_string();
        self
    }

    /// Set whether this option is initially selected.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }
}

impl<T: Display> PartialEq for DemandOption<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T: Display> Eq for DemandOption<T> {}

/// Lay out `options` on pages `available` rows tall in a terminal `width`
/// columns wide, drawn the way `Select` and `MultiSelect` draw them:
/// `indent` columns of cursor and prefix, then ` label`, padded to the
/// widest label when the option has a description, then `  description`.
/// Returns how many options fit on a page, and how to print descriptions.
///
/// Every option is given as many rows as the tallest one wraps into, so a
/// page fits on screen whichever options land on it (jdx/demand#235). An
/// option too tall to fit even alone is the exception: rather than size
/// every page for it, its description is cut short to the height the rest
/// were given.
pub(crate) fn layout_options<T>(
    options: &[&DemandOption<T>],
    indent: usize,
    width: usize,
    available: usize,
) -> (usize, DescriptionFit) {
    let label_width = options
        .iter()
        .map(|o| console::measure_text_width(&o.label))
        .max()
        .unwrap_or(0);
    let available = available.max(1);
    let mut rows = 1;
    let mut cut = false;
    for option in options {
        let needed = option_rows(option, indent, label_width, width);
        if needed <= available {
            rows = rows.max(needed);
        } else {
            // It can be cut down to its label and a `…`, and no further.
            let least = match option.description {
                Some(_) => rendered_rows(&option_line(option, indent, label_width, "…"), width),
                None => needed,
            };
            rows = rows.max(least);
            cut = true;
        }
    }
    let fit = DescriptionFit {
        rows: cut.then_some(rows),
        width,
    };
    ((available / rows).max(1), fit)
}

/// How descriptions are printed after [`layout_options`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DescriptionFit {
    /// Rows an option may take before its description is cut short with
    /// `…`, or `None` when every description is printed whole.
    pub rows: Option<usize>,
    width: usize,
}

impl DescriptionFit {
    /// `desc` as printed after `before`, the text ahead of it on the
    /// option's line: whole if the line fits in the rows allowed, or cut as
    /// short as it takes to fit, on one line.
    pub(crate) fn fit<'d>(&self, before: &str, desc: &'d str) -> Cow<'d, str> {
        let Some(rows) = self.rows else {
            return desc.into();
        };
        let fits = |desc: &str| rendered_rows(&format!("{before}{desc}"), self.width) <= rows;
        if fits(desc) {
            return desc.into();
        }
        let desc = desc.replace('\n', " ");
        // The widest cut that fits; a wide character can wrap before the
        // edge, so the columns alone don't say.
        let (mut lo, mut hi) = (0, console::measure_text_width(&desc));
        while lo < hi {
            let mid = (lo + hi).div_ceil(2);
            if fits(&console::truncate_str(&desc, mid, "…")) {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        console::truncate_str(&desc, lo, "…").into_owned().into()
    }
}

/// The line `option` is drawn as, with `desc` in place of its description.
fn option_line<T>(
    option: &DemandOption<T>,
    indent: usize,
    label_width: usize,
    desc: &str,
) -> String {
    format!(
        "{} {}  {desc}",
        " ".repeat(indent),
        console::pad_str(&option.label, label_width, console::Alignment::Left, None)
    )
}

/// Rows `option` wraps into, its label padded to `label_width`. Labels are
/// padded to the widest one on the page rather than in the whole list, so
/// measuring against the whole list can only overestimate.
fn option_rows<T>(
    option: &DemandOption<T>,
    indent: usize,
    label_width: usize,
    width: usize,
) -> usize {
    let desc = option.description.as_deref();
    let cols = indent
        + 1
        + match desc {
            Some(desc) => label_width + 2 + console::measure_text_width(desc),
            None => console::measure_text_width(&option.label),
        };
    // Most options fit, and a line that fits can't wrap: skip building it
    // and walking it a character at a time.
    if cols <= width && !option.label.contains('\n') && !desc.is_some_and(|d| d.contains('\n')) {
        return 1;
    }
    let line = match desc {
        Some(desc) => option_line(option, indent, label_width, desc),
        None => format!("{} {}", " ".repeat(indent), option.label),
    };
    rendered_rows(&line, width)
}
