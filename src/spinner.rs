use std::{
    io::{self, Write},
    marker::PhantomData,
    sync::{
        LazyLock,
        mpsc::{self, Sender, TryRecvError},
    },
    thread::sleep,
    time::Duration,
};

use console::Term;
use termcolor::{Buffer, WriteColor};

use crate::{Theme, ctrlc, theme};

/// tell a prompt to do something while running
/// currently its only useful for spinner
/// but that could change
pub enum SpinnerAction {
    /// change the theme
    Theme(&'static Theme),
    /// change the style
    Style(&'static SpinnerStyle),
    /// change the title
    Title(String),
}

// SAFETY: ensure that 'spinner lives longer than any use of style or theme by spinner
pub struct SpinnerActionRunner<'spinner> {
    sender: Sender<SpinnerAction>,
    r: PhantomData<&'spinner ()>, // need to use 'spinner to have it on the struct
}

impl<'spinner> SpinnerActionRunner<'spinner> {
    fn new(sender: Sender<SpinnerAction>) -> Self {
        Self {
            sender,
            r: PhantomData,
        }
    }

    /// set the spinner theme
    /// will not compile if ref to theme doesn't outlast spinner
    pub fn theme(
        &mut self, // with just this the compiler assumes that theme might be stored in self so it wont let u mutate it after this fn call
        theme: &'spinner Theme,
    ) -> Result<(), std::sync::mpsc::SendError<SpinnerAction>> {
        let theme = unsafe { std::mem::transmute::<&Theme, &Theme>(theme) };
        self.sender.send(SpinnerAction::Theme(theme))
    }

    /// set the spinner style
    /// will not compile if ref to style doesn't outlast spinner
    pub fn style(
        &mut self, // with just this the compiler assumes that theme might be stored in self so it wont let u mutate it after this fn call
        style: &'spinner SpinnerStyle,
    ) -> Result<(), std::sync::mpsc::SendError<SpinnerAction>> {
        let style = unsafe { std::mem::transmute::<&SpinnerStyle, &SpinnerStyle>(style) };
        self.sender.send(SpinnerAction::Style(style))
    }

