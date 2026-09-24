use std::io;

use console::{Key, Term};

pub(crate) struct EventReader {
    #[cfg(unix)]
    resize: unix::ResizeListener,
}

// Only keys can be read on Windows; resizes and updates need `select()`.
#[cfg_attr(not(unix), allow(dead_code))]
pub(crate) enum Event {
    Key(Key),
    /// The terminal was resized.
    Resize,
    /// A [`PromptHandle`](crate::PromptHandle) made an update.
    Update,
}

impl EventReader {
    pub(crate) fn new() -> io::Result<Self> {
        Self::with_updates(None)
    }

    /// Also wake up when `updates` receives one, so it can be drawn
    /// without waiting for a key.
    pub(crate) fn with_updates(updates: Option<&crate::handle::Updates>) -> io::Result<Self> {
        #[cfg(not(unix))]
        let _ = updates;
        Ok(Self {
            #[cfg(unix)]
            resize: unix::ResizeListener::new(
                updates
                    .and_then(|u| u.waker())
                    .map(|w| w.try_clone())
                    .transpose()?,
            )?,
        })
    }

    /// The next key, or `None` if the terminal was resized.
    pub(crate) fn read_key(&mut self, term: &Term) -> io::Result<Option<Key>> {
        loop {
            match self.read(term)? {
                Event::Key(key) => return Ok(Some(key)),
                Event::Resize => return Ok(None),
                Event::Update => {}
            }
        }
    }

    #[cfg(unix)]
    pub(crate) fn read(&mut self, term: &Term) -> io::Result<Event> {
        self.resize.read(term)
    }

    #[cfg(not(unix))]
    pub(crate) fn read(&mut self, term: &Term) -> io::Result<Event> {
        term.read_key().map(Event::Key)
    }
}

#[cfg(unix)]
mod unix {
    use std::fs::{File, OpenOptions};
    use std::io::{self, Read};
    use std::mem;
    use std::os::fd::{AsRawFd, RawFd};
    use std::os::unix::net::UnixStream;

    use console::Term;
    use signal_hook::{SigId, consts::SIGWINCH, low_level};

    use super::Event;

    pub(super) struct ResizeListener {
        input: Option<File>,
        read: UnixStream,
        updates: Option<UnixStream>,
        signal_id: SigId,
    }

    impl ResizeListener {
        pub(super) fn new(updates: Option<UnixStream>) -> io::Result<Self> {
            let input = if unsafe { libc::isatty(libc::STDIN_FILENO) } == 1 {
                None
            } else {
                Some(OpenOptions::new().read(true).write(true).open("/dev/tty")?)
            };
            let (read, write) = UnixStream::pair()?;
            read.set_nonblocking(true)?;
            let signal_id = signal_hook::low_level::pipe::register(SIGWINCH, write)?;
            Ok(Self {
                input,
                read,
                updates,
                signal_id,
            })
        }

        pub(super) fn read(&mut self, term: &Term) -> io::Result<Event> {
            let input = self
                .input
                .as_ref()
                .map(AsRawFd::as_raw_fd)
                .unwrap_or(libc::STDIN_FILENO);
            let resize = self.read.as_raw_fd();
            let updates = self.updates.as_ref().map(AsRawFd::as_raw_fd);
            let _raw_mode = RawMode::new(input)?;

            loop {
                let mut read_fds = unsafe { mem::zeroed::<libc::fd_set>() };
                unsafe {
                    libc::FD_ZERO(&mut read_fds);
                    libc::FD_SET(input, &mut read_fds);
                    libc::FD_SET(resize, &mut read_fds);
                    if let Some(fd) = updates {
                        libc::FD_SET(fd, &mut read_fds);
                    }
                }

                let result = unsafe {
                    libc::select(
                        input.max(resize).max(updates.unwrap_or(0)) + 1,
                        &mut read_fds,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                    )
                };
                if result < 0 {
                    let err = io::Error::last_os_error();
                    if err.kind() == io::ErrorKind::Interrupted {
                        continue;
                    }
                    return Err(err);
                }

                if unsafe { libc::FD_ISSET(resize, &read_fds) } {
                    drain(&mut self.read);
                    return Ok(Event::Resize);
                }
                // Keys before updates: a handle updating faster than frames
                // are drawn would otherwise keep the socket readable and
                // starve the keyboard. Every frame applies all pending
                // updates, so an update waiting behind a key isn't lost.
                if unsafe { libc::FD_ISSET(input, &read_fds) } {
                    return term.read_key().map(Event::Key);
                }
                if let Some(fd) = updates
                    && unsafe { libc::FD_ISSET(fd, &read_fds) }
                {
                    if let Some(stream) = self.updates.as_mut() {
                        drain(stream);
                    }
                    return Ok(Event::Update);
                }
            }
        }
    }

    fn drain(stream: &mut UnixStream) {
        let mut bytes = [0; 64];
        while matches!(stream.read(&mut bytes), Ok(n) if n > 0) {}
    }

    impl Drop for ResizeListener {
        fn drop(&mut self) {
            low_level::unregister(self.signal_id);
        }
    }

    struct RawMode {
        fd: RawFd,
        original: libc::termios,
    }

    impl RawMode {
        fn new(fd: RawFd) -> io::Result<Self> {
            let mut original = unsafe { mem::zeroed::<libc::termios>() };
            if unsafe { libc::tcgetattr(fd, &mut original) } != 0 {
                return Err(io::Error::last_os_error());
            }

            let mut raw = original;
            unsafe { libc::cfmakeraw(&mut raw) };
            raw.c_oflag = original.c_oflag;
            if unsafe { libc::tcsetattr(fd, libc::TCSADRAIN, &raw) } != 0 {
                return Err(io::Error::last_os_error());
            }

            Ok(Self { fd, original })
        }
    }

    impl Drop for RawMode {
        fn drop(&mut self) {
            unsafe {
                libc::tcsetattr(self.fd, libc::TCSADRAIN, &self.original);
            }
        }
    }
}
