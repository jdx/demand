use std::io::Write;
use std::process::{Command, Stdio};

/// Helper to check if output contains terminal control sequences
fn has_escape_sequences(text: &str) -> bool {
    text.contains('\x1b')
}

#[test]
fn test_piped_stdin() {
    // Run the input example with piped stdin (non-TTY)
    // The example has validation requiring 8+ chars, so use a valid value
    let mut child = Command::new("cargo")
        .args(&["run", "--quiet", "--example", "input"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn example");

    // Write test value to stdin that passes validation (8+ chars)
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(b"TestValue123\n")
            .expect("Failed to write to stdin");
    }

    // Wait for completion and get output
    let output = child
        .wait_with_output()
        .expect("Failed to wait for example");

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
    let mut child = Command::new("cargo")
        .args(&["run", "--quiet", "--example", "input"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn example");

    // Write a value that fails validation (too short - needs 8+ chars)
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(b"short\n")
            .expect("Failed to write to stdin");
    }

    let output = child
        .wait_with_output()
        .expect("Failed to wait for example");

    // Should fail validation
    assert!(
        !output.status.success(),
        "Example should fail with invalid input (too short)"
    );
}

#[test]
fn test_piped_stdin_windows_line_endings() {
    // Test that Windows line endings (\r\n) are handled correctly
    let mut child = Command::new("cargo")
        .args(&["run", "--quiet", "--example", "input"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn example");

    // Write test value with Windows line ending (\r\n)
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(b"TestValue123\r\n")
            .expect("Failed to write to stdin");
    }

    let output = child
        .wait_with_output()
        .expect("Failed to wait for example");

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
    let mut child = Command::new("cargo")
        .args(&["run", "--quiet", "--example", "input"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn example");

    // Write test value to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(b"TestValue123\n")
            .expect("Failed to write to stdin");
    }

    let output = child
        .wait_with_output()
        .expect("Failed to wait for example");

    // Check that stderr doesn't contain escape sequences
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !has_escape_sequences(&stderr),
        "stderr should not contain terminal escape sequences in non-TTY mode, got: {}",
        stderr
    );

    // Check that stdout doesn't contain escape sequences
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !has_escape_sequences(&stdout),
        "stdout should not contain terminal escape sequences in non-TTY mode, got: {}",
        stdout
    );

    assert!(
        output.status.success(),
        "Example should exit successfully, stderr: {}",
        stderr
    );
}

#[test]
fn test_confirm_no_escape_sequences_in_non_tty() {
    // Verify that Confirm doesn't output terminal control sequences when stderr is not a TTY
    let mut child = Command::new("cargo")
        .args(&["run", "--quiet", "--example", "confirm"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn example");

    // Write "yes" to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(b"y\n")
            .expect("Failed to write to stdin");
    }

    let output = child
        .wait_with_output()
        .expect("Failed to wait for example");

    // Check that stderr doesn't contain escape sequences
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !has_escape_sequences(&stderr),
        "stderr should not contain terminal escape sequences in non-TTY mode, got: {}",
        stderr
    );

    // Check that stdout doesn't contain escape sequences
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !has_escape_sequences(&stdout),
        "stdout should not contain terminal escape sequences in non-TTY mode, got: {}",
        stdout
    );

    assert!(
        output.status.success(),
        "Example should exit successfully, stderr: {}",
        stderr
    );
}

#[test]
fn test_input_prompt_visible_in_non_tty() {
    // Verify that the prompt is still visible in non-TTY mode
    let mut child = Command::new("cargo")
        .args(&["run", "--quiet", "--example", "input"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn example");

    // Write test value to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(b"TestValue123\n")
            .expect("Failed to write to stdin");
    }

    let output = child
        .wait_with_output()
        .expect("Failed to wait for example");

    // Check that stderr contains the prompt text (without escape sequences)
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("What's your name?"),
        "stderr should contain the prompt text, got: {}",
        stderr
    );

    assert!(
        output.status.success(),
        "Example should exit successfully"
    );
}
