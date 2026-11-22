//! List the files `load()` can reach in Maxima's share directory.
//!
//! Maxima's default search paths include `<share>/**/*.mac` and
//! `<share>/**/*.lisp`, so `load("orthopoly")` finds
//! `share/orthopoly/orthopoly.lisp` and `load("ctensor")` finds
//! `share/tensor/ctensor.mac`.  The trust check uses this list to treat such
//! loads as loading part of Maxima rather than an unknown file.

use std::fs;
use std::path::Path;

/// Top-level share subdirectories that hold test fixtures rather than
/// loadable code.
const SKIP_TOP_DIRS: &[&str] = &["test_batch_encodings"];

/// Subdirectories, at any depth, that hold test scripts.  Some of these
/// batch other files as soon as they load.
const SKIP_DIRS: &[&str] = &["test", "tests"];

/// Return the path of every `.mac` and `.lisp` file under `share_dir`,
/// relative to it, with `/` separators and no extension, sorted.
///
/// Test scripts (any name containing `test`, and anything under a `test` or
/// `tests` folder) are left out, and so are files whose names `load` could not take
/// as a plain name (anything but letters, digits, `-` and `_`).
pub fn scan_share_files(share_dir: &Path) -> Vec<String> {
    let mut files = Vec::new();
    walk(share_dir, "", &mut files);
    files.sort();
    files.dedup();
    files
}

fn walk(dir: &Path, prefix: &str, out: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            let skip = SKIP_DIRS.contains(&name.as_str())
                || (prefix.is_empty() && SKIP_TOP_DIRS.contains(&name.as_str()));
            if skip {
                continue;
            }
            if is_plain(&name) {
                walk(&path, &format!("{prefix}{name}/"), out);
            }
            continue;
        }
        let Some(stem) = name
            .strip_suffix(".mac")
            .or_else(|| name.strip_suffix(".lisp"))
        else {
            continue;
        };
        if stem.contains("test") || !is_plain(stem) {
            continue;
        }
        out.push(format!("{prefix}{stem}"));
    }
}

fn is_plain(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(root: &Path, rel: &str) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "").unwrap();
    }

    #[test]
    fn lists_mac_and_lisp_files_recursively() {
        let dir = tempfile::tempdir().unwrap();
        touch(dir.path(), "orthopoly/orthopoly.lisp");
        touch(dir.path(), "tensor/ctensor.mac");
        touch(dir.path(), "cobyla/lisp/cobyla.lisp");
        touch(dir.path(), "distrib/distrib.mac");
        assert_eq!(
            scan_share_files(dir.path()),
            vec![
                "cobyla/lisp/cobyla",
                "distrib/distrib",
                "orthopoly/orthopoly",
                "tensor/ctensor",
            ]
        );
    }

    #[test]
    fn skips_tests_other_files_and_odd_names() {
        let dir = tempfile::tempdir().unwrap();
        touch(dir.path(), "orthopoly/rtest_orthopoly.mac");
        touch(dir.path(), "orthopoly/README");
        touch(dir.path(), "orthopoly/h_atom.dem");
        touch(dir.path(), "contrib/odd name.mac");
        touch(dir.path(), "contrib/v1.2.mac");
        touch(dir.path(), "test_batch_encodings/latin1.mac");
        touch(dir.path(), "contrib/gentran/test/runge.mac");
        touch(dir.path(), "numeric/tests/check.lisp");
        touch(dir.path(), "algebra/charsets/test1.mac");
        touch(dir.path(), "fftpack5/test_fft_speed.mac");
        touch(dir.path(), "contrib/lurkmathml/mathmltest.mac");
        touch(dir.path(), ".hidden/x.mac");
        assert!(scan_share_files(dir.path()).is_empty());
    }

    #[test]
    fn keeps_both_files_when_mac_and_lisp_share_a_stem() {
        let dir = tempfile::tempdir().unwrap();
        touch(dir.path(), "draw/draw.lisp");
        touch(dir.path(), "draw/draw.mac");
        assert_eq!(scan_share_files(dir.path()), vec!["draw/draw"]);
    }
}
