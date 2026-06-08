//! Phase B / C overlays: lift data from kernel-events envelopes into
//! `EvalResult` after the legacy parser has run.
//!
//! Each overlay runs strictly after the legacy parser, and only
//! *overrides* fields when the relevant envelope is present.  Absent
//! envelopes leave the legacy parser's view authoritative — a strict
//! compatibility superset; sessions without the kernel-events channel
//! see no behaviour change.
//!
//! The overlays don't decide whether to *call* the legacy parser;
//! they assume it ran first and just refine its output where
//! envelopes have more reliable information.

use crate::catalog::packages::PackageCatalog;
use crate::catalog::search::Catalog;
use crate::error::AppError;
#[cfg(unix)]
use crate::maxima::errors as error_enhance;
#[cfg(unix)]
use crate::maxima::envelope::types::{Envelope, ErrorKind};
use crate::maxima::types::EvalResult;

/// Phase B / B.1: when an `error` envelope arrived during the eval,
/// take the kernel-events view as authoritative.  kernel-events
/// captures the merror() message verbatim and tags it with one of
/// maxima_error / lisp_error / parser_error / timeout / cancelled —
/// information the stdout scrape can't reliably recover (lisp errors
/// in particular often lack the markers the regex keys on).
///
/// Returns `Err(AppError::EvalCancelled)` when the first error
/// envelope has `kind: cancelled` so the caller surfaces it through
/// a dedicated UI affordance rather than the generic error path.
/// Other kinds are flattened into `EvalResult.error` and enhanced
/// through the existing pattern-match suggestions pipeline.
///
/// No-op (returns `Ok`) when no error envelope is present, so the
/// legacy parser scrape stays authoritative on builds where
/// kernel-events is disabled.
#[cfg(unix)]
pub fn apply_error_envelopes(
    result: &mut EvalResult,
    envelopes: &[Envelope],
    catalog: &Catalog,
    packages: Option<&PackageCatalog>,
) -> Result<(), AppError> {
    let first = envelopes.iter().find_map(|e| match e {
        Envelope::Error(err) => Some(err),
        _ => None,
    });
    let Some(err) = first else { return Ok(()) };

    if matches!(err.kind, ErrorKind::Cancelled) {
        // Short-circuit: cancellation isn't an evaluation error
        // proper — the user (or a host-side cancel transport) asked
        // for it.  Caller maps to a distinct UI state.
        return Err(AppError::EvalCancelled(err.message.clone()));
    }

    result.error = Some(err.message.clone());
    result.is_error = true;
    result.error_info = error_enhance::enhance_error_with_packages(
        &err.message,
        catalog,
        packages,
    );
    // Output label has no meaning when the eval errored — no %oN was
    // assigned.  The legacy parser also clears this in the same case.
    result.output_label = None;
    Ok(())
}

#[cfg(not(unix))]
pub fn apply_error_envelopes(
    _result: &mut EvalResult,
    _envelopes: &[()],
    _catalog: &Catalog,
    _packages: Option<&PackageCatalog>,
) -> Result<(), AppError> {
    Ok(())
}

/// Phase C: when a `display` envelope carrying a structured plot
/// arrived during the eval, take its inline JSON as authoritative
/// over the parser's `.plotly.json` path scrape.  The envelope path
/// avoids two failure modes the legacy scrape is exposed to: 1) the
/// path landing in a LaTeX `\mbox{}` block when display is suppressed
/// (where it's easy to miss); 2) reading a stale file when temp-name
/// collision happens.
///
/// Only fires when at least one display envelope carries the
/// `application/x-maxima-plotly` mime; absent envelopes (kernel-events
/// disabled, ax-plots predating Phase C, …) leave the parser's
/// legacy path-read authoritative — a strict compatibility superset.
///
/// First display envelope wins.  Multiple plots in one cell stay
/// represented through the legacy path-print + parser scrape until a
/// future phase teaches EvalResult to carry an array.
#[cfg(unix)]
pub fn apply_display_envelopes(result: &mut EvalResult, envelopes: &[Envelope]) {
    for env in envelopes {
        if let Envelope::Display(d) = env {
            if let Some(value) = d.mime_bundle.get("application/x-maxima-plotly") {
                let json_str = match value {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                if !json_str.is_empty() {
                    result.plot_data = Some(json_str);
                    return;
                }
            }
        }
    }
}

#[cfg(not(unix))]
pub fn apply_display_envelopes(_result: &mut EvalResult, _envelopes: &[()]) {}

/// Phase A.1: log what came in on the events channel during an eval
/// so we can validate the drain end-to-end before any envelope type
/// starts feeding into EvalResult.  Off when no envelopes arrived
/// (kernel-events not enabled / not installed) to keep stderr quiet.
#[cfg(unix)]
pub fn log_envelope_summary(cell_id: &str, envelopes: &[Envelope]) {
    if envelopes.is_empty() {
        return;
    }
    let mut kinds: std::collections::BTreeMap<&'static str, usize> =
        std::collections::BTreeMap::new();
    for env in envelopes {
        *kinds.entry(env.kind_label()).or_insert(0) += 1;
    }
    eprintln!("[events] cell={} envelopes={:?}", cell_id, kinds);
}

#[cfg(not(unix))]
pub fn log_envelope_summary(_cell_id: &str, _envelopes: &[()]) {}
