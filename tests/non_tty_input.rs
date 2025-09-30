use std::io::Write;
use std::process::{Command, Stdio};

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
