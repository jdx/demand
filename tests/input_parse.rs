//! Drives `Input::run_parsed` through a real pty: a parse error is shown
//! and blocks submission, and a corrected answer then goes through.
#![cfg(unix)]

use std::io::{Read, Write};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use portable_pty::{CommandBuilder, PtySize, native_pty_system};

#[test]
fn input_parse_child_scenario() {
    if std::env::var_os("DEMAND_INPUT_PARSE_SCENARIO").is_none() {
        return;
    }
    let port: u16 = demand::Input::new("Port")
        .run_parsed(|s: &str| s.parse::<u16>().map_err(|_| "not a port number"))
        .expect("run input");
    println!("RESULT={port}");
    std::process::exit(0);
}

#[test]
fn a_parse_error_is_shown_and_a_corrected_answer_submits() {
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("openpty");
    let mut cmd = CommandBuilder::new(std::env::current_exe().expect("current_exe"));
    cmd.args(["--exact", "input_parse_child_scenario", "--nocapture"]);
    cmd.env("DEMAND_INPUT_PARSE_SCENARIO", "1");
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
    assert!(wait_for(&rx, &mut output, "Port"), "prompt did not render");
    type_keys(&mut writer, b"http\r");
    let shown_error = wait_for(&rx, &mut output, "not a port number");
    // Clear the line and give a valid answer.
    // ctrl-u is the terminal's line-kill character while it's in canonical
    // mode, which it is between keys: `console` only switches to raw mode
    // while reading one. Sent then, the terminal eats it and the prompt
    // never sees it. Wait for the prompt to be reading again first.
    wait_for_raw_mode(pair.master.as_raw_fd().expect("pty fd"));
    type_keys(&mut writer, b"\x15");
    thread::sleep(Duration::from_millis(50));
    type_keys(&mut writer, b"8080\r");
    let submitted = wait_for(&rx, &mut output, "RESULT=");

    drop(writer);
    if !submitted {
        // The prompt is still waiting for input and would never exit on
        // its own; stop it so the assertions below can report why. It may
        // have exited early instead, and then there's nothing to kill: the
        // error is ignored so the captured output still gets reported.
        let _ = child.kill();
    }
    child.wait().expect("wait child");
    drop(pair.master);
    reader_thread.join().expect("join reader");
    while let Ok(chunk) = rx.try_recv() {
        output.extend(chunk);
    }
    let shown = String::from_utf8_lossy(&output).escape_debug().to_string();
    assert!(shown_error, "parse error not shown: {shown}");
    assert!(submitted, "corrected answer not submitted: {shown}");
    assert!(
        String::from_utf8_lossy(&output).contains("RESULT=8080"),
        "wrong value returned: {shown}"
    );
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

fn type_keys(writer: &mut impl Write, keys: &[u8]) {
    writer.write_all(keys).expect("write keys");
    writer.flush().expect("flush");
}

fn wait_for(rx: &Receiver<Vec<u8>>, output: &mut Vec<u8>, needle: &str) -> bool {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !String::from_utf8_lossy(output).contains(needle) {
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
