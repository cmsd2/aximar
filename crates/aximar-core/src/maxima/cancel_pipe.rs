//! fd-4 pipe plumbing for the kernel-events cancel transport.
//!
//! Phase-D companion to `events_pipe.rs`: open an inheritable OS pipe
//! before spawn, hand the read end to the child as fd 4 (with
//! `MAXIMA_CANCEL_FD=4` in the environment), keep the write end in the
//! parent.  Writing a single byte to the pipe wakes the cancel
//! watcher thread kernel-events spawns inside Maxima, which sets the
//! cancel flag; the next `check-cancel` in user code aborts and emits
//! a `cancelled` error envelope.
//!
//! The CancelHandle is intentionally separable from the rest of
//! MaximaProcess so a host can fire a cancel *while* the session lock
//! is held by an in-flight `protocol::evaluate` — otherwise cancel
//! couldn't fire until the very eval it's meant to abort released the
//! lock.  Unix-only; non-Unix and non-Local backends fall back to no
//! cancel support (today's `interrupt_and_resync` SIGINT path is the
//! only thing they have).

#![cfg(unix)]

use std::io::Write;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};

/// The two ends of a freshly-created OS pipe used for cancel signals.
/// Parent retains `write_end`; `read_end` is dup'd onto fd 4 in the
/// child during `pre_exec` (see `pre_exec_dup_to_fd4`).
pub struct CancelPipe {
    pub read_end: OwnedFd,
    pub write_end: OwnedFd,
}

impl CancelPipe {
    /// Create the pipe with both ends CLOEXEC.  `pre_exec_dup_to_fd4`
    /// clears CLOEXEC on the child's fd 4 so it survives the exec;
    /// the parent's write end keeps it (we don't fork it further).
    pub fn new() -> std::io::Result<Self> {
        let mut fds: [libc::c_int; 2] = [-1, -1];
        let r = unsafe { libc::pipe(fds.as_mut_ptr()) };
        if r != 0 {
            return Err(std::io::Error::last_os_error());
        }
        for fd in fds {
            let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
            if flags < 0 {
                let e = std::io::Error::last_os_error();
                unsafe {
                    libc::close(fds[0]);
                    libc::close(fds[1]);
                }
                return Err(e);
            }
            unsafe { libc::fcntl(fd, libc::F_SETFD, flags | libc::FD_CLOEXEC) };
        }
        // SAFETY: pipe() succeeded; we own both fds.
        let read_end = unsafe { OwnedFd::from_raw_fd(fds[0]) };
        let write_end = unsafe { OwnedFd::from_raw_fd(fds[1]) };
        Ok(CancelPipe { read_end, write_end })
    }
}

/// Build a `pre_exec` closure that duplicates `read_fd` onto fd 4 in
/// the child and clears CLOEXEC so it survives the exec.  Mirrors
/// `events_pipe::pre_exec_dup_to_fd3` but for the cancel transport.
pub fn pre_exec_dup_to_fd4(read_fd: RawFd) -> impl FnMut() -> std::io::Result<()> + Send + Sync {
    move || {
        // SAFETY: dup2 and fcntl on F_GETFD / F_SETFD are
        // async-signal-safe.  read_fd was created in the parent
        // before fork, so it's valid in the child here.
        unsafe {
            if libc::dup2(read_fd, 4) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            let flags = libc::fcntl(4, libc::F_GETFD);
            if flags >= 0 {
                libc::fcntl(4, libc::F_SETFD, flags & !libc::FD_CLOEXEC);
            }
        }
        Ok(())
    }
}

/// Movable cancel signaller for a running Maxima process.
///
/// Holds the write end of the cancel pipe in a way that's independent
/// of `MaximaProcess`'s lock — extract it once at session start and
/// keep it in the host's session registry, so `request_cancel()` can
/// fire while the session mutex is held by an in-flight evaluation.
/// Dropping the handle closes the pipe; the in-kernel watcher then
/// sees EOF and exits, so the cancel mechanism stops working for the
/// rest of that Maxima session.
pub struct CancelHandle {
    write: OwnedFd,
}

impl CancelHandle {
    /// Wrap an OwnedFd held by the parent for the cancel pipe.
    pub fn new(write: OwnedFd) -> Self {
        Self { write }
    }

    /// Write a single byte to the pipe.  The kernel-side watcher's
    /// blocking read returns, sets `*cancel-flag*`, and the next call
    /// to `kernel-events:check-cancel` from user code raises
    /// `cancellation-requested`, which the eval-hooks translate into
    /// a `cancelled` error envelope.
    ///
    /// Returns Ok even when the pipe write would have blocked (cannot
    /// happen here — single byte, atomic on POSIX up to PIPE_BUF).
    /// Returns Err if the pipe has been closed or the kernel is gone.
    pub fn request_cancel(&self) -> std::io::Result<()> {
        // Use a raw `write(2)` so we don't run into BufWriter
        // semantics; one byte, one syscall, no buffering.
        let n = unsafe {
            libc::write(self.write.as_raw_fd(), b"x".as_ptr() as *const _, 1)
        };
        if n < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        }
    }
}

/// Spawn-time helper: hold the write-end in an `OwnedFd` we can later
/// move into a CancelHandle.  Symmetric naming with events_pipe.rs's
/// `EventsPipe`.
impl From<OwnedFd> for CancelHandle {
    fn from(write: OwnedFd) -> Self {
        Self::new(write)
    }
}

// `Write` impl lets tests and host code reach for the familiar
// trait — the underlying syscall is the same as request_cancel.
impl Write for CancelHandle {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n = unsafe {
            libc::write(
                self.write.as_raw_fd(),
                buf.as_ptr() as *const _,
                buf.len(),
            )
        };
        if n < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(n as usize)
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn cancel_handle_fires_one_byte() {
        let pipe = CancelPipe::new().expect("pipe");
        let handle = CancelHandle::from(pipe.write_end);
        handle.request_cancel().expect("fire");
        let mut std_read = std::fs::File::from(pipe.read_end);
        let mut buf = [0u8; 1];
        let n = std_read.read(&mut buf).expect("read");
        assert_eq!(n, 1);
        assert_eq!(buf, *b"x");
    }
}
