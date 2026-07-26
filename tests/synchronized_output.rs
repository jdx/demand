//! Exercises a real interactive redraw under a pseudo-terminal and verifies
//! that terminals never receive a presentable partial frame.
#![cfg(unix)]

use std::io::{Read, Write};

use portable_pty::{CommandBuilder, PtySize, native_pty_system};

const BEGIN: &[u8] = b"\x1b[?2026h";

#[test]
fn synchronized_output_child_scenario() {
    if std::env::var_os("DEMAND_PTY_SCENARIO").is_none() {
        return;
    }

    use demand::{DemandOption, MultiSelect};

    MultiSelect::new("Choose")
        .filterable(true)
        .option(DemandOption::new("one"))
        .option(DemandOption::new("two"))
        .run()
        .expect("run multiselect");

    std::process::exit(0);
}

#[test]
fn every_multiselect_redraw_is_synchronized() {
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("openpty");

    let mut cmd = CommandBuilder::new(std::env::current_exe().expect("current_exe"));
    cmd.args([
        "--exact",
        "synchronized_output_child_scenario",
        "--nocapture",
    ]);
    cmd.env("DEMAND_PTY_SCENARIO", "1");

    let mut child = pair.slave.spawn_command(cmd).expect("spawn child");
    drop(pair.slave);

    let mut writer = pair.master.take_writer().expect("take writer");
    // Trigger a second frame, select the highlighted item, and submit.
    writer.write_all(b"\x1b[B \r").expect("write input");
    writer.flush().expect("flush input");
    drop(writer);

    let mut reader = pair.master.try_clone_reader().expect("clone reader");
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).expect("read pty");
    child.wait().expect("wait child");
    drop(pair.master);

    assert!(
        contains(&bytes, BEGIN),
        "scenario emitted no synchronized-update sequences"
    );

    let mut open = false;
    let mut frames = 0;
    for operation in escapes(&bytes) {
        match operation {
            Escape::Begin => {
                assert!(!open, "nested synchronized update");
                open = true;
                frames += 1;
            }
            Escape::End => {
                assert!(open, "synchronized-update end without a begin");
                open = false;
            }
            Escape::Erase => {
                assert!(
                    open,
                    "in-place erase outside a synchronized update: {}",
                    String::from_utf8_lossy(&bytes).escape_debug()
                );
            }
        }
    }
    assert!(!open, "stream ended with a synchronized update open");
    assert!(frames >= 2, "scenario did not exercise a redraw");
}

enum Escape {
    Begin,
    End,
    Erase,
}

fn escapes(bytes: &[u8]) -> Vec<Escape> {
    let mut output = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != 0x1b || bytes.get(index + 1) != Some(&b'[') {
            index += 1;
            continue;
        }

        let params_start = index + 2;
        let mut end = params_start;
        while end < bytes.len() && !(0x40..=0x7e).contains(&bytes[end]) {
            end += 1;
        }
        if end >= bytes.len() {
            break;
        }

        let params = &bytes[params_start..end];
        match bytes[end] {
            b'h' if params == b"?2026" => output.push(Escape::Begin),
            b'l' if params == b"?2026" => output.push(Escape::End),
            b'A' | b'J' | b'K' => output.push(Escape::Erase),
            _ => {}
        }
        index = end + 1;
    }
    output
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}
