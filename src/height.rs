/// Number of physical terminal rows a rendered frame occupies in a
/// terminal `width` columns wide.
///
/// Widgets redraw by clearing the previous frame with
/// `Term::clear_last_lines`, which counts *physical* rows. Counting
/// logical lines instead leaves the wrapped remainder of any over-wide
/// line on screen, and the next frame draws below the leftovers — so the
/// prompt appears to duplicate itself, once per keypress.
///
/// What's counted is the rows *above* the cursor, since that's where a
/// frame leaves it: everything before the final newline. The text after
/// that newline — the color reset every widget writes last, or nothing at
/// all — is the row the cursor rests on, not a row to clear. Splitting on
/// `'\n'` rather than using `lines()` makes both endings behave the same;
/// `lines()` swallows a trailing empty segment and would lose a row when
/// the reset is absent.
pub(crate) fn rendered_height(output: &str, width: usize) -> usize {
    let mut lines: Vec<&str> = output.split('\n').collect();
    // Whatever follows the last newline is the row the cursor rests on.
    lines.pop();
    lines.iter().map(|line| rows_for(line, width)).sum()
}

/// Every row a frame occupies, including the one the cursor rests on.
///
/// For widgets whose frame ends without a newline — the spinner writes a
/// single line and leaves the cursor at the end of it — the last row is
/// part of the frame and has to be cleared like any other.
pub(crate) fn rendered_rows(output: &str, width: usize) -> usize {
    output.split('\n').map(|line| rows_for(line, width)).sum()
}

/// Rows one logical line wraps into. A line exactly `width` wide still
/// occupies a single row — terminals defer the wrap until the next
/// character arrives.
fn rows_for(line: &str, width: usize) -> usize {
    let printed = console::measure_text_width(line);
    if width == 0 || printed <= width {
        1
    } else {
        printed.div_ceil(width)
    }
}

#[cfg(test)]
mod tests {
    use super::rendered_height;

    /// The trailing reset fragment is not a row.
    #[test]
    fn drops_the_trailing_reset_fragment() {
        assert_eq!(rendered_height("title\noption\n\x1b[0m", 80), 2);
    }

    #[test]
    fn counts_a_line_that_fits_as_one_row() {
        assert_eq!(rendered_height("12345678\n\x1b[0m", 8), 1);
    }

    #[test]
    fn counts_the_rows_an_over_wide_line_wraps_into() {
        // 9 and 17 columns in an 8-column terminal: two rows, then three.
        assert_eq!(rendered_height("123456789\n\x1b[0m", 8), 2);
        assert_eq!(
            rendered_height(&format!("{}\n\x1b[0m", "x".repeat(17)), 8),
            3
        );
    }

    /// Color codes are not printed, so they must not push a line into a
    /// second row.
    #[test]
    fn measures_printed_width_not_byte_length() {
        let colored = format!("\x1b[38;5;252m{}\x1b[0m\n\x1b[0m", "x".repeat(8));
        assert_eq!(rendered_height(&colored, 8), 1);
    }

    /// An unknown terminal width can't wrap anything, so fall back to the
    /// old logical-line count rather than dividing by zero.
    #[test]
    fn treats_zero_width_as_one_row_per_line() {
        assert_eq!(rendered_height("aaaa\nbbbb\n\x1b[0m", 0), 2);
    }

    /// A frame with no trailing newline — the spinner — is entirely above
    /// nothing: `rendered_height` sees no rows to clear, `rendered_rows`
    /// counts the row the cursor is sitting in.
    #[test]
    fn rendered_rows_counts_a_frame_the_cursor_sits_inside() {
        use super::rendered_rows;
        assert_eq!(rendered_height("/ Loading", 80), 0);
        assert_eq!(rendered_rows("/ Loading", 80), 1);
        assert_eq!(rendered_rows(&format!("/ {}", "x".repeat(100)), 80), 2);
    }
}
