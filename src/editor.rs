use std::ffi::OsString;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use console::{Key, Term};
use termcolor::{Buffer, WriteColor};

use crate::theme::Theme;
use crate::{ctrlc, theme};

/// Multi-line text input, entered in the user's text editor
///
/// Shows the title and a preview of the text so far. `e` opens the text in
/// `$VISUAL` or `$EDITOR` (falling back to `vi`, or `notepad` on Windows),
/// and saving and closing the editor brings the user back to the prompt
/// with the new text. `enter` submits it.
///
/// Without a terminal, the text is read from stdin until it ends instead.
///
/// # Example
/// ```no_run
/// use demand::Editor;
///
/// let description = Editor::new("Description")
///     .description("What does this task involve?")
///     .default_value("TODO: \n")
///     .extension("md")
///     .run()
///     .expect("error running editor");
/// ```
pub struct Editor<'a> {
    /// The title of the prompt
    pub title: String,
    /// A description to display after the title
    pub description: String,
    /// The colors/style of the prompt
    pub theme: &'a Theme,
    /// Lines of the text shown in the preview before it's cut off
    pub preview_lines: usize,

    text: String,
    extension: String,
    command: Option<OsString>,
    term: Term,
    frame: crate::frame::Frame,
    err: Option<String>,
}

impl<'a> Editor<'a> {
    /// Create a new editor prompt with the given title
    pub fn new<S: Into<String>>(title: S) -> Self {
        Self {
            title: title.into(),
            description: String::new(),
            theme: &*theme::DEFAULT,
            preview_lines: 5,
            text: String::new(),
            extension: "txt".to_string(),
            command: None,
            term: Term::stderr(),
            frame: Default::default(),
            err: None,
        }
    }

    /// Set the description of the prompt
    pub fn description(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }

    /// Set the text the editor starts with
    pub fn default_value(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self
    }

    /// Set the extension of the file the text is edited in, without the
    /// dot. Editors use it to pick syntax highlighting. Defaults to `txt`.
    pub fn extension(mut self, extension: &str) -> Self {
        self.extension = extension.trim_start_matches('.').to_string();
        self
    }

    /// Set the editor to run instead of `$VISUAL` / `$EDITOR`. It may
    /// include arguments, separated by whitespace, like `code --wait`.
    pub fn editor_command(mut self, command: impl Into<OsString>) -> Self {
        self.command = Some(command.into());
        self
    }

    /// Set the number of lines of text shown in the preview
    pub fn preview_lines(mut self, lines: usize) -> Self {
        self.preview_lines = lines;
        self
    }

