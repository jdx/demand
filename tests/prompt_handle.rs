//! Verifies that a `PromptHandle` update redraws an idle prompt without
//! requiring keyboard input.
#![cfg(unix)]

use std::io::{Read, Write};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use portable_pty::{CommandBuilder, PtySize, native_pty_system};

const CLEAR_SCREEN: &[u8] = b"\x1b[2J";

#[test]
fn prompt_handle_child_scenario() {
    if std::env::var_os("DEMAND_HANDLE_PTY_SCENARIO").is_none() {
        return;
    }

    use demand::{DemandOption, Select};

    let mut select = Select::new("Coins inserted: 0")
        .description("price: 100")
        .option(DemandOption::new("Complete payment"))
        .option(DemandOption::new("Quit"));
    let handle = select.handle();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(300));
        handle.set_title("Coins inserted: 50");
        handle.set_description("price: 100, 50 to go");
    });
    select.run().expect("run select");

    std::process::exit(0);
}

#[test]
fn handle_update_redraws_without_keyboard_input() {
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("openpty");

    let mut cmd = CommandBuilder::new(std::env::current_exe().expect("current_exe"));
    cmd.args(["--exact", "prompt_handle_child_scenario", "--nocapture"]);
    cmd.env("DEMAND_HANDLE_PTY_SCENARIO", "1");

    let mut child = pair.slave.spawn_command(cmd).expect("spawn child");
    drop(pair.slave);

    let mut writer = pair.master.take_writer().expect("take writer");
    let mut reader = pair.master.try_clone_reader().expect("clone reader");
    let (tx, rx) = mpsc::channel();
    let reader_thread = thread::spawn(move || {
        let mut chunk = [0; 4096];
        while let Ok(count) = reader.read(&mut chunk) {
            if count == 0 || tx.send(chunk[..count].to_vec()).is_err() {
                break;
            }
        }
    });

    let mut output = Vec::new();
    let drew_initial = wait_for(&rx, &mut output, b"Coins inserted: 0");
    let drew_update = wait_for(&rx, &mut output, b"50 to go");

    writer.write_all(b"\r").expect("submit prompt");
    writer.flush().expect("flush input");
    drop(writer);
    let status = child.wait().expect("wait child");
    drop(pair.master);
    reader_thread.join().expect("join reader");
    while let Ok(chunk) = rx.try_recv() {
        output.extend(chunk);
    }

    let shown = String::from_utf8_lossy(&output).escape_debug().to_string();
    assert!(drew_initial, "prompt did not render: {shown}");
    assert!(
        drew_update,
        "prompt did not redraw after a handle update without keyboard input: {shown}"
    );
    assert!(
        String::from_utf8_lossy(&output).contains("Coins inserted: 50"),
        "title update missing: {shown}"
    );
    // An update is drawn in place, not by resetting the viewport as a
    // resize does.
    assert_eq!(
        occurrences(&output, CLEAR_SCREEN),
        0,
        "update cleared the screen: {shown}"
    );
    assert!(status.success(), "child failed: {shown}");
}

fn wait_for(rx: &Receiver<Vec<u8>>, output: &mut Vec<u8>, needle: &[u8]) -> bool {
    let deadline = Instant::now() + Duration::from_secs(10);
    while occurrences(output, needle) == 0 {
        let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
            return false;
        };
        match rx.recv_timeout(remaining) {
            Ok(chunk) => output.extend(chunk),
            Err(_) => return false,
        }
    }
    true
}

fn occurrences(haystack: &[u8], needle: &[u8]) -> usize {
    haystack
        .windows(needle.len())
        .filter(|window| *window == needle)
        .count()
}
