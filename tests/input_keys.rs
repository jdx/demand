//! Drives `Input` through a real pty to check that the readline-style
//! bindings see the key events `console` actually produces for them.
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

/// Type `keys` into a fresh `Input` and return what it submitted.
fn submit(keys: &[&[u8]]) -> String {
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("openpty");
    let mut cmd = CommandBuilder::new(std::env::current_exe().expect("current_exe"));
    cmd.args(["--exact", "input_keys_child_scenario", "--nocapture"]);
    cmd.env("DEMAND_INPUT_KEYS_SCENARIO", "1");
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

    // Wait for the prompt before typing, then send each key on its own so
    // escape sequences aren't split or merged differently than a terminal
    // would send them.
    let mut output = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !String::from_utf8_lossy(&output).contains("Name") && Instant::now() < deadline {
        if let Ok(chunk) = rx.recv_timeout(Duration::from_millis(100)) {
            output.extend(chunk);
        }
    }
    let fd = pair.master.as_raw_fd().expect("pty fd");
    for key in keys.iter().chain([&b"\r"[..]].iter()) {
        // Between keys the pty is in canonical mode (`console` only enters
        // raw mode while reading one), where ctrl-d, ctrl-u, backspace and
        // others are the terminal's own editing keys: sent then, the
        // terminal acts on them and the prompt never sees them. Wait for
        // the prompt to be reading before each key.
        wait_for_raw_mode(fd);
        writer.write_all(key).expect("write key");
        writer.flush().expect("flush");
        thread::sleep(Duration::from_millis(30));
    }
    drop(writer);
    // If a key sequence never submits, the prompt is still waiting for
    // input and would never exit: stop it so the panic below can show the
    // output instead of hanging. It may have exited on its own already,
    // and then there's nothing to kill, so the error is ignored.
    let deadline = Instant::now() + Duration::from_secs(10);
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
    let result = output
        .split("RESULT=")
        .nth(1)
        .unwrap_or_else(|| panic!("no result in output: {}", output.escape_debug()));
    result.lines().next().unwrap_or_default().trim().to_string()
}

/// Block until the pty is out of canonical mode, i.e. the prompt is
/// waiting for a key in raw mode.
fn wait_for_raw_mode(fd: std::os::fd::RawFd) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let mut termios = unsafe { std::mem::zeroed::<libc::termios>() };
        let got = unsafe { libc::tcgetattr(fd, &mut termios) } == 0;
        if got && termios.c_lflag & libc::ICANON == 0 {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "prompt never started reading keys"
        );
        thread::sleep(Duration::from_millis(5));
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
