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
/// redraw path and replaying the bytes through a vt100 emulator.
///
/// `Term::read_write_pair` is `#[cfg(unix)]` in the console crate. CI also
/// runs on Windows, where there is simply no test seam — the redraw bugs
/// these cover were reported on Linux/macOS and the fixes are
/// platform-agnostic, so unix-only coverage is sufficient.
#[cfg(unix)]
pub use capture::{capture_term, snapshot};

#[cfg(unix)]
mod capture {
    use console::Term;
    use std::fs::{File, OpenOptions};
    use std::io::Write;
    use std::os::fd::{AsRawFd, RawFd};
    use std::sync::{Arc, Mutex};

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
}
