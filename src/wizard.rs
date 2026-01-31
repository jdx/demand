use std::io;
use std::io::Write;

use crate::theme::Theme;
use crate::{ctrlc, theme};
use console::{Key, Term};
use termcolor::{Buffer, WriteColor};

/// Navigation result from a wizard section
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Navigation {
    /// Go to the next section
    Next,
    /// Go to the previous section
    Back,
    /// Jump to a specific section by index
    Jump(usize),
    /// Return to the hub (section 0) - for hub-and-spoke patterns
    Hub,
    /// Stay on the current section (re-render)
    Stay,
    /// Complete the wizard
    Done,
}

/// Type alias for the section run function
pub type SectionFn<'a, S> = Box<dyn Fn(&mut S, &Theme) -> io::Result<Navigation> + 'a>;

/// A section in a wizard flow
pub struct Section<'a, S> {
    /// The label shown in the breadcrumb
    pub label: String,
    /// The function to run when this section is active
    run: SectionFn<'a, S>,
}

impl<'a, S> Section<'a, S> {
    /// Create a new section with a label and run function
    pub fn new<L, F>(label: L, run: F) -> Self
    where
        L: Into<String>,
        F: Fn(&mut S, &Theme) -> io::Result<Navigation> + 'a,
    {
        Self {
            label: label.into(),
            run: Box::new(run),
        }
    }
}

/// A multi-step wizard with clickable breadcrumb navigation
///
/// # Example
/// ```ignore
/// use demand::{Wizard, Section, Navigation, Input};
///
/// #[derive(Default)]
/// struct State {
///     name: String,
///     email: String,
/// }
///
/// let result = Wizard::new("Setup")
///     .section("Name", |state: &mut State, theme| {
///         let name = Input::new("Your name")
///             .theme(theme)
///             .run()?;
///         state.name = name;
///         Ok(Navigation::Next)
///     })
///     .section("Email", |state: &mut State, theme| {
///         let email = Input::new("Your email")
///             .theme(theme)
///             .run()?;
///         state.email = email;
///         Ok(Navigation::Next)
///     })
///     .section("Done", |_state, _theme| {
///         Ok(Navigation::Done)
///     })
///     .run(&mut State::default())?;
/// ```
pub struct Wizard<'a, S> {
    /// The title displayed above the breadcrumb
    title: String,
    /// The sections in the wizard
    sections: Vec<Section<'a, S>>,
    /// The current section index
    current: usize,
    /// The theme for styling
    theme: &'a Theme,
    /// Track which sections have been visited
    visited: Vec<bool>,
    /// Terminal handle
    term: Term,
}

impl<'a, S> Wizard<'a, S> {
    /// Create a new wizard with the given title
    pub fn new<T: Into<String>>(title: T) -> Self {
        Self {
            title: title.into(),
            sections: Vec::new(),
            current: 0,
            theme: &theme::DEFAULT,
            visited: Vec::new(),
            term: Term::stderr(),
        }
    }

    /// Add a section to the wizard
    pub fn section<L, F>(mut self, label: L, run: F) -> Self
    where
        L: Into<String>,
        F: Fn(&mut S, &Theme) -> io::Result<Navigation> + 'a,
    {
        self.sections.push(Section::new(label, run));
        self.visited.push(false);
        self
    }

    /// Set the theme for the wizard
    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Run the wizard with the given state
    ///
    /// Returns the final state when Navigation::Done is received.
    /// Returns an error with `io::ErrorKind::Interrupted` if the user cancels.
    pub fn run(mut self, state: &mut S) -> io::Result<()> {
        if self.sections.is_empty() {
            return Ok(());
        }

        let ctrlc_handle = ctrlc::show_cursor_after_ctrlc(&self.term)?;

        // Mark first section as visited
        self.visited[0] = true;

        loop {
            // Run the current section
            let section = &self.sections[self.current];
            let result = (section.run)(state, self.theme);

            match result {
                Ok(Navigation::Next) => {
                    if self.current + 1 < self.sections.len() {
                        self.current += 1;
                        self.visited[self.current] = true;
                    }
                }
                Ok(Navigation::Back) => {
                    if self.current > 0 {
                        self.current -= 1;
                        // Mark as visited in case we jumped over this section
                        self.visited[self.current] = true;
                    }
                }
                Ok(Navigation::Jump(idx)) => {
                    if idx < self.sections.len() {
                        self.current = idx;
                        self.visited[self.current] = true;
                    }
                }
                Ok(Navigation::Hub) => {
                    // Return to section 0 (the hub)
                    self.current = 0;
                }
                Ok(Navigation::Stay) => {
                    // Stay on current section, just re-render
                }
                Ok(Navigation::Done) => {
                    self.term.show_cursor()?;
                    ctrlc_handle.close();
                    return Ok(());
                }
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {
                    // Escape pressed - go back if possible, otherwise cancel
                    if self.current > 0 {
                        self.current -= 1;
                        // Mark as visited in case we jumped over this section
                        self.visited[self.current] = true;
                    } else {
                        self.term.show_cursor()?;
                        ctrlc_handle.close();
                        return Err(e);
                    }
                }
                Err(e) => {
                    self.term.show_cursor()?;
                    ctrlc_handle.close();
                    return Err(e);
                }
            }
        }
    }