    /// Set the theme of the prompt
    pub fn theme(mut self, theme: &'a Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Displays the prompt to the user and returns the text they entered
    ///
    /// This function will block until the user submits the text. If the user cancels,
    /// an error of type `io::ErrorKind::Interrupted` is returned.
    pub fn run(mut self) -> io::Result<String> {
        if !crate::tty::is_tty() {
            crate::tty::write_prompt(&self.title, &self.description, "")?;
            let mut text = String::new();
            io::stdin().read_to_string(&mut text)?;
            return Ok(text);
        }

        let ctrlc_handle = ctrlc::show_cursor_after_ctrlc(&self.term)?;
        self.term.hide_cursor()?;
        loop {
            let term = self.term.clone();
            crate::synchronized_output::run(&term, || self.draw())?;
            match self.term.read_key()? {
                Key::Char('e') => {
                    // The editor needs the whole terminal: take the prompt
                    // down and hand over a visible cursor until it exits.
                    self.clear()?;
                    self.term.show_cursor()?;
                    let edited = self.edit();
                    self.term.hide_cursor()?;
                    match edited {
                        Ok(text) => {
                            self.text = text;
                            self.err = None;
                        }
                        Err(err) => self.err = Some(err.to_string()),
                    }
                }
                Key::Enter => {
                    ctrlc_handle.close();
                    let term = self.term.clone();
                    crate::synchronized_output::run(&term, || {
                        self.clear()?;
                        self.term.show_cursor()?;
                        let output = self.render_success()?;
                        self.term.write_all(output.as_bytes())
                    })?;
                    return Ok(self.text);
                }
                Key::Escape => {
                    self.clear()?;
                    self.term.show_cursor()?;
                    ctrlc_handle.close();
                    return Err(io::Error::new(io::ErrorKind::Interrupted, "user cancelled"));
                }
                _ => {}
            }
        }
    }

    /// Write the text to a temporary file, open it in the editor, and read
    /// it back once the editor exits.
    fn edit(&self) -> io::Result<String> {
        let file = TempFile::create(&self.extension, &self.text)?;
        let command = self.command.clone().unwrap_or_else(default_editor);
        let mut child = editor_command(&command, file.path())?;
        // The prompt is on stderr so callers can capture stdout, as in
        // `notes=$(my-cli)`. The editor must not inherit that capture, or
        // its UI ends up in the caller's output: give it the terminal.
        // Without `/dev/tty`, stderr is the terminal (the prompt only runs
        // when it is), so the editor's output goes there instead.
        #[cfg(unix)]
        let tty = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/tty")
            .ok();
        #[cfg(not(unix))]
        let tty: Option<fs::File> = None;
        match tty {
            Some(tty) => {
                child
                    .stdin(tty.try_clone()?)
                    .stdout(tty.try_clone()?)
                    .stderr(tty);
            }
            None => {
                child.stdout(Stdio::from(io::stderr()));
            }
        }
        let status = child.status().map_err(|err| {
            io::Error::new(
                err.kind(),
                format!("could not run editor {}: {err}", command.to_string_lossy()),
            )
        })?;
        if !status.success() {
            return Err(io::Error::other(format!(
                "editor {} exited with {status}",
                command.to_string_lossy()
            )));
        }
        fs::read_to_string(file.path())
    }

    fn render(&self) -> io::Result<String> {
        let mut out = Buffer::ansi();

        out.set_color(&self.theme.title)?;
        writeln!(out, "{}", self.title)?;
        if !self.description.is_empty() {
            out.set_color(&self.theme.description)?;
            writeln!(out, "{}", self.description)?;
        }

        let lines: Vec<&str> = self.text.lines().collect();
        if lines.is_empty() {
            out.set_color(&self.theme.input_placeholder)?;
            writeln!(out, "  (empty)")?;
        } else {
            out.set_color(&self.theme.unselected_option)?;
            for line in lines.iter().take(self.preview_lines) {
                writeln!(out, "  {line}")?;
            }
            if lines.len() > self.preview_lines {
                out.set_color(&self.theme.description)?;
                writeln!(out, "  … {} more lines", lines.len() - self.preview_lines)?;
            }
        }

        if let Some(err) = &self.err {
            out.set_color(&self.theme.error_indicator)?;
            writeln!(out, "✗ {err}")?;
        }

        writeln!(out)?;
        for (i, (key, desc)) in [("e", "edit"), ("enter", "submit"), ("esc", "cancel")]
            .iter()
            .enumerate()
        {
            if i > 0 {
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

    fn render_success(&self) -> io::Result<String> {
        let mut out = Buffer::ansi();
        out.set_color(&self.theme.title)?;
        write!(out, "{}", self.title)?;
        out.set_color(&self.theme.selected_option)?;
        let mut lines = self.text.lines();
        let first = lines.next().unwrap_or_default();
        match lines.count() {
            0 => writeln!(out, " {first}")?,
            n => writeln!(out, " {first} (+{n} more lines)")?,
        }
        out.reset()?;
        Ok(std::str::from_utf8(out.as_slice()).unwrap().to_string())
    }

    /// Render a frame and draw it over the previous one, rewriting only
    /// the lines that changed.
    fn draw(&mut self) -> io::Result<()> {
        let output = self.render()?;
        self.frame.update(&self.term, output)
    }

    fn clear(&mut self) -> io::Result<()> {
        self.frame.clear(&self.term)?;
        Ok(())
    }
}

/// `$VISUAL`, then `$EDITOR`, then the platform's stock editor.
fn default_editor() -> OsString {
    ["VISUAL", "EDITOR"]
        .iter()
        .filter_map(std::env::var_os)
        .find(|v| !v.is_empty())
        .unwrap_or_else(|| {
            if cfg!(windows) {
                "notepad".into()
            } else {
                "vi".into()
            }
        })
}

/// The command to open `path` in `editor`. Like git, the editor setting may
/// carry arguments (`code --wait`), so it's split into words; single or
/// double quotes keep a word with spaces together, as in
/// `"C:\Program Files\Editor\edit.exe" --wait`.
fn editor_command(editor: &OsString, path: &Path) -> io::Result<Command> {
    let words = split_words(&editor.to_string_lossy());
    let (program, args) = words
        .split_first()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no editor configured"))?;
    let mut command = Command::new(program);
    command.args(args).arg(path);
    Ok(command)
}

/// Split on whitespace into words. A word that starts with a single or
/// double quote and ends with the same quote — the closing one followed by
/// whitespace or the end — is taken as the text between them, spaces and
/// all, as in `"C:\\Program Files\\Editor\\edit.exe" --wait` or
/// `vim -c 'set tw=72'`. Any other quote is an ordinary character, so an
/// apostrophe inside a path (`/home/o'brien/bin/edit`) needs no quoting
/// and can't pair with a quote later in the command. Backslashes are kept
/// as they are, since they separate Windows paths.
fn split_words(s: &str) -> Vec<String> {
    let chars: Vec<char> = s.chars().collect();
    let mut words = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }
        let quote = chars[i];
        if quote == '"' || quote == '\'' {
            let close = (i + 1..chars.len())
                .find(|&j| chars[j] == quote && chars.get(j + 1).is_none_or(|c| c.is_whitespace()));
            if let Some(close) = close {
                words.push(chars[i + 1..close].iter().collect());
                i = close + 1;
                continue;
            }
        }
        let end = (i..chars.len())
            .find(|&j| chars[j].is_whitespace())
            .unwrap_or(chars.len());
        words.push(chars[i..end].iter().collect());
        i = end;
    }
    words
}

/// A file in the temp dir that is removed when dropped.
struct TempFile {
    path: PathBuf,
}

impl TempFile {
    fn create(extension: &str, contents: &str) -> io::Result<Self> {
        static COUNT: AtomicUsize = AtomicUsize::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or_default();
        let path = std::env::temp_dir().join(format!(
            "demand-{}-{nanos}-{}.{extension}",
            std::process::id(),
            COUNT.fetch_add(1, Ordering::Relaxed),
        ));
        // `create_new` refuses to follow or reuse anything already at the
        // path, so another user can't plant a file there first, and on Unix
        // the file is readable only by its owner: the temp dir is usually
        // shared and the text stays there while the editor is open.
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        let mut file = options.open(&path)?;
        // Take ownership before writing, so a failed write still removes
        // the file.
        let temp = Self { path };
        file.write_all(contents.as_bytes())?;
        Ok(temp)
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::without_ansi;
    use indoc::indoc;

    #[test]
    fn test_render_empty() {
        let editor = Editor::new("Notes").description("Anything else?");
        assert_eq!(
            indoc! {"
                Notes
                Anything else?
                  (empty)

                e edit • enter submit • esc cancel
            "},
            without_ansi(&editor.render().unwrap())
        );
    }

    #[test]
    fn test_render_preview_is_cut_off() {
        let editor = Editor::new("Notes")
            .default_value("one\ntwo\nthree\nfour\n")
            .preview_lines(2);
        assert_eq!(
            indoc! {"
                Notes
                  one
                  two
                  … 2 more lines

                e edit • enter submit • esc cancel
            "},
            without_ansi(&editor.render().unwrap())
        );
    }

    #[test]
    fn test_render_success() {
        let editor = Editor::new("Notes").default_value("first\nsecond\nthird\n");
        assert_eq!(
            "Notes first (+2 more lines)\n",
            without_ansi(&editor.render_success().unwrap())
        );
        let editor = Editor::new("Notes").default_value("only");
        assert_eq!(
            "Notes only\n",
            without_ansi(&editor.render_success().unwrap())
        );
    }

    #[test]
    fn editor_command_splits_arguments() {
        let command = editor_command(&"code --wait".into(), Path::new("/tmp/x.md")).unwrap();
        assert_eq!(command.get_program(), "code");
        let args: Vec<_> = command.get_args().collect();
        assert_eq!(args, ["--wait", "/tmp/x.md"]);
        assert!(editor_command(&"  ".into(), Path::new("x")).is_err());
    }

    #[test]
    fn editor_command_keeps_quoted_paths_together() {
        let command = editor_command(
            &r#""C:\Program Files\Editor\edit.exe" --wait"#.into(),
            Path::new("x.md"),
        )
        .unwrap();
        assert_eq!(command.get_program(), r"C:\Program Files\Editor\edit.exe");
        let args: Vec<_> = command.get_args().collect();
        assert_eq!(args, ["--wait", "x.md"]);
        assert_eq!(
            split_words("vim -c 'set tw=72'"),
            ["vim", "-c", "set tw=72"]
        );
        // A lone apostrophe is part of the word, not an opening quote.
        assert_eq!(
            split_words("/home/o'brien/bin/editor --wait"),
            ["/home/o'brien/bin/editor", "--wait"]
        );
        assert_eq!(
            split_words(r#""/home/o'brien/edit" -w"#),
            ["/home/o'brien/edit", "-w"]
        );
        // An apostrophe mid-word can't pair with a quote later on and
        // swallow the words between them.
        assert_eq!(
            split_words("/home/o'brien/bin/edit -c 'set tw=72'"),
            ["/home/o'brien/bin/edit", "-c", "set tw=72"]
        );
        assert_eq!(
            split_words("/home/o'brien/it's/edit --wait"),
            ["/home/o'brien/it's/edit", "--wait"]
        );
        // A quote only closes where a word ends.
        assert_eq!(split_words(r#""a b"c d"#), [r#""a"#, r#"b"c"#, "d"]);
        assert_eq!(split_words("  '' x "), ["", "x"]);
    }

    #[cfg(unix)]
    #[test]
    fn temp_file_is_private_to_its_owner() {
        use std::os::unix::fs::PermissionsExt;
        let file = TempFile::create("txt", "secret").unwrap();
        let mode = fs::metadata(file.path()).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    /// Round-trips through a real process standing in for the editor: it
    /// appends a line to the file it's given.
    #[cfg(unix)]
    #[test]
    fn edit_returns_what_the_editor_saved() {
        let script = TempFile::create("sh", "echo world >> \"$1\"\n").unwrap();
        let editor = Editor::new("Notes")
            .default_value("hello\n")
            .editor_command(format!("sh {}", script.path().display()));
        assert_eq!(editor.edit().unwrap(), "hello\nworld\n");
    }

    #[cfg(unix)]
    #[test]
    fn edit_reports_an_editor_that_fails() {
        let editor = Editor::new("Notes").editor_command("false");
        let err = editor.edit().unwrap_err();
        assert!(err.to_string().contains("exited with"), "{err}");
    }

    #[test]
    fn temp_file_is_removed_on_drop() {
        let file = TempFile::create("txt", "x").unwrap();
        let path = file.path().to_path_buf();
        assert!(path.exists());
        drop(file);
        assert!(!path.exists());
    }
}
