use std::borrow::Cow;

#[ctor::ctor(unsafe)]
fn init() {
    console::set_colors_enabled(false);
    console::set_colors_enabled_stderr(false);
}

pub fn without_ansi(s: &str) -> Cow<'_, str> {
    console::strip_ansi_codes(s)
}

/// A `Term` whose output is captured in memory, for driving a widget's
/// redraw path and replaying the bytes through a terminal emulator.
///
/// `Term::read_write_pair` is `#[cfg(unix)]` in the console crate. CI also
/// runs on Windows, where there is simply no test seam — the redraw bugs
/// these cover were reported on Linux/macOS and the fixes are
/// platform-agnostic, so unix-only coverage is sufficient.
#[cfg(unix)]
pub use capture::{Parser, capture_term, replay, snapshot};

#[cfg(unix)]
mod capture {
    use console::Term;
    use rio_vt::ansi::CursorShape;
    use rio_vt::crosswords::formatter::FormatOptions;
    use rio_vt::crosswords::{Crosswords, CrosswordsSize};
    use rio_vt::event::{VoidListener, WindowId};
    use rio_vt::performer::handler::Processor;
    use std::fs::{File, OpenOptions};
    use std::io::Write;
    use std::os::fd::{AsRawFd, RawFd};
    use std::sync::{Arc, Mutex};

    /// Minimal vt100-shaped adapter over rio-vt: feed bytes with `process`
    /// and read the screen back through `screen()`.
    pub struct Parser {
        term: Crosswords<VoidListener>,
        processor: Processor,
    }

    impl Parser {
        pub fn new(rows: u16, cols: u16, _scrollback: usize) -> Self {
            Self {
                term: Crosswords::new(
                    CrosswordsSize::new(cols as usize, rows as usize),
                    CursorShape::Block,
                    VoidListener,
                    WindowId::from(0),
                    0,
                    0,
                ),
                processor: Processor::default(),
            }
        }

        pub fn process(&mut self, bytes: &[u8]) {
            self.processor.advance(&mut self.term, bytes);
        }

        pub fn screen(&self) -> Screen<'_> {
            Screen { term: &self.term }
        }
    }

    pub struct Screen<'a> {
        term: &'a Crosswords<VoidListener>,
    }

    impl Screen<'_> {
        pub fn contents(&self) -> String {
            self.term.format(FormatOptions::plain())
        }

        pub fn cursor_position(&self) -> (u16, u16) {
            let pos = self.term.cursor().pos;
            (pos.row.0 as u16, pos.col.0 as u16)
        }
    }

    /// `Term::read_write_pair` is bounded by `Write + Debug + AsRawFd + Send
    /// + 'static`. It only uses the fd for its own `AsRawFd` impl — the
    /// actual I/O goes through `Write::write_all` — so we can satisfy the
    /// trait by holding a throwaway `/dev/null` handle and capture bytes
    /// into a shared `Vec` without any kernel round-trip.
    #[derive(Debug)]
    pub struct CaptureWriter {
        _fd: File,
        bytes: Arc<Mutex<Vec<u8>>>,
    }

    impl Write for CaptureWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.bytes.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl AsRawFd for CaptureWriter {
        fn as_raw_fd(&self) -> RawFd {
            self._fd.as_raw_fd()
        }
    }

    pub fn capture_term() -> (Term, Arc<Mutex<Vec<u8>>>) {
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let writer = CaptureWriter {
            _fd: OpenOptions::new().write(true).open("/dev/null").unwrap(),
            bytes: Arc::clone(&bytes),
        };
        let reader = File::open("/dev/null").unwrap();
        (Term::read_write_pair(reader, writer), bytes)
    }

    pub fn snapshot(buf: &Arc<Mutex<Vec<u8>>>) -> Vec<u8> {
        buf.lock().unwrap().clone()
    }

    /// Replay captured terminal output with the newline translation a real
    /// Unix TTY performs. The emulator does not apply ONLCR itself, so feeding
    /// raw `\n` bytes would leave the cursor column unchanged and make redraw
    /// assertions model a pipe rather than a terminal.
    pub fn replay(parser: &mut Parser, output: &[u8]) {
        let mut tty_output = Vec::with_capacity(output.len());
        for &byte in output {
            if byte == b'\n' {
                tty_output.push(b'\r');
            }
            tty_output.push(byte);
        }
        parser.process(&tty_output);
    }
}
