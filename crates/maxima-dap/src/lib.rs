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
//! - **Function redefinition clears breakpoints** (Legacy only) — Reloading a file invalidates breakpoints.
//!   Enhanced Maxima auto-reapplies breakpoints on redefinition.
//!
//! ## Dual-mode support
//!
//! The server auto-detects Enhanced Maxima (with `set_breakpoint` support) at launch
//! and selects the appropriate breakpoint strategy:
//! - **Legacy**: function+offset breakpoints, temp file, top-level code extraction
//! - **Enhanced**: file:line breakpoints, deferred breakpoints, line-snapping, direct batchload

pub mod breakpoints;
pub mod frames;
pub mod server;
pub mod strategy;
pub mod strategy_enhanced;
pub mod strategy_legacy;
pub mod transport;
pub mod types;
