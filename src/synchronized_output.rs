use std::io;

use console::Term;

const BEGIN: &str = "\x1b[?2026h";
const END: &str = "\x1b[?2026l";

/// Run an in-place terminal update inside DEC private mode 2026.
///
/// Supporting terminals buffer the enclosed cursor movement, clearing, and
/// drawing, then present it as one frame. Terminals that do not implement the
/// mode ignore the unknown private-mode sequences.
pub(crate) fn run<T>(term: &Term, update: impl FnOnce() -> io::Result<T>) -> io::Result<T> {
    term.write_str(BEGIN)?;
    let guard = Guard {
        term: term.clone(),
        finished: false,
    };
    let result = update();
    let end_result = guard.finish();
    match result {
        Ok(value) => end_result.map(|()| value),
        Err(err) => Err(err),
    }
}

struct Guard {
    term: Term,
    finished: bool,
}

impl Guard {
    fn finish(mut self) -> io::Result<()> {
        self.finished = true;
        self.term.write_str(END)
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        if !self.finished {
            let _ = self.term.write_str(END);
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::test::{capture_term, snapshot};

    #[test]
    fn brackets_an_update() {
        let (term, bytes) = capture_term();

        run(&term, || term.write_str("frame")).unwrap();

        assert_eq!(snapshot(&bytes), b"\x1b[?2026hframe\x1b[?2026l");
    }

    #[test]
    fn closes_an_update_after_an_error() {
        let (term, bytes) = capture_term();

        let result = run::<()>(&term, || Err(io::Error::other("redraw failed")));

        assert_eq!(result.unwrap_err().to_string(), "redraw failed");
        assert_eq!(snapshot(&bytes), b"\x1b[?2026h\x1b[?2026l");
    }
}
