use std::fmt::Display;
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

/// Rows the tallest of `options` wraps into in a terminal `width` columns
/// wide, drawn the way `Select` and `MultiSelect` draw them: `indent`
/// columns of cursor and prefix, then ` label`, padded to the widest label
/// when the option has a description, then `  description`.
///
/// Pagination gives every option this many rows, so a page fits on screen
/// whichever options land on it. Labels are padded to the widest one on
/// the page rather than in the whole list, so measuring against the whole
/// list can only overestimate.
pub(crate) fn tallest_option_rows<T>(
    options: &[&DemandOption<T>],
    indent: usize,
    width: usize,
) -> usize {
    let label_width = options
        .iter()
        .map(|o| console::measure_text_width(&o.label))
        .max()
        .unwrap_or(0);
    options
        .iter()
        .map(|o| {
            let desc = o.description.as_deref();
            let cols = indent
                + 1
                + match desc {
                    Some(desc) => label_width + 2 + console::measure_text_width(desc),
                    None => console::measure_text_width(&o.label),
                };
            // Most options fit, and a line that fits can't wrap: skip
            // building it and walking it a character at a time.
            if cols <= width && !o.label.contains('\n') && !desc.is_some_and(|d| d.contains('\n')) {
                return 1;
            }
            let line = match desc {
                Some(desc) => format!(
                    "{} {}  {desc}",
                    " ".repeat(indent),
                    console::pad_str(&o.label, label_width, console::Alignment::Left, None)
                ),
                None => format!("{} {}", " ".repeat(indent), o.label),
            };
            crate::height::rendered_rows(&line, width)
        })
        .max()
        .unwrap_or(1)
        .max(1)
}
