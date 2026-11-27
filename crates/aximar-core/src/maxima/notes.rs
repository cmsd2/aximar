//! Notes: things Maxima prints along the way that aren't errors, such as
//! "rat: replaced 0.5 by 1/2 = 0.5", said in plain language.
//!
//! Each note covers a range of a cell's text output lines; the window
//! shows the note in their place, with Maxima's lines one click away.  A
//! note's message has the note's ID and `.text` as its context, like the
//! fields of an error explanation (`maxima/errors.rs`).  Recognisers match
//! Maxima's English, anchored to whole lines, so print text that only
//! looks like a note stays text.  Notes are never written to files: they
//! are found again from the text when a notebook opens.

use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::message::Message;
use crate::msg_ctx;

/// A note on some of a cell's text output
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Note {
    /// Such as `note.rat_replaced`
    pub id: String,
    pub message: Message,
    /// The text output lines it covers, from `start` to before `end`,
    /// counting from 0
    pub start: usize,
    pub end: usize,
}

static RAT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^rat: replaced (\S+) by (\S+) = \S+$").unwrap());

static SOLVE_DEPENDENT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^solve: dependent equations eliminated: (.+)$").unwrap());

static PLOT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(plot2d|plot3d): expression evaluates to non-numeric value somewhere in plotting range\.$").unwrap()
});

static QUADRATURE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(quad_\w+): Numerical evaluation of (.+) at (\S+)$").unwrap());

const SOLVE_ARC_TRIG: &str = "solve: using arc-trig functions to get a solution.";
const ZERO_POWER: &str = "expt: undefined: 0 to a negative exponent.";

/// The notes in a cell's text output.  `has_result` is false for a cell
/// that failed, where Maxima's lines are part of the error.
pub fn notes(text_output: &str, has_result: bool) -> Vec<Note> {
    if !has_result {
        return Vec::new();
    }
    let lines: Vec<&str> = text_output.lines().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        match note_at(&lines, i) {
            Some(note) => {
                i = note.end;
                out.push(note);
            }
            None => i += 1,
        }
    }
    out
}