    /// set the spinner title
    pub fn title<S: Into<String>>(
        &self,
        title: S,
    ) -> Result<(), std::sync::mpsc::SendError<SpinnerAction>> {
        self.sender.send(SpinnerAction::Title(title.into()))
    }
}

/// Show a spinner
///
/// # Example
/// ```rust
/// use demand::{Spinner,SpinnerStyle};
/// use std::time::Duration;
/// use std::thread::sleep;
///
/// let spinner = Spinner::new("Loading data...")
///   .style(&SpinnerStyle::line())
///   .run(|_| {
///        sleep(Duration::from_secs(2));
///    })
///   .expect("error running spinner");
/// ```
pub struct Spinner<'a> {
    // The title of the spinner
    pub title: String,
    // The style of the spinner
    pub style: &'a SpinnerStyle,
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
            style: &DEFAULT,
            theme: &theme::DEFAULT,
            term: Term::stderr(),
            frame: 0,
            height: 0,
        }
    }

    /// Set the style of the spinner
    pub fn style(mut self, style: &'a SpinnerStyle) -> Self {
        self.style = style;
        self
    }

    /// Set the theme of the dialog
    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Displays the dialog to the user and returns their response
    // SAFETY: 'spinner must out live 'scope
    // this ensures that as long as the spinner doesnt try to access the theme
    // or style outside of the scope closure the theme and style will still be valid
    pub fn run<'scope, 'spinner: 'scope, F, T>(mut self, func: F) -> io::Result<T>
    where
        F: FnOnce(&mut SpinnerActionRunner<'spinner>) -> T + Send + 'scope,
        T: Send + 'scope,
    {
        let t = self.term.clone();
        let _ctrlc_handle = ctrlc::set_ctrlc_handler(move || {
            t.show_cursor().unwrap();
            std::process::exit(130);
        })?;

        std::thread::scope(|s| {
            let (sender, receiver) = mpsc::channel();
            let handle = s.spawn(move || {
                // so you can just |s| instead of |mut s|
                let mut sender = SpinnerActionRunner::new(sender);
                func(&mut sender)
            });
            self.term.hide_cursor()?;
            loop {
                match receiver.try_recv() {
                    Ok(a) => match a {
                        SpinnerAction::Title(title) => self.title = title,
                        SpinnerAction::Style(s) => self.style = s,
                        SpinnerAction::Theme(theme) => self.theme = theme,
                    },
                    Err(TryRecvError::Empty) => (),
                    Err(TryRecvError::Disconnected) => {
                        self.clear()?;
                        self.term.show_cursor()?;
                        break;
                    }
                }
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
            handle
                .join()
                .map_err(|e| io::Error::other(format!("thread panicked: {e:?}")))
        })
    }

    /// Render the spinner and return the output
    fn render(&mut self) -> io::Result<String> {
        let mut out = Buffer::ansi();

        if self.frame > self.style.frames.len() - 1 {
            self.frame = 0
        }

        out.set_color(&self.theme.input_prompt)?;
        write!(out, "{} ", self.style.frames[self.frame])?;
        out.reset()?;

        write!(out, "{}", self.title)?;

        self.frame += 1;

        Ok(std::str::from_utf8(out.as_slice()).unwrap().to_string())
    }

    fn clear(&mut self) -> io::Result<()> {
        if self.height == 0 {
            self.term.clear_line()?;
        } else {
            self.term.clear_last_lines(self.height)?;
        }
        self.height = 0;
        Ok(())
    }
}

pub(crate) static DEFAULT: LazyLock<SpinnerStyle> = LazyLock::new(SpinnerStyle::line);

/// The style of the spinner
///
/// # Example
/// ```rust
/// use demand::SpinnerStyle;
/// use std::time::Duration;
///
/// let dots_style = SpinnerStyle::dots();
/// let custom_style = SpinnerStyle {
///   frames: vec!["  ", ". ", "..", "..."],
///   fps: Duration::from_millis(1000 / 10),
/// };
/// ```
pub struct SpinnerStyle {
    /// The characters to use as frames for the spinner
    pub frames: Vec<&'static str>,
    /// The frames per second of the spinner
    /// Usually represented as a fraction of a second in milliseconds for example `Duration::from_millis(1000/10)`
    /// which would be 10 frames per second
    pub fps: Duration,
}

impl SpinnerStyle {
    // Create a new spinner type of dots
    pub fn dots() -> Self {
        Self {
            frames: vec!["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"],
            fps: Duration::from_millis(1000 / 10),
        }
    }
    // Create a new spinner type of jump
    pub fn jump() -> Self {
        Self {
            frames: vec!["⢄", "⢂", "⢁", "⡁", "⡈", "⡐", "⡠"],
            fps: Duration::from_millis(1000 / 10),
        }
    }
    // Create a new spinner type of line
    pub fn line() -> Self {
        Self {
            frames: vec!["-", "\\", "|", "/"],
            fps: Duration::from_millis(1000 / 10),
        }
    }
    // Create a new spinner type of points
    pub fn points() -> Self {
        Self {
            frames: vec!["∙∙∙", "●∙∙", "∙●∙", "∙∙●"],
            fps: Duration::from_millis(1000 / 7),
        }
    }
    // Create a new spinner type of meter
    pub fn meter() -> Self {
        Self {
            frames: vec!["▱▱▱", "▰▱▱", "▰▰▱", "▰▰▰", "▰▰▱", "▰▱▱", "▱▱▱"],
            fps: Duration::from_millis(1000 / 7),
        }
    }
    // Create a new spinner type of mini dots
    pub fn minidots() -> Self {
        Self {
            frames: vec!["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
            fps: Duration::from_millis(1000 / 12),
        }
    }
    // Create a new spinner type of ellipsis
    pub fn ellipsis() -> Self {
        Self {
            frames: vec!["   ", ".  ", ".. ", "..."],
            fps: Duration::from_millis(1000 / 3),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::test::without_ansi;

    use super::*;

    #[test]
    fn test_render() {
        for t in vec![
            SpinnerStyle::dots(),
            SpinnerStyle::jump(),
            SpinnerStyle::line(),
            SpinnerStyle::points(),
            SpinnerStyle::meter(),
            SpinnerStyle::minidots(),
            SpinnerStyle::ellipsis(),
        ] {
            let mut spinner = Spinner::new("Loading data...").style(&t);
            for f in spinner.style.frames.clone().iter() {
                assert_eq!(
                    format!("{} Loading data...", f),
                    without_ansi(spinner.render().unwrap().as_str())
                );
            }
        }
    }

    #[test]
    fn scope_test() {
        let spinner = Spinner::new("Scoped");
        let mut a = [1, 2, 3];
        let mut i = 0;
        let out = spinner
            .run(|_| {
                for n in &mut a {
                    if i == 1 {
                        *n = 5;
                    }
                    i += 1;
                    std::thread::sleep(Duration::from_millis(*n));
                }
                i * 5
            })
            .unwrap();
        assert_eq!(a, [1, 5, 3]);
        assert_eq!(out, 15);
    }
}
