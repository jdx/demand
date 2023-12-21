use std::sync::atomic::AtomicUsize;

/// An individual option in a select or multi-select.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DemandOption {
    /// Unique ID for this option.
    id: usize,
    /// Key for this option. Returned by the select.
    pub key: String,
    /// Display label for this option.
    pub label: String,
    /// Whether this option is initially selected.
    pub selected: bool,
}

impl DemandOption {
    /// Create a new option with the given key.
    pub fn new(key: &str) -> Self {
        static ID: AtomicUsize = AtomicUsize::new(0);
        Self {
            id: ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            key: key.to_string(),
            label: key.to_string(),
            selected: false,
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
