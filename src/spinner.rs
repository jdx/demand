use std::{
    io::{self, Write},
    thread::sleep,
    time::Duration,
};

use console::Term;
use termcolor::{Buffer, WriteColor};

use crate::{theme, Theme};

/// Show a spinner
///
/// # Example
/// ```rust
/// use demand::{Spinner,SpinnerStyle};
/// use std::time::Duration;
/// use std::thread::sleep;
///
/// let spinner = Spinner::new("Loading data...")
///   .style(SpinnerStyle::line())
///   .run(|| {
///        sleep(Duration::from_secs(2));
///    })
///   .expect("error running spinner");
/// ```
pub struct Spinner<'a> {
    // The title of the spinner
    pub title: String,
    // The style of the spinner
    pub style: SpinnerStyle,
    /// The colors/style of the spinner
    pub theme: &'a Theme,

    term: Term,
    frame: usize,
    height: usize,
}

impl<'a> Spinner<'a> {
    /// Create a new spinner with the given title
    pub fn new<S: Into<String>>(title: S) -> Self {
        Self {
            title: title.into(),
            style: SpinnerStyle::line(),
            theme: &*theme::DEFAULT,
            term: Term::stderr(),
            frame: 0,
            height: 0,
        }
    }

    /// Set the style of the spinner
    pub fn style(mut self, style: SpinnerStyle) -> Self {
        self.style = style;
        self
    }

    /// Set the theme of the dialog
    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Displays the dialog to the user and returns their response
    pub fn run<F>(mut self, func: F) -> io::Result<()>
    where
        F: Fn() + Send + 'static,
    {
        let handle = std::thread::spawn(move || {
            func();
        });

        self.term.hide_cursor()?;
        loop {
            self.clear()?;
            let output = self.render()?;
            self.height = output.lines().count() - 1;
            self.term.write_all(output.as_bytes())?;
            sleep(self.style.fps);
            if handle.is_finished() {
                self.clear()?;
                self.term.show_cursor()?;
                break;
            }
        }
        Ok(())
    }

    /// Render the spinner and return the output
    fn render(&mut self) -> io::Result<String> {
        let mut out = Buffer::ansi();

        if self.frame > self.style.chars.len() - 1 {
            self.frame = 0
        }

        out.set_color(&self.theme.input_prompt)?;
        write!(out, "{} ", self.style.chars[self.frame])?;
        out.reset()?;

        write!(out, "{}", self.title)?;

        self.frame += 1;

        Ok(std::str::from_utf8(out.as_slice()).unwrap().to_string())
    }

    fn clear(&mut self) -> io::Result<()> {
        self.term.clear_to_end_of_screen()?;
        self.term.clear_last_lines(self.height)?;
        self.height = 0;
        Ok(())
    }
}

/// The style of the spinner
///
/// # Example
/// ```rust
/// use demand::SpinnerStyle;
///
/// let style = SpinnerStyle::dots();
/// ```
pub struct SpinnerStyle {
    chars: Vec<&'static str>,
    fps: Duration,
}

impl SpinnerStyle {
    // Create a new spinner type of dots
    pub fn dots() -> Self {
        Self {
            chars: vec!["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"],
            fps: Duration::from_millis(1000 / 10),
        }
    }
    // Create a new spinner type of jump
    pub fn jump() -> Self {
        Self {
            chars: vec!["⢄", "⢂", "⢁", "⡁", "⡈", "⡐", "⡠"],
            fps: Duration::from_millis(1000 / 10),
        }
    }
    // Create a new spinner type of line
    pub fn line() -> Self {
        Self {
            chars: vec!["-", "\\", "|", "/"],
            fps: Duration::from_millis(1000 / 10),
        }
    }
    // Create a new spinner type of points
    pub fn points() -> Self {
        Self {
            chars: vec!["∙∙∙", "●∙∙", "∙●∙", "∙∙●"],
            fps: Duration::from_millis(1000 / 7),
        }
    }
    // Create a new spinner type of meter
    pub fn meter() -> Self {
        Self {
            chars: vec!["▱▱▱", "▰▱▱", "▰▰▱", "▰▰▰", "▰▰▱", "▰▱▱", "▱▱▱"],
            fps: Duration::from_millis(1000 / 7),
        }
    }
    // Create a new spinner type of mini dots
    pub fn minidots() -> Self {
        Self {
            chars: vec!["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
            fps: Duration::from_millis(1000 / 12),
        }
    }
    // Create a new spinner type of ellipsis
    pub fn ellipsis() -> Self {
        Self {
            chars: vec!["   ", ".  ", ".. ", "..."],
            fps: Duration::from_millis(1000 / 3),
        }
    }
}
