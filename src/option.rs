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
        }
    }
    pub fn item<I>(self, item: I) -> DemandOption<I> {
        DemandOption {
            id: self.id,
            item,
            label: self.label,
            selected: self.selected,
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
}

impl<T: Display> PartialEq for DemandOption<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T: Display> Eq for DemandOption<T> {}
