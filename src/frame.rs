use std::io;

use console::Term;

/// The frame a widget last drew, so the next one can be drawn by rewriting
/// only what changed.
///
/// Every frame is drawn with the cursor left where the frame ends. To
/// replace it, the cursor moves up to the first line that differs, clears
/// from there to the end of the screen, and writes the rest of the new
/// frame. Lines above the first change are left alone, and a frame
/// identical to the last one writes nothing at all — pressing ↓ on the
/// last option of a `Select` no longer repaints the whole prompt.
#[derive(Default)]
pub(crate) struct Frame {
    last: String,
}

impl Frame {
    /// Draw `next` in place of the frame currently on screen.
    pub(crate) fn update(&mut self, term: &Term, next: String) -> io::Result<()> {
        let width = term.size().1 as usize;
        if let Some(patch) = patch(&self.last, &next, width) {
            term.write_str(&patch)?;
            term.flush()?;
        }
        self.last = next;
        Ok(())
    }

    /// Whether drawing `next` would change anything on screen.
    pub(crate) fn is_current(&self, next: &str) -> bool {
        !self.last.is_empty() && self.last == next
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.last.is_empty()
    }

    /// Erase the frame from the screen.
    pub(crate) fn clear(&mut self, term: &Term) -> io::Result<()> {
        term.clear_last_lines(self.height(term))?;
        self.last.clear();
        Ok(())
    }

    /// Forget the frame without erasing it, for when the screen was cleared
    /// some other way and nothing of it is left.
    pub(crate) fn forget(&mut self) {
        self.last.clear();
    }

    /// Rows the frame occupies above the cursor.
    pub(crate) fn height(&self, term: &Term) -> usize {
        crate::height::rendered_height(&self.last, term.size().1 as usize)
    }
}

/// Bytes that turn `prev`, drawn with the cursor at its end, into `next`.
/// `None` when the two are identical and there is nothing to do.
fn patch(prev: &str, next: &str, width: usize) -> Option<String> {
    if prev == next {
        return None;
    }
    if prev.is_empty() {
        return Some(next.to_string());
    }

    let prev_lines: Vec<&str> = prev.split('\n').collect();
    let next_lines: Vec<&str> = next.split('\n').collect();
    // The last segment is the row the cursor rests on: it can be compared
    // like the others, but it is never a row to move up past.
    let unchanged = prev_lines
        .iter()
        .zip(&next_lines)
        .take(prev_lines.len().min(next_lines.len()) - 1)
        .take_while(|(a, b)| a == b)
        .count();

    let rows_up: usize = prev_lines[unchanged..prev_lines.len() - 1]
        .iter()
        .map(|line| crate::height::rows_for(line, width))
        .sum();
    let kept: usize = next_lines[..unchanged].iter().map(|l| l.len() + 1).sum();

    let mut out = String::new();
    if rows_up > 0 {
        out.push_str(&format!("\x1b[{rows_up}A"));
    }
    out.push_str("\r\x1b[J");
    // Colors carry across newlines, so a rewrite starting mid-frame has to
    // restore whatever the skipped lines left set.
    out.push_str(&active_style(&next[..kept]));
    out.push_str(&next[kept..]);
    Some(out)
}

/// The SGR sequences still in effect at the end of `s`: everything since
/// the last reset.
fn active_style(s: &str) -> String {
    let mut style = String::new();
    let mut rest = s;
    while let Some(start) = rest.find("\x1b[") {
        rest = &rest[start..];
        let Some(len) = rest[2..]
            .find(|c: char| !(c.is_ascii_digit() || c == ';'))
            .map(|i| i + 2)
        else {
            break;
        };
        if rest[len..].starts_with('m') {
            let seq = &rest[..=len];
            if seq == "\x1b[0m" || seq == "\x1b[m" {
                style.clear();
            } else {
                style.push_str(seq);
            }
            rest = &rest[len + 1..];
        } else {
            rest = &rest[len..];
        }
    }
    style
}

#[cfg(test)]
mod tests {
    use super::{active_style, patch};

    #[test]
    fn an_identical_frame_writes_nothing() {
        assert_eq!(patch("a\nb\n\x1b[0m", "a\nb\n\x1b[0m", 80), None);
    }

    #[test]
    fn the_first_frame_is_written_whole() {
        assert_eq!(patch("", "a\nb\n", 80).as_deref(), Some("a\nb\n"));
    }

    /// Only the lines from the first change down are rewritten.
    #[test]
    fn rewrites_from_the_first_changed_line() {
        let prev = "title\n❯ one\n  two\nhelp\n";
        let next = "title\n  one\n❯ two\nhelp\n";
        assert_eq!(
            patch(prev, next, 80).as_deref(),
            Some("\x1b[3A\r\x1b[J  one\n❯ two\nhelp\n")
        );
    }

    /// The rows to move up are physical: a changed line that wrapped takes
    /// all its rows with it.
    #[test]
    fn moves_up_past_wrapped_rows() {
        let prev = format!("title\n{}\n", "x".repeat(20));
        let next = format!("title\n{}\n", "y".repeat(20));
        assert_eq!(
            patch(&prev, &next, 8).as_deref(),
            Some(format!("\x1b[3A\r\x1b[J{}\n", "y".repeat(20)).as_str())
        );
    }

    /// A change only on the cursor's row needs no movement up.
    #[test]
    fn a_change_on_the_cursor_row_stays_put() {
        assert_eq!(
            patch("a\n/ Loading", "a\n- Loading", 80).as_deref(),
            Some("\r\x1b[J- Loading")
        );
    }

    #[test]
    fn a_shorter_frame_clears_what_it_no_longer_covers() {
        let out = patch("a\nb\nc\n", "a\n", 80).unwrap();
        assert_eq!(out, "\x1b[2A\r\x1b[J");
    }

    #[test]
    fn restores_the_color_the_skipped_lines_left_set() {
        let prev = "\x1b[0m\x1b[1mtitle\n\x1b[32mone\n";
        let next = "\x1b[0m\x1b[1mtitle\n\x1b[32mtwo\n";
        // The change is on line 1, which starts with its own color, but
        // the bold from line 0 is still in effect.
        assert_eq!(
            patch(prev, next, 80).as_deref(),
            Some("\x1b[1A\r\x1b[J\x1b[1m\x1b[32mtwo\n")
        );
    }

    #[test]
    fn active_style_resets() {
        assert_eq!(
            active_style("\x1b[1mx\x1b[0my\x1b[38;5;2mz"),
            "\x1b[38;5;2m"
        );
        assert_eq!(active_style("plain"), "");
    }
}
