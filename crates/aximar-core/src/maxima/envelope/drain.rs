//! Race a future against envelope arrival on the kernel-events channel.
//!
//! Used by `protocol::evaluate` to read stdout (the legacy sentinel-
//! terminated read) while concurrently pulling envelopes off the
//! kernel-events mpsc.  Without this, envelopes would back up in the
//! channel while the eval is in progress; the post-eval try_recv
//! drain captures everything in flight up to the moment the main
//! future resolved.

#[cfg(unix)]
use super::types::Envelope;

/// Wait on `main` while concurrently draining any envelopes that
/// arrive on `events_rx`.  Returns the future's output and the
/// vector of envelopes collected during its lifetime.
///
/// `biased` select preference means envelopes are pulled out of the
/// mpsc whenever they're ready, so the channel doesn't back up while
/// the main future is still running.  The post-main `try_recv` loop
/// captures envelopes that arrived between the main future
/// completing and our select loop noticing.
#[cfg(unix)]
pub async fn drive_with_envelope_drain<F, T>(
    main: F,
    events_rx: &mut Option<tokio::sync::mpsc::UnboundedReceiver<Envelope>>,
) -> (T, Vec<Envelope>)
where
    F: std::future::Future<Output = T>,
{
    let mut envs = Vec::new();
    tokio::pin!(main);
    let output = loop {
        tokio::select! {
            biased;
            env = recv_maybe(events_rx) => {
                if let Some(e) = env {
                    envs.push(e);
                }
                // Receiver closed: stop polling the events arm but
                // keep waiting for the main future to finish.
            }
            out = &mut main => break out,
        }
    };
    if let Some(rx) = events_rx.as_mut() {
        while let Ok(e) = rx.try_recv() {
            envs.push(e);
        }
    }
    (output, envs)
}

/// Helper for `tokio::select!`: when the receiver is None (kernel-
/// events disabled or already taken), return a never-resolving future
/// so the other arm always wins.  When it's Some, defer to recv().
#[cfg(unix)]
async fn recv_maybe(
    rx: &mut Option<tokio::sync::mpsc::UnboundedReceiver<Envelope>>,
) -> Option<Envelope> {
    match rx {
        Some(r) => r.recv().await,
        None => std::future::pending().await,
    }
}

/// Non-unix stub: no envelope channel, so the drain degenerates to
/// just awaiting `main` with an empty collected-vec.
#[cfg(not(unix))]
pub async fn drive_with_envelope_drain<F, T>(
    main: F,
    _events_rx: &mut Option<()>,
) -> (T, Vec<()>)
where
    F: std::future::Future<Output = T>,
{
    (main.await, Vec::new())
}
