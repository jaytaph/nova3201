use std::io;
use std::os::unix::io::{RawFd, BorrowedFd, AsRawFd};
use nix::fcntl::{fcntl, FcntlArg, OFlag};
use nix::pty::openpty;
use nix::unistd::{close, read, write, ttyname};
use crate::devices::uart::UartBackend;

pub struct PtyBackend {
    master: RawFd,
}

impl PtyBackend {
    /// Create a new PTY backend.
    ///
    /// Returns (backend, slave_path) where `slave_path` is something like `/dev/pts/7`
    /// that you can connect to with minicom/screen/etc.
    pub fn new() -> io::Result<(Self, String)> {
        // Create a new pty pair
        let pty = openpty(None, None)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to open pty: {e}")))?;

        // Get the slave device path from the slave FD
        let slave_path = ttyname(&pty.slave)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to get slave pty name: {e}")))?
            .to_string_lossy()
            .into_owned();

        // Close slave FD - we only need the master
        close(pty.slave)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to close slave pty: {e}")))?;

        // Make master non-blocking (do this before extracting raw FD)
        let flags_raw = fcntl(&pty.master, FcntlArg::F_GETFL)
            .map_err(|e| io::Error::from(e))?;
        let flags = OFlag::from_bits_truncate(flags_raw);

        fcntl(
            &pty.master,
            FcntlArg::F_SETFL(flags | OFlag::O_NONBLOCK),
        )
            .map_err(|e| io::Error::from(e))?;

        // Extract the raw FD from the master before it gets dropped
        let master_fd = pty.master.as_raw_fd();

        // Forget the master so it doesn't get closed
        std::mem::forget(pty.master);

        Ok((Self { master: master_fd }, slave_path))
    }

    /// Get the master file descriptor (useful for select/poll)
    pub fn master_fd(&self) -> RawFd {
        self.master
    }

    /// Helper to create a borrowed FD from the raw FD
    fn as_borrowed_fd(&self) -> BorrowedFd<'_> {
        unsafe { BorrowedFd::borrow_raw(self.master) }
    }
}

impl Drop for PtyBackend {
    fn drop(&mut self) {
        // Clean up the master FD
        let _ = close(self.master);
    }
}

impl UartBackend for PtyBackend {
    fn read_byte(&mut self) -> Option<u8> {
        let mut buf = [0u8; 1];
        match read(self.as_borrowed_fd(), &mut buf) {
            Ok(1) => Some(buf[0]),
            Ok(0) => None, // EOF
            Ok(_) => None, // shouldn't happen for 1-byte read
            Err(nix::errno::Errno::EAGAIN) => None, // no data
            Err(_) => None, // other errors (could log these in production)
        }
    }

    fn write_byte(&mut self, b: u8) {
        println!("PtyBackend: Writing byte: {}", b as char);
        let _ = write(self.as_borrowed_fd(), &[b]);
    }
}

// Safety: RawFd can be sent between threads
unsafe impl Send for PtyBackend {}