/// The note starting at line `i`, if any
fn note_at(lines: &[&str], i: usize) -> Option<Note> {
    let line = lines[i].trim_end();
    let next = lines.get(i + 1).map(|l| l.trim());
    let note = |id: &str, message: Message, len: usize| Note {
        id: id.to_string(),
        message,
        start: i,
        end: i + len,
    };

    // Consecutive replacements make one note
    if RAT_RE.is_match(line) {
        let mut replacements = Vec::new();
        let mut end = i;
        while let Some(caps) = lines.get(end).and_then(|l| RAT_RE.captures(l.trim_end())) {
            replacements.push(format!("{} → {}", &caps[1], &caps[2]));
            end += 1;
        }
        let replacements = replacements.join(", ");
        return Some(note(
            "note.rat_replaced",
            msg_ctx!(
                "note.rat_replaced.text",
                // i18n: Maxima: "rat: replaced 0.5 by 1/2 = 0.5", one line per number.
                // replacements lists them, such as "0.25 → 1/4, 0.5 → 1/2"
                "Maxima replaced decimals with exact fractions: {replacements}.",
                replacements = replacements
            ),
            end - i,
        ));
    }

    if line == SOLVE_ARC_TRIG {
        let len = if next == Some("Some solutions will be lost.") {
            2
        } else {
            1
        };
        return Some(note(
            "note.solve_arc_trig",
            msg_ctx!(
                "note.solve_arc_trig.text",
                // i18n: Maxima: "solve: using arc-trig functions to get a solution. Some solutions will be lost."
                "solve used inverse trigonometric functions, so it gives only some of the solutions."
            ),
            len,
        ));
    }

    if let Some(caps) = SOLVE_DEPENDENT_RE.captures(line) {
        let equations = &caps[1];
        return Some(note(
            "note.solve_dependent",
            msg_ctx!(
                "note.solve_dependent.text",
                // i18n: Maxima: "solve: dependent equations eliminated: (2)".
                // equations lists their positions, such as (2)
                "solve dropped equations that follow from the others: {equations}.",
                equations = equations
            ),
            1,
        ));
    }

    if line == "Principal Value" {
        return Some(note(
            "note.principal_value",
            msg_ctx!(
                "note.principal_value.text",
                // i18n: Maxima: "Principal Value", from integrate
                "The integral diverges inside the range, so the result is its Cauchy principal value."
            ),
            1,
        ));
    }

    if let Some(caps) = PLOT_RE.captures(line) {
        let function = &caps[1];
        return Some(note(
            "note.plot_non_numeric",
            msg_ctx!(
                "note.plot_non_numeric.text",
                // i18n: Maxima: "plot2d: expression evaluates to non-numeric value somewhere in plotting range."
                // function is plot2d or plot3d
                "{function} left out points where the expression has no real value.",
                function = function
            ),
            1,
        ));
    }

    if let Some(caps) = QUADRATURE_RE.captures(line) {
        if next == Some("is not a float or is not defined") {
            let (function, expr, point) = (&caps[1], &caps[2], &caps[3]);
            return Some(note(
                "note.quadrature_undefined",
                msg_ctx!(
                    "note.quadrature_undefined.text",
                    // i18n: Maxima: "quad_qags: Numerical evaluation of f at 0.0 is not a float or is not defined".
                    // function is a quadrature function; expr a Maxima expression; point a number
                    "{function} couldn't evaluate {expr} at {point}, so the result may be inaccurate.",
                    function = function,
                    expr = expr,
                    point = point
                ),
                2,
            ));
        }
    }

    if line == ZERO_POWER {
        return Some(note(
            "note.zero_power",
            msg_ctx!(
                "note.zero_power.text",
                // i18n: Maxima: "expt: undefined: 0 to a negative exponent.", in a cell
                // that still has a result
                "Maxima met 0 to a negative power on the way, and carried on without that part."
            ),
            1,
        ));
    }

    // The operator is printed in 2D across two lines, so it isn't named
    if line.starts_with("tellsimp: warning: rule will treat") {
        let len = if line.ends_with("as noncommutative and nonassociative.") {
            1
        } else if next.is_some_and(|n| n.ends_with("as noncommutative and nonassociative.")) {
            2
        } else {
            1
        };
        return Some(note(
            "note.tellsimp_noncommutative",
            msg_ctx!(
                "note.tellsimp_noncommutative.text",
                // i18n: Maxima: "tellsimp: warning: rule will treat '+' as noncommutative and nonassociative."
                "tellsimp will treat the rule's main operator as noncommutative and nonassociative, so the rule matches only terms in the order it was written."
            ),
            len,
        ));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(text: &str) -> Vec<(String, usize, usize)> {
        notes(text, true)
            .into_iter()
            .map(|n| (n.id, n.start, n.end))
            .collect()
    }

    #[test]
    fn consecutive_replacements_are_one_note() {
        let text = "rat: replaced 0.25 by 1/4 = 0.25\nrat: replaced 0.5 by 1/2 = 0.5";
        let found = notes(text, true);
        assert_eq!(found.len(), 1);
        let note = &found[0];
        assert_eq!((note.start, note.end), (0, 2));
        assert_eq!(note.message.id, "note.rat_replaced.text");
        assert_eq!(note.message.params["replacements"], "0.25 → 1/4, 0.5 → 1/2");
        assert!(note.message.text.contains("0.5 → 1/2"));
    }

    #[test]
    fn each_survey_note_is_found() {
        let cases = [
            ("solve: using arc-trig functions to get a solution.\nSome solutions will be lost.", "note.solve_arc_trig", 2),
            ("solve: dependent equations eliminated: (2)", "note.solve_dependent", 1),
            ("Principal Value", "note.principal_value", 1),
            ("plot2d: expression evaluates to non-numeric value somewhere in plotting range.", "note.plot_non_numeric", 1),
            ("quad_qags: Numerical evaluation of 1/sqrt(abs(x)) at 0.0\n          is not a float or is not defined", "note.quadrature_undefined", 2),
            ("expt: undefined: 0 to a negative exponent.", "note.zero_power", 1),
            ("tellsimp: warning: rule will treat 'mplus\n                                    ' as noncommutative and nonassociative.", "note.tellsimp_noncommutative", 2),
        ];
        for (text, id, len) in cases {
            assert_eq!(ids(text), vec![(id.to_string(), 0, len)], "{text}");
        }
        let n = &notes("solve: dependent equations eliminated: (2)", true)[0];
        assert_eq!(n.message.params["equations"], "(2)");
    }

    #[test]
    fn notes_keep_their_place_among_other_lines() {
        let text = "hello\nexpt: undefined: 0 to a negative exponent.\nplot2d: expression evaluates to non-numeric value somewhere in plotting range.\nbye";
        assert_eq!(
            ids(text),
            vec![
                ("note.zero_power".to_string(), 1, 2),
                ("note.plot_non_numeric".to_string(), 2, 3)
            ]
        );
    }

    #[test]
    fn print_text_that_looks_like_a_note_stays_text() {
        assert!(notes("rat: my own label", true).is_empty());
        assert!(notes("Principal Value of the loan", true).is_empty());
        assert!(notes(
            "my solve: using arc-trig functions to get a solution.",
            true
        )
        .is_empty());
    }

    #[test]
    fn a_failed_cell_has_no_notes() {
        assert!(notes("expt: undefined: 0 to a negative exponent.", false).is_empty());
    }
}
