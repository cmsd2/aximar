//! fd-3 pipe plumbing for the kernel-events channel.
//!
//! Phase-A scope: open an inheritable OS pipe before spawning Maxima,
//! hand the write end to the child as fd 3 (with `MAXIMA_EVENTS_FD=3`
//! in the environment), and spawn a tokio reader task on the parent
//! end that parses envelopes and forwards them to a callback.
//!
//! Unix-only for now.  The kernel-events design doc carries a Windows
//! fallback (inherited handle number), but every aximar backend except
//! Local already runs through a separate transport (Docker stdio, WSL,
//! …) where fd inheritance doesn't apply cleanly — so we restrict the
//! prototype to the Local backend on Unix and let other backends fall
//! back to the existing sentinel-based pipeline.

#![cfg(unix)]

use std::os::fd::{FromRawFd, OwnedFd, RawFd};

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;

use super::types::Envelope;

/// The two ends of a freshly-created OS pipe used for kernel-events.
///
/// The parent retains the read end; the write end is duped to fd 3 in
/// the child during `pre_exec` (see `pre_exec_dup_to_fd3`).
pub struct EventsPipe {
    pub read_end: OwnedFd,
    pub write_end: OwnedFd,
}

impl EventsPipe {
    /// Create the pipe with both ends `O_CLOEXEC` (child won't
    /// accidentally inherit either; the dup2 in `pre_exec` is what
    /// puts fd 3 into the child with CLOEXEC cleared).
    pub fn new() -> std::io::Result<Self> {
        let mut fds: [libc::c_int; 2] = [-1, -1];
        let r = unsafe { libc::pipe(fds.as_mut_ptr()) };
        if r != 0 {
            return Err(std::io::Error::last_os_error());
        }
        // Set CLOEXEC on both ends.  We didn't use pipe2 because not
        // every libc on macOS exposes it consistently across versions.
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
        Ok(EventsPipe { read_end, write_end })
    }
}

/// Build a `pre_exec` closure that duplicates `write_fd` onto fd 3 in
/// the child, clears CLOEXEC on fd 3 so it survives the exec, and
/// returns.  Intended for one-shot use through `Command::pre_exec`.
///
/// `write_fd` is the raw fd of the parent-held write end; both that fd
/// and fd 3 in the child remain open across the exec, but the
/// original parent-held fd is CLOEXEC and so the *child's* copy of it
/// closes at exec — only fd 3 survives.
pub fn pre_exec_dup_to_fd3(write_fd: RawFd) -> impl FnMut() -> std::io::Result<()> + Send + Sync {
    move || {
        // SAFETY: dup2, fcntl(F_GETFD), fcntl(F_SETFD) are all
        // async-signal-safe and have no preconditions beyond the fd
        // arguments being valid in the post-fork child, which they
        // are (fds were created in the parent before fork).
        unsafe {
            if libc::dup2(write_fd, 3) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            let flags = libc::fcntl(3, libc::F_GETFD);
            if flags >= 0 {
                libc::fcntl(3, libc::F_SETFD, flags & !libc::FD_CLOEXEC);
            }
        }
        Ok(())
    }
}

/// Spawn a background tokio task that reads JSON-line envelopes from
/// `read_end` and forwards them to `sender`.  Non-JSON lines and
/// parse failures are logged but do not stop the loop.  The task
/// ends when the pipe is closed (the child process exits) or when
/// the receiver is dropped.
pub fn spawn_reader_task(read_end: OwnedFd, sender: mpsc::UnboundedSender<Envelope>) {
    tokio::spawn(async move {
        // OwnedFd -> std::fs::File -> tokio::fs::File: tokio::fs runs
        // reads on the blocking pool, so the underlying fd must stay
        // blocking (don't set O_NONBLOCK — it surfaces as EAGAIN).
        // One reader task per session is fine on the blocking pool.
        let std_file = std::fs::File::from(read_end);
        let async_file = tokio::fs::File::from_std(std_file);
        let mut reader = BufReader::new(async_file);
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => break, // EOF — child closed its end.
                Ok(_) => {
                    let trimmed = line.trim_end();
                    if trimmed.is_empty() {
                        continue;
                    }
                    match serde_json::from_str::<Envelope>(trimmed) {
                        Ok(env) => {
                            if sender.send(env).is_err() {
                                // Receiver dropped — protocol layer is
                                // gone; nothing more to do.
                                break;
                            }
                        }
                        Err(e) => {
                            // Bad-envelope diagnostics still go to
                            // stderr — a malformed line is a real
                            // protocol bug worth surfacing.
                            eprintln!(
                                "[events] bad envelope: {} | line: {:?}",
                                e, trimmed
                            );
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[events] read error: {}", e);
                    break;
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[tokio::test]
    async fn reader_task_forwards_envelopes_and_handles_bad_json() {
        let pipe = EventsPipe::new().expect("pipe");
        // Move write end into a std::fs::File so we can write from
        // the test without going through a child process.
        let write_file = std::fs::File::from(pipe.write_end);
        let (tx, mut rx) = mpsc::unbounded_channel::<Envelope>();
        spawn_reader_task(pipe.read_end, tx);

        // Write two valid envelopes and one bad line.
        let mut w = write_file;
        writeln!(w, r#"{{"type":"ready"}}"#).unwrap();
        writeln!(w, "not json at all").unwrap();
        writeln!(
            w,
            r#"{{"type":"eval_end","eval_id":"e_1","status":"ok","duration_ms":10}}"#
        )
        .unwrap();
        drop(w); // Close write end so reader hits EOF.

        let mut received = Vec::new();
        while let Some(env) = rx.recv().await {
            received.push(env.kind_label());
        }

        assert_eq!(received, vec!["ready", "eval_end"]);
    }
}
