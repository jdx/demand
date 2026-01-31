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

#[test]
fn test_confirm_custom_labels_ambiguous_single_char() {
    // Test that single char 'c' is rejected when both labels start with 'c' (Confirm/Cancel)
    let output = run_example_with_input("confirm_custom", b"c\n");

    assert!(
        !output.status.success(),
        "Example should reject ambiguous 'c' when both labels start with 'c'"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Ambiguous input"),
        "Should show ambiguous input error, got: {}",
        stderr
    );
}

#[test]
fn test_confirm_custom_labels_reject_hardcoded_yes() {
    // Test that hardcoded "yes" is rejected when using custom labels
    let output = run_example_with_input("confirm_custom", b"yes\n");

    assert!(
        !output.status.success(),
        "Example should reject 'yes' when using custom labels (Confirm/Cancel)"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Invalid input"),
        "Should show invalid input error, got: {}",
        stderr
    );
}

#[test]
fn test_confirm_custom_labels_reject_hardcoded_y() {
    // Test that hardcoded "y" is rejected when using custom labels
    let output = run_example_with_input("confirm_custom", b"y\n");

    assert!(
        !output.status.success(),
        "Example should reject 'y' when using custom labels (Confirm/Cancel)"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Invalid input"),
        "Should show invalid input error, got: {}",
        stderr
    );
}

#[test]
fn test_confirm_custom_labels_accept_full_word() {
    // Test that full custom label word works
    let output = run_example_with_input("confirm_custom", b"confirm\n");

    assert!(
        output.status.success(),
        "Example should accept 'confirm', stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Confirmed!"),
        "Should confirm with 'confirm', got: {}",
        stdout
    );
}

#[test]
fn test_confirm_custom_labels_accept_partial_match() {
    // Test that partial match of custom label works (beyond the unique prefix)
    let output = run_example_with_input("confirm_custom", b"conf\n");

    assert!(
        output.status.success(),
        "Example should accept 'conf' as partial match, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Confirmed!"),
        "Should confirm with 'conf', got: {}",
        stdout
    );
}

#[test]
fn test_confirm_custom_labels_unique_prefix_affirmative() {
    // Test that 'co' (unique prefix for Confirm) works
    let output = run_example_with_input("confirm_custom", b"co\n");

    assert!(
        output.status.success(),
        "Example should accept 'co' as unique prefix for Confirm, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Confirmed!"),
        "Should confirm with 'co', got: {}",
        stdout
    );
}

#[test]
fn test_confirm_custom_labels_unique_prefix_negative() {
    // Test that 'ca' (unique prefix for Cancel) works
    let output = run_example_with_input("confirm_custom", b"ca\n");

    assert!(
        output.status.success(),
        "Example should accept 'ca' as unique prefix for Cancel, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Cancelled!"),
        "Should cancel with 'ca', got: {}",
        stdout
    );
}

#[test]
fn test_confirm_custom_labels_prompt_shows_unique_prefix() {
    // Verify that the prompt shows [co/ca] instead of [c/c]
    let output = run_example_with_input("confirm_custom", b"\n");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("[co/ca]"),
        "Prompt should show unique prefixes [co/ca], got: {}",
        stderr
    );
}
