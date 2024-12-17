use std::io::Error;

use console::Term;

pub struct CtrlcHandle();

impl CtrlcHandle {
    pub fn close(&self) {}
}

pub fn show_cursor_after_ctrlc(_term: &Term) -> Result<CtrlcHandle, Error> {
    Ok(CtrlcHandle())
}

pub fn set_ctrlc_handler<F>(_handler: F) -> Result<CtrlcHandle, Error>
where
    F: FnMut() + 'static + Send,
{
    Ok(CtrlcHandle())
}