    /// Render the breadcrumb navigation bar as a string.
    ///
    /// Sections can call this to include the breadcrumb in their output if desired.
    /// Returns the rendered breadcrumb as a string that can be printed.
    pub fn render_breadcrumb(&self) -> io::Result<String> {
        let mut out = Buffer::ansi();

        // Title
        out.set_color(&self.theme.title)?;
        writeln!(out, "{}", self.title)?;

        // Breadcrumb: [1:Section] > 2:Section > 3:Section
        for (i, section) in self.sections.iter().enumerate() {
            if i > 0 {
                out.set_color(&self.theme.description)?;
                write!(out, "{}", self.theme.breadcrumb_separator)?;
            }

            let is_current = i == self.current;
            let is_visited = self.visited[i];

            if is_current {
                out.set_color(&self.theme.breadcrumb_active)?;
                write!(out, "[{}:{}]", i + 1, section.label)?;
            } else if is_visited {
                out.set_color(&self.theme.breadcrumb_clickable)?;
                write!(out, "{}:{}", i + 1, section.label)?;
            } else {
                out.set_color(&self.theme.breadcrumb_future)?;
                write!(out, "{}:{}", i + 1, section.label)?;
            }
        }
        writeln!(out)?;

        // Separator line
        out.set_color(&self.theme.description)?;
        let width = self.term.size().1 as usize;
        let line_width = width.min(50);
        writeln!(out, "{}", "─".repeat(line_width))?;
        writeln!(out)?;

        out.reset()?;
        Ok(std::str::from_utf8(out.as_slice()).unwrap().to_string())
    }
}

/// Helper to check if a key is a navigation key and return the navigation action.
///
/// This utility function is provided for library consumers who want to implement
/// custom widgets that support wizard navigation shortcuts. It checks for:
/// - Number keys (1-9): Jump to that section
/// - Escape: Go back to the previous section (returns `None` if at first section)
///
/// # Example
/// ```ignore
/// use demand::{handle_navigation_key, Navigation};
/// use console::Key;
///
/// // In your custom widget's key handling loop:
/// if let Some(nav) = handle_navigation_key(key, current_section, total_sections) {
///     return Ok(nav);
/// }
/// // ... handle other keys
/// ```
pub fn handle_navigation_key(key: Key, current: usize, section_count: usize) -> Option<Navigation> {
    match key {
        Key::Escape => {
            if current > 0 {
                Some(Navigation::Back)
            } else {
                None // Let the widget handle cancel
            }
        }
        Key::Char(c) if c.is_ascii_digit() && c != '0' => {
            let idx = (c as usize) - ('1' as usize);
            if idx < section_count {
                Some(Navigation::Jump(idx))
            } else {
                None
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navigation_enum() {
        assert_eq!(Navigation::Next, Navigation::Next);
        assert_eq!(Navigation::Back, Navigation::Back);
        assert_eq!(Navigation::Jump(0), Navigation::Jump(0));
        assert_ne!(Navigation::Jump(0), Navigation::Jump(1));
        assert_eq!(Navigation::Hub, Navigation::Hub);
        assert_eq!(Navigation::Stay, Navigation::Stay);
        assert_eq!(Navigation::Done, Navigation::Done);
    }

    #[test]
    fn test_handle_navigation_key() {
        // Test number keys
        assert_eq!(
            handle_navigation_key(Key::Char('1'), 2, 4),
            Some(Navigation::Jump(0))
        );
        assert_eq!(
            handle_navigation_key(Key::Char('3'), 0, 4),
            Some(Navigation::Jump(2))
        );
        assert_eq!(
            handle_navigation_key(Key::Char('5'), 0, 4),
            None // Index out of range
        );
        assert_eq!(
            handle_navigation_key(Key::Char('0'), 0, 4),
            None // 0 is not a valid navigation key
        );

        // Test escape
        assert_eq!(
            handle_navigation_key(Key::Escape, 2, 4),
            Some(Navigation::Back)
        );
        assert_eq!(
            handle_navigation_key(Key::Escape, 0, 4),
            None // Can't go back from first section
        );

        // Test other keys
        assert_eq!(handle_navigation_key(Key::Enter, 0, 4), None);
        assert_eq!(handle_navigation_key(Key::ArrowDown, 0, 4), None);
    }

    #[test]
    fn test_section_creation() {
        let section: Section<'_, i32> = Section::new("Test", |_state, _theme| Ok(Navigation::Next));
        assert_eq!(section.label, "Test");
    }

    #[test]
    fn test_wizard_builder() {
        let wizard: Wizard<'_, i32> = Wizard::new("Test Wizard")
            .section("Step 1", |_state, _theme| Ok(Navigation::Next))
            .section("Step 2", |_state, _theme| Ok(Navigation::Done));

        assert_eq!(wizard.sections.len(), 2);
        assert_eq!(wizard.sections[0].label, "Step 1");
        assert_eq!(wizard.sections[1].label, "Step 2");
    }
}
