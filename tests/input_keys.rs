//! Drives prompts through a real pty to check that their key bindings see
//! the key events `console` actually produces for them.
#![cfg(unix)]

use std::io::{Read, Write};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use portable_pty::{CommandBuilder, PtySize, native_pty_system};

#[test]
fn input_keys_child_scenario() {
    if std::env::var_os("DEMAND_INPUT_KEYS_SCENARIO").is_none() {
        return;
    }
    let value = demand::Input::new("Name").run().expect("run input");
    println!("RESULT={value:?}");
    std::process::exit(0);
}

#[test]
fn grid_select_child_scenario() {
    if std::env::var_os("DEMAND_GRID_SELECT_SCENARIO").is_none() {
        return;
    }
    let choices = demand::GridSelect::new("Packages")
        .columns(["Current", "Range"])
        .filterable(true)
        .row(demand::GridRow::new("react").cell("1.0.0").cell("1.1.0"))
        .row(demand::GridRow::new("jest").cell("2.0.0").cell("2.1.0"))
        .run()
        .expect("run grid select");
    println!("RESULT={choices:?}");
    std::process::exit(0);
}

/// Type `keys` into a fresh `Input` and return what it submitted.
fn submit(keys: &[&[u8]]) -> String {
    submit_to(
        "input_keys_child_scenario",
        "DEMAND_INPUT_KEYS_SCENARIO",
        keys,
    )
}

/// Type `keys` into the prompt run by the child `scenario` test, which
/// only runs when `env` is set, then press Enter and return what the
/// prompt submitted.
fn submit_to(scenario: &str, env: &str, keys: &[&[u8]]) -> String {
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("openpty");
    let mut cmd = CommandBuilder::new(std::env::current_exe().expect("current_exe"));
    cmd.args(["--exact", scenario, "--nocapture"]);
    cmd.env(env, "1");
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

    // Send one key at a time, and only once the prompt is ready for it.
    //
    // Between keys the pty is in canonical mode (`console` only enters raw
    // mode while reading one), where ctrl-d, ctrl-u, backspace and others
    // are the terminal's own editing keys: sent then, the terminal acts on
    // them and the prompt never sees them. Seeing raw mode isn't enough on
    // its own, though: it could still be the read of the previous key. So
    // after each key, first wait for the frame the prompt draws once it
    // has handled it (every loop ends one synchronized update), and only
    // then for raw mode, which can then only be the next read.
    let presses: Vec<&[u8]> = keys
        .iter()
        .flat_map(|key| {
            // An escape sequence is one key; anything else is one per byte.
            if key.starts_with(b"\x1b") {
                vec![*key]
            } else {
                key.chunks(1).collect()
            }
        })
        .chain([&b"\r"[..]])
        .collect();
    let fd = pair.master.as_raw_fd().expect("pty fd");
    let mut output = Vec::new();
    let mut ready = wait_for_frames(&rx, &mut output, 1) && wait_for_raw_mode(fd);
    for (i, key) in presses.iter().enumerate() {
        if !ready {
            break;
        }
        writer.write_all(key).expect("write key");
        writer.flush().expect("flush");
        // Enter submits: there's no next read to wait for.
        if i + 1 < presses.len() {
            ready = wait_for_frames(&rx, &mut output, i + 2) && wait_for_raw_mode(fd);
        }
    }
    drop(writer);
    // If a key sequence never submits, or the prompt stopped responding
    // while keys were being sent, it's still waiting for input and would
    // never exit: stop it so the panic below can show the output instead
    // of hanging. It may have exited on its own already, and then there's
    // nothing to kill, so the error is ignored.
    let deadline = Instant::now() + Duration::from_secs(if ready { 10 } else { 0 });
    while child.try_wait().expect("poll child").is_none() {
        if Instant::now() >= deadline {
            let _ = child.kill();
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    child.wait().expect("wait child");
    drop(pair.master);
    reader_thread.join().expect("join reader");
    while let Ok(chunk) = rx.try_recv() {
        output.extend(chunk);
    }
    let output = String::from_utf8_lossy(&output).to_string();
    let result = output.split("RESULT=").nth(1).unwrap_or_else(|| {
        let why = if ready {
            "no result in output"
        } else {
            "prompt stopped responding to keys"
        };
        panic!("{why}: {}", output.escape_debug())
    });
    result.lines().next().unwrap_or_default().trim().to_string()
}

/// The end of a synchronized update, which the prompt writes once per
/// frame it draws.
const FRAME_END: &[u8] = b"\x1b[?2026l";

/// Wait until the prompt has drawn `frames` frames in total, collecting
/// its output. False if it doesn't within 10 seconds.
fn wait_for_frames(rx: &mpsc::Receiver<Vec<u8>>, output: &mut Vec<u8>, frames: usize) -> bool {
    let deadline = Instant::now() + Duration::from_secs(10);
    while output
        .windows(FRAME_END.len())
        .filter(|w| *w == FRAME_END)
        .count()
        < frames
    {
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

/// Wait until the pty is out of canonical mode, i.e. the prompt is
/// reading a key in raw mode. False if it isn't within 10 seconds.
fn wait_for_raw_mode(fd: std::os::fd::RawFd) -> bool {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let mut termios = unsafe { std::mem::zeroed::<libc::termios>() };
        let got = unsafe { libc::tcgetattr(fd, &mut termios) } == 0;
        if got && termios.c_lflag & libc::ICANON == 0 {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn kill_and_yank() {
    // alt-b back over "three", ctrl-k kills it, ctrl-a to the start,
    // ctrl-y yanks it back there.
    let result = submit(&[b"one two three", b"\x1bb", b"\x0b", b"\x01", b"\x19"]);
    assert_eq!(result, r#""threeone two ""#);
}

#[test]
fn char_motion_delete_and_transpose() {
    // ctrl-b twice lands before "c"; ctrl-d deletes it; ctrl-g has no
    // binding and isn't inserted; ctrl-t swaps "b" and "d".
    let result = submit(&[b"abcd", b"\x02", b"\x02", b"\x04", b"\x07", b"\x14"]);
    assert_eq!(result, r#""adb""#);
}

#[test]
fn word_kills() {
    // alt-backspace kills "three"; alt-b, then alt-d kills "two".
    let result = submit(&[b"one two three", b"\x1b\x7f", b"\x1bb", b"\x1bd"]);
    assert_eq!(result, r#""one  ""#);
}

#[test]
fn grid_select_enter_applies_filter_before_confirming() {
    // Enter while typing a filter applies it instead of confirming, so the
    // right arrow after it still moves jest to its range. Had the first
    // Enter confirmed, jest would still be on column 0.
    let result = submit_to(
        "grid_select_child_scenario",
        "DEMAND_GRID_SELECT_SCENARIO",
        &[b"/jest", b"\r", b"\x1b[C"],
    );
    assert_eq!(result, r#"[("react", 0), ("jest", 1)]"#);
}
