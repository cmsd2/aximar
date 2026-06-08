//! Legacy stdout-parsing pipeline: regex sentinel matching, junk-
//! latex filtering, plot-path scraping out of `\mbox{…}` blocks.
//!
//! Everything in this module is the *old* way of reconstructing
//! kernel events out of Maxima's textual output stream.  The
//! migration plan is to retire it one feature at a time as the
//! kernel-events envelope channel (see [`crate::maxima::envelope`])
//! takes over: errors via the `error` envelope (Phase B), structured
//! display via the `display` envelope (Phase C), eventually
//! eval-lifecycle and stdout text too.
//!
//! Until the envelope channel covers everything, the orchestrator in
//! [`crate::maxima::protocol`] runs the legacy parser first and then
//! lets the envelope overlay refine the result where it has more
//! reliable information.  Hosts/backends without envelope support
//! (Docker, WSL, non-Unix) keep running legacy-only.
//!
//! The intent of the file split is that when we go envelope-only on
//! a given path, the corresponding code can be deleted from this
//! module without touching the rest of the codebase.

pub mod parser;
