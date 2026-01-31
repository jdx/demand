use std::io::{self, BufRead, IsTerminal, Write};

/// Check if both stdin and stderr are terminals
pub fn is_tty() -> bool {
    io::stdin().is_terminal() && io::stderr().is_terminal()
}

/// Write a simple text prompt to stderr (without terminal control sequences)
pub fn write_prompt(title: &str, description: &str, prompt: &str) -> io::Result<()> {
    let mut stderr = io::stderr();

    if !title.is_empty() {
        writeln!(stderr, "{}", title)?;
    }
    if !description.is_empty() {
        writeln!(stderr, "{}", description)?;
    }
    if !prompt.is_empty() {
        write!(stderr, "{}", prompt)?;
    }

    stderr.flush()
}

/// Read a line from stdin and strip line endings
pub fn read_line() -> io::Result<String> {
    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line)?;

    // Remove trailing line endings (handles both \n and \r\n for Windows)
    let mut input = line.as_str();
    if let Some(stripped) = input.strip_suffix('\n') {
        input = stripped;
    }
    if let Some(stripped) = input.strip_suffix('\r') {
        input = stripped;
    }

    Ok(input.to_string())
}
