//! Debug Adapter Protocol server for Maxima CAS.
//!
//! Bridges VS Code's debug UI to Maxima's built-in debugger, translating
//! between DAP concepts (file:line breakpoints, stack frames, variables)
//! and Maxima's text-based debugger protocol.
//!
//! # Known limitations
//!
//! - **SBCL required** — `:bt` and `:frame` produce no output on GCL.
//! - **No step-out** — Maxima has no native step-out. `:resume` continues to next breakpoint.
//! - **Top-level breakpoints impossible** — Lines outside function definitions are marked unverified.
//! - **`errcatch` suppresses breakpoints** — Breakpoints inside `errcatch()` don't fire.
//! - **Function redefinition clears breakpoints** — Reloading a file invalidates breakpoints.

pub mod breakpoints;
pub mod frames;
pub mod server;
pub mod transport;
pub mod types;
