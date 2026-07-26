//! Verifies that a terminal resize redraws an idle prompt without requiring
//! keyboard input.
#![cfg(unix)]

use std::io::{Read, Write};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use portable_pty::{CommandBuilder, PtySize, native_pty_system};

const BEGIN: &[u8] = b"\x1b[?2026h";
const CLEAR_SCREEN: &[u8] = b"\r\x1b[2J\r\x1b[H";

#[test]
fn resize_child_scenario() {
    if std::env::var_os("DEMAND_RESIZE_PTY_SCENARIO").is_none() {
        return;
    }

    use demand::{DemandOption, Select};

    Select::new("Choose")
        .option(DemandOption::new("one"))
        .option(DemandOption::new("two"))
        .run()
        .expect("run select");

    std::process::exit(0);
}

#[test]
fn resize_redraws_without_keyboard_input() {
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("openpty");

    let mut cmd = CommandBuilder::new(std::env::current_exe().expect("current_exe"));
    cmd.args(["--exact", "resize_child_scenario", "--nocapture"]);
    cmd.env("DEMAND_RESIZE_PTY_SCENARIO", "1");

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
    assert!(
        wait_for_frames(&rx, &mut output, 1),
        "prompt did not render its initial frame"
    );

    pair.master
        .resize(PtySize {
            rows: 10,
            cols: 40,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("resize pty");

    let redrew = wait_for_frames(&rx, &mut output, 2);

    writer.write_all(b"\r").expect("submit prompt");
    writer.flush().expect("flush input");
    drop(writer);
    child.wait().expect("wait child");
    drop(pair.master);
    reader_thread.join().expect("join reader");
    while let Ok(chunk) = rx.try_recv() {
        output.extend(chunk);
    }

    assert!(
        redrew,
        "prompt did not redraw after SIGWINCH without keyboard input: {}",
        String::from_utf8_lossy(&output).escape_debug()
    );
    assert!(
        frame(&output, 1).is_some_and(|redraw| occurrences(redraw, CLEAR_SCREEN) == 1),
        "resize redraw did not reset the viewport: {}",
        String::from_utf8_lossy(&output).escape_debug()
    );
}

fn wait_for_frames(rx: &Receiver<Vec<u8>>, output: &mut Vec<u8>, expected: usize) -> bool {
    let deadline = Instant::now() + Duration::from_secs(10);
    while occurrences(output, BEGIN) < expected {
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

fn frame(output: &[u8], index: usize) -> Option<&[u8]> {
    let start = output
        .windows(BEGIN.len())
        .enumerate()
        .filter(|(_, window)| *window == BEGIN)
        .nth(index)
        .map(|(offset, _)| offset + BEGIN.len())?;
    Some(&output[start..])
}
