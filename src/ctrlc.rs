use console::Term;
use once_cell::sync::Lazy;
use signal_hook::{
    consts::SIGINT,
    iterator::{Handle, Signals},
};
use std::{
    io::Error,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex, RwLock,
    },
    thread,
};

static MUTEX: Mutex<()> = Mutex::new(());
static INIT: AtomicBool = AtomicBool::new(false);
static HANDLE: Lazy<RwLock<CtrlcHandle>> = Lazy::new(|| RwLock::new(CtrlcHandle(None)));

#[derive(Clone)]
pub struct CtrlcHandle(Option<Handle>);

impl CtrlcHandle {
    pub fn close(&self) {
        if let Some(handle) = &self.0 {
            handle.close();
        }
    }
}

/// Show cursor after Ctrl+C is pressed
///
/// The caller should call the close method of the returned handle to release the resources
///
/// # Arguments
///
/// * `term` - The terminal to show the cursor
///
/// # Returns
///
/// * `CtrlcHandle` - The handle to release the resources
///
/// # Errors
///
/// * `Error` - If failed to set the Ctrl+C handler
///
pub fn show_cursor_after_ctrlc(term: &Term) -> Result<CtrlcHandle, Error> {
    let t = term.clone();
    set_ctrlc_handler(move || {
        let _ = t.show_cursor();
    })
}

/// Set Ctrl+C handler
///
/// The caller should call the close method of the returned handle to release the resources
///
/// # Arguments
///
/// * `handler` - The handler to be called when Ctrl+C is pressed
///
/// # Returns
///
/// * `Result<Option<CtrlcHandle>, Error>` - The handle to release the resources
///
/// # Errors
/// * `Error` - If failed to set the Ctrl+C handler
///
pub fn set_ctrlc_handler<F>(handler: F) -> Result<CtrlcHandle, Error>
where
    F: FnMut() + 'static + Send,
{
    let _mutex = MUTEX.lock();
    if INIT.load(Ordering::Relaxed) {
        let handle_guard = HANDLE.read().unwrap();
        return Ok(handle_guard.clone());
    }
    INIT.store(true, Ordering::Relaxed);

    let handle = set_ctrlc_handler_internal(handler)?;
    {
        let mut handle_guard = HANDLE.write().unwrap();
        *handle_guard = CtrlcHandle(Some(handle.clone()));
    }
    Ok(CtrlcHandle(Some(handle)))
}

fn set_ctrlc_handler_internal<F>(mut handler: F) -> Result<Handle, Error>
where
    F: FnMut() + 'static + Send,
{
    let mut signals = Signals::new([SIGINT])?;
    let handle = signals.handle();
    thread::Builder::new()
        .name("ctrl-c".into())
        .spawn(move || {
            for _ in signals.forever() {
                handler();
            }
        })?;
    Ok(handle)
}
