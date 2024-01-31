use console::Term;

/// Returns a tuple with the height of the output and the number of lines that overflow the terminal width
pub fn get_height(term: &Term, output: String) -> (usize, usize) {
    let max_width = term.size().1 as usize;
    let height = output.lines().count() - 1;
    // calculate potential overflow of lines overflowing terminal width
    let overflow = output
        .lines()
        .map(|l| {
            if l.chars().count() > max_width {
                l.chars().count() / max_width
            } else {
                0
            }
        })
        .sum::<usize>();
    (height, overflow)
}
