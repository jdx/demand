use console::Term;
use signal_hook::{
    consts::SIGINT,
    iterator::{Handle, Signals},
};
use std::{
    io::Error,
    sync::{LazyLock, RwLock},
    thread,
};

static HANDLE: LazyLock<RwLock<CtrlcHandle>> = LazyLock::new(|| RwLock::new(CtrlcHandle(None)));

#[derive(Clone)]
pub struct CtrlcHandle(Option<Handle>);

impl CtrlcHandle {
    pub fn close(&self) {
        if let Some(handle) = &self.0 {
            handle.close();
            let mut handle_guard = HANDLE.write().unwrap();
            if handle_guard.0.is_some() {
                handle_guard.0 = None;
            }
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
    let mut handle_guard = HANDLE.write().unwrap();
    if handle_guard.0.is_some() {
        return Ok(handle_guard.clone());
    }

    let handle = Some(set_ctrlc_handler_internal(handler).unwrap());

    if let Some(h) = handle {
        *handle_guard = CtrlcHandle(Some(h.clone()));
        Ok(CtrlcHandle(Some(h)))
    } else {
        Err(Error::new(
            std::io::ErrorKind::Other,
            "Failed to set Ctrl+C handler",
        ))
    }
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
