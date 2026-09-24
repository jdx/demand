use std::sync::{Arc, Mutex};

/// Changes a running prompt's title or description from another thread.
///
/// Get one from [`Select::handle`](crate::Select::handle) before calling
/// `run`, and move a clone to whichever thread produces the updates. The
/// prompt redraws as soon as an update arrives. On Windows, updates are
/// shown on the next keypress instead.
///
/// Calls made after the prompt has finished are ignored.
///
/// # Example
/// ```no_run
/// use std::{thread, time::Duration};
/// use demand::{DemandOption, Select};
///
/// let mut select = Select::new("Coins inserted: 0")
///     .option(DemandOption::new("Complete payment"))
///     .option(DemandOption::new("Quit"));
/// let handle = select.handle();
/// thread::spawn(move || {
///     for coins in 1.. {
///         thread::sleep(Duration::from_secs(1));
///         handle.set_title(format!("Coins inserted: {coins}"));
///     }
/// });
/// let choice = select.run().expect("error running select");
/// ```
#[derive(Clone)]
pub struct PromptHandle {
    shared: Arc<Shared>,
}

impl PromptHandle {
    /// Replace the prompt's title.
    pub fn set_title(&self, title: impl Into<String>) {
        self.shared.pending.lock().unwrap().title = Some(title.into());
        self.shared.wake();
    }

    /// Replace the prompt's description.
    pub fn set_description(&self, description: impl Into<String>) {
        self.shared.pending.lock().unwrap().description = Some(description.into());
        self.shared.wake();
    }
}

/// What a prompt with a handle keeps, to pick up updates from.
pub(crate) struct Updates {
    shared: Arc<Shared>,
}

impl Updates {
    pub(crate) fn new() -> Self {
        Self {
            shared: Arc::new(Shared::new()),
        }
    }

    pub(crate) fn handle(&self) -> PromptHandle {
        PromptHandle {
            shared: Arc::clone(&self.shared),
        }
    }

    /// Take the updates made since the last call.
    pub(crate) fn take(&self) -> Pending {
        std::mem::take(&mut *self.shared.pending.lock().unwrap())
    }

    /// A socket that becomes readable when an update is made.
    #[cfg(unix)]
    pub(crate) fn waker(&self) -> Option<&std::os::unix::net::UnixStream> {
        self.shared.socket.as_ref().map(|(read, _)| read)
    }
}

#[derive(Default)]
pub(crate) struct Pending {
    pub(crate) title: Option<String>,
    pub(crate) description: Option<String>,
}

struct Shared {
    pending: Mutex<Pending>,
    /// The (read, write) ends of the wake-up socket. Both live here, so a
    /// handle outliving the prompt never writes to a closed socket. `None`
    /// if the socket couldn't be created, and updates wait for a key.
    #[cfg(unix)]
    socket: Option<(
        std::os::unix::net::UnixStream,
        std::os::unix::net::UnixStream,
    )>,
}

impl Shared {
    fn new() -> Self {
        Self {
            pending: Mutex::default(),
            #[cfg(unix)]
            socket: Self::socket().ok(),
        }
    }

    #[cfg(unix)]
    fn socket() -> std::io::Result<(
        std::os::unix::net::UnixStream,
        std::os::unix::net::UnixStream,
    )> {
        let (read, write) = std::os::unix::net::UnixStream::pair()?;
        read.set_nonblocking(true)?;
        // A full socket already holds a pending wake-up; writing must
        // never block the updating thread.
        write.set_nonblocking(true)?;
        Ok((read, write))
    }

    fn wake(&self) {
        #[cfg(unix)]
        if let Some((_, write)) = &self.socket {
            use std::io::Write;
            let _ = (&*write).write(&[0]);
        }
    }
}
