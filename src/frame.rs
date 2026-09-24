use std::io;

use console::Term;

/// The frame a widget last drew, so the next one can be drawn by rewriting
/// only what changed.
///
/// Every frame is drawn with the cursor left where the frame ends. To
/// replace it, the cursor moves up to the first line that differs, clears
/// the rows the old frame occupied from there down, and writes the rest of
/// the new frame. Lines above the first change are left alone, and a frame
/// identical to the last one writes nothing at all — pressing ↓ on the
/// last option of a `Select` no longer repaints the whole prompt.
#[derive(Default)]
pub(crate) struct Frame {
    last: String,
    /// Terminal width the frame was drawn at.
    width: usize,
}

impl Frame {
    /// Draw `next` in place of the frame currently on screen.
    pub(crate) fn update(&mut self, term: &Term, next: String) -> io::Result<()> {
        let width = term.size().1 as usize;
        // A resize reflows what's on screen, so nothing of the old frame
        // can be kept — even when its text hasn't changed.
        let reflowed = !self.last.is_empty() && width != self.width;
        if let Some(patch) = patch(&self.last, &next, width, reflowed) {
            term.write_str(&patch)?;
            term.flush()?;
        }
        self.last = next;
        self.width = width;
        Ok(())
    }

    /// Whether drawing `next` would leave the screen exactly as it is. A
    /// resize reflows the frame even when its text is unchanged, so that
    /// only counts at the width it was drawn at.
    pub(crate) fn is_current(&self, term: &Term, next: &str) -> bool {
        !self.last.is_empty() && self.last == next && self.width == term.size().1 as usize
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.last.is_empty()
    }

    /// Whether the terminal has been resized since the frame was drawn.
    /// The terminal may have reflowed the frame's wrapped rows in ways it
    /// doesn't report, so row counts from before the resize can't be
    /// trusted to find it.
    pub(crate) fn resized(&self, term: &Term) -> bool {
        !self.last.is_empty() && self.width != term.size().1 as usize
    }

    /// Pretend the frame was drawn at `width`, to exercise resize handling
    /// with a terminal whose size can't change.
    #[cfg(test)]
    pub(crate) fn set_width(&mut self, width: usize) {
        self.width = width;
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
///
/// With `reflowed`, the terminal was resized since `prev` was drawn: the
/// whole frame is cleared and written again.
fn patch(prev: &str, next: &str, width: usize, reflowed: bool) -> Option<String> {
    if prev == next && !reflowed {
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
        .take_while(|(a, b)| !reflowed && a == b)
        .count();

    let rows_up: usize = prev_lines[unchanged..prev_lines.len() - 1]
        .iter()
        .map(|line| crate::height::rows_for(line, width))
        .sum();
    let kept: usize = next_lines[..unchanged].iter().map(|l| l.len() + 1).sum();

    // Clear only the rows the old frame occupied, like `clear_last_lines`:
    // clearing to the end of the screen would also take out anything
    // below the prompt that it doesn't own.
    let mut out = String::new();
    if rows_up > 0 {
        out.push_str(&format!("\x1b[{rows_up}A"));
        out.push_str(&"\x1b[2K\x1b[1B".repeat(rows_up));
    }
    out.push_str("\x1b[2K");
    if rows_up > 0 {
        out.push_str(&format!("\x1b[{rows_up}A"));
    }
    out.push('\r');
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

    /// What `patch` writes to move up `rows` rows, clearing each on the
    /// way down, and return to the first.
    fn clear(rows: usize) -> String {
        if rows == 0 {
            return "\x1b[2K\r".to_string();
        }
        format!(
            "\x1b[{rows}A{}\x1b[2K\x1b[{rows}A\r",
            "\x1b[2K\x1b[1B".repeat(rows)
        )
    }

    #[test]
    fn an_identical_frame_writes_nothing() {
        assert_eq!(patch("a\nb\n\x1b[0m", "a\nb\n\x1b[0m", 80, false), None);
    }

    #[test]
    fn the_first_frame_is_written_whole() {
        assert_eq!(patch("", "a\nb\n", 80, false).as_deref(), Some("a\nb\n"));
    }

    /// Only the lines from the first change down are rewritten.
    #[test]
    fn rewrites_from_the_first_changed_line() {
        let prev = "title\n❯ one\n  two\nhelp\n";
        let next = "title\n  one\n❯ two\nhelp\n";
        assert_eq!(
            patch(prev, next, 80, false).as_deref(),
            Some(format!("{}  one\n❯ two\nhelp\n", clear(3)).as_str())
        );
    }

    /// The rows to move up are physical: a changed line that wrapped takes
    /// all its rows with it.
    #[test]
    fn moves_up_past_wrapped_rows() {
        let prev = format!("title\n{}\n", "x".repeat(20));
        let next = format!("title\n{}\n", "y".repeat(20));
        assert_eq!(
            patch(&prev, &next, 8, false).as_deref(),
            Some(format!("{}{}\n", clear(3), "y".repeat(20)).as_str())
        );
    }

    /// A change only on the cursor's row needs no movement up.
    #[test]
    fn a_change_on_the_cursor_row_stays_put() {
        assert_eq!(
            patch("a\n/ Loading", "a\n- Loading", 80, false).as_deref(),
            Some(format!("{}- Loading", clear(0)).as_str())
        );
    }

    /// After a resize, an unchanged frame is still rewritten in full: the
    /// old one has reflowed, and keeping any of it would misplace the
    /// cursor for widgets like `Input` that move it afterwards.
    #[test]
    fn a_reflowed_frame_is_rewritten_whole() {
        let frame = "title\n❯ one\n";
        assert_eq!(
            patch(frame, frame, 80, true).as_deref(),
            Some(format!("{}{frame}", clear(2)).as_str())
        );
    }

    #[test]
    fn a_shorter_frame_clears_what_it_no_longer_covers() {
        let out = patch("a\nb\nc\n", "a\n", 80, false).unwrap();
        assert_eq!(out, clear(2));
    }

    #[test]
    fn restores_the_color_the_skipped_lines_left_set() {
        let prev = "\x1b[0m\x1b[1mtitle\n\x1b[32mone\n";
        let next = "\x1b[0m\x1b[1mtitle\n\x1b[32mtwo\n";
        // The change is on line 1, which starts with its own color, but
        // the bold from line 0 is still in effect.
        assert_eq!(
            patch(prev, next, 80, false).as_deref(),
            Some(format!("{}\x1b[1m\x1b[32mtwo\n", clear(1)).as_str())
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

    /// A patch clears only the rows the old frame occupied: text below
    /// the prompt, which it doesn't own, survives the redraw.
    #[cfg(unix)]
    #[test]
    fn leaves_content_below_the_frame_alone() {
        use crate::test::{Parser, replay};

        let prev = "title\n❯ one\n  two\n";
        let next = "title\n  one\n❯ two\n";
        let mut parser = Parser::new(10, 20, 0);
        replay(&mut parser, prev.as_bytes());
        // Something else wrote two rows further down, then put the cursor
        // back where the frame left it.
        replay(&mut parser, b"\x1b7\x1b[2Bkeep me\x1b8");
        replay(
            &mut parser,
            patch(prev, next, 20, false).unwrap().as_bytes(),
        );

        let screen = parser.screen().contents();
        assert!(screen.contains("❯ two"), "{screen}");
        assert!(screen.contains("keep me"), "{screen}");
    }
}
