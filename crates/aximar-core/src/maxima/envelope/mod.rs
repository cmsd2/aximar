//! kernel-events envelope channel: the new-side of the migration
//! away from sentinel/regex stdout parsing.
//!
//! Everything in this module is part of the structured envelope
//! pipeline introduced by `kernel-events` (the Maxima package).  When
//! the channel isn't wired (non-Unix, non-Local backend, or
//! `AXIMAR_KERNEL_EVENTS` unset), every entry point in here is a
//! no-op — the orchestrator (`protocol.rs`) keeps owning behaviour
//! through the [`crate::maxima::legacy`] module instead.
//!
//! Submodule layout:
//!
//!   - [`types`] — typed `Envelope` enum mirroring the v1 schema.
//!   - [`events_pipe`] — fd-3 transport: OS pipe + reader task that
//!     parses JSON lines into `Envelope`s.
//!   - [`cancel_pipe`] — fd-4 transport: OS pipe + `CancelHandle` for
//!     host→kernel cancel signals.
//!   - [`overlay`] — Phase B / C overlays that lift envelope data
//!     into `EvalResult` after the legacy parser has run.
//!   - [`drain`] — primitive for racing a future against envelope
//!     collection; used by both per-eval and per-internal-command
//!     drains in `protocol.rs`.
//!   - [`observer`] — live tee for the fd-3 stream, used by the GUI
//!     "Events" log tab to display every frame as it arrives.

pub mod types;
#[cfg(unix)]
pub mod events_pipe;
#[cfg(unix)]
pub mod cancel_pipe;
pub mod observer;
pub mod overlay;
pub mod drain;

pub use observer::{EnvelopeFrame, EnvelopeObserver, NullEnvelopeObserver};
pub use types::Envelope;
