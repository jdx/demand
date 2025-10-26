use std::io::Write;
use std::process::{Command, Output, Stdio};

/// Helper to check if output contains terminal control sequences
fn has_escape_sequences(text: &str) -> bool {
    text.contains('\x1b')
}

/// Helper to run an example with piped I/O and return the output
fn run_example_with_input(example: &str, input: &[u8]) -> Output {
    let mut child = Command::new("cargo")
        .args(&["run", "--quiet", "--example", example])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn example");

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input).expect("Failed to write to stdin");
    }

    child
        .wait_with_output()
        .expect("Failed to wait for example")
}

/// Helper to assert no escape sequences in output
fn assert_no_escape_sequences(output: &Output) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !has_escape_sequences(&stderr),
        "stderr should not contain terminal escape sequences in non-TTY mode, got: {}",
        stderr
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !has_escape_sequences(&stdout),
        "stdout should not contain terminal escape sequences in non-TTY mode, got: {}",
        stdout
    );
}

#[test]
fn test_piped_stdin() {
    // Run the input example with piped stdin (non-TTY)
    // The example has validation requiring 8+ chars, so use a valid value
    let output = run_example_with_input("input", b"TestValue123\n");

    // Should exit successfully (validation passed)
    assert!(
        output.status.success(),
        "Example should exit successfully with valid input, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_piped_stdin_validation_failure() {
    // Test that validation errors are handled correctly with piped input
    let output = run_example_with_input("input", b"short\n");

    // Should fail validation
    assert!(
        !output.status.success(),
        "Example should fail with invalid input (too short)"
    );
}

#[test]
fn test_piped_stdin_windows_line_endings() {
    // Test that Windows line endings (\r\n) are handled correctly
    let output = run_example_with_input("input", b"TestValue123\r\n");

    // Should exit successfully (validation passed, \r stripped correctly)
    assert!(
        output.status.success(),
        "Example should handle Windows line endings correctly, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_input_no_escape_sequences_in_non_tty() {
    // Verify that Input doesn't output terminal control sequences when stderr is not a TTY
    let output = run_example_with_input("input", b"TestValue123\n");

    assert_no_escape_sequences(&output);
    assert!(
        output.status.success(),
        "Example should exit successfully, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_confirm_no_escape_sequences_in_non_tty() {
    // Verify that Confirm doesn't output terminal control sequences when stderr is not a TTY
    let output = run_example_with_input("confirm", b"y\n");

    assert_no_escape_sequences(&output);
    assert!(
        output.status.success(),
        "Example should exit successfully, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_input_prompt_visible_in_non_tty() {
    // Verify that the prompt is still visible in non-TTY mode
    let output = run_example_with_input("input", b"TestValue123\n");

    // Check that stderr contains the prompt text (without escape sequences)
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("What's your name?"),
        "stderr should contain the prompt text, got: {}",
        stderr
    );

    assert!(output.status.success(), "Example should exit successfully");
}
