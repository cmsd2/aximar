//! Which Maxima the Local backend runs, and why: the Maxima path setting,
//! then the `AXIMAR_MAXIMA_PATH` environment variable, then the standard
//! install locations, then a search of PATH.  The session starts the
//! program this finds, and the Settings window shows it, so the two agree.
//! Finding it starts nothing.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use serde::Serialize;

/// The environment variable that names the Maxima program
pub const MAXIMA_PATH_VAR: &str = "AXIMAR_MAXIMA_PATH";

/// Where the program's path came from
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MaximaSource {
    /// The Maxima path setting
    Setting,
    /// `AXIMAR_MAXIMA_PATH`
    Environment,
    /// A standard install location, such as /opt/homebrew/bin/maxima
    StandardLocation,
    /// A search of PATH
    Path,
}

/// Why the path found won't start Maxima
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MaximaProblem {
    /// No file at the path; or, with no path, no Maxima anywhere
    NotFound,
    /// A file, but not one the system can run
    NotExecutable,
}

/// The Maxima the Local backend would run
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MaximaLocation {
    /// The program; `None` when nothing was found
    pub path: Option<PathBuf>,
    /// Where `path` came from; `None` when nothing was found
    pub source: Option<MaximaSource>,
    pub problem: Option<MaximaProblem>,
}

impl MaximaLocation {
    /// The program to start: the path found, or the bare name for the
    /// system to look up when there is none
    pub fn command(&self) -> String {
        self.path
            .as_ref()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| "maxima".to_string())
    }
}

/// The standard install locations, in the order they're tried
fn standard_locations() -> Vec<PathBuf> {
    if cfg!(target_os = "windows") {
        let mut paths = Vec::new();
        if let Ok(entries) = std::fs::read_dir("C:\\") {
            let mut dirs: Vec<_> = entries
                .flatten()
                .filter(|e| e.file_name().to_string_lossy().starts_with("maxima-"))
                .map(|e| e.path())
                .collect();
            dirs.sort();
            paths.extend(dirs.into_iter().map(|d| d.join("bin").join("maxima.bat")));
        }
        paths
    } else {
        vec![
            PathBuf::from("/opt/homebrew/bin/maxima"),
            PathBuf::from("/usr/local/bin/maxima"),
            PathBuf::from("/usr/bin/maxima"),
        ]
    }
}

fn is_executable(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// A path someone named: a bare name is looked up on PATH
fn named(name: &str, source: MaximaSource, path_var: Option<&OsString>) -> MaximaLocation {
    let given = PathBuf::from(name);
    if given.components().count() == 1 && !given.is_absolute() {
        if let Some(found) = search(name, path_var) {
            return MaximaLocation {
                path: Some(found),
                source: Some(source),
                problem: None,
            };
        }
    }
    let problem = if !given.exists() {
        Some(MaximaProblem::NotFound)
    } else if !is_executable(&given) {
        Some(MaximaProblem::NotExecutable)
    } else {
        None
    };
    MaximaLocation {
        path: Some(given),
        source: Some(source),
        problem,
    }
}

fn search(name: &str, path_var: Option<&OsString>) -> Option<PathBuf> {
    which::which_in(name, path_var, std::env::temp_dir()).ok()
}

/// Where Maxima is, given each place to look.  `locate_maxima` passes the
/// real ones; tests pass their own.
pub fn locate_maxima_in(
    setting: Option<&str>,
    env_value: Option<&str>,
    candidates: &[PathBuf],
    path_var: Option<&OsString>,
) -> MaximaLocation {
    if let Some(setting) = setting.map(str::trim).filter(|s| !s.is_empty()) {
        return named(setting, MaximaSource::Setting, path_var);
    }
    if let Some(value) = env_value.map(str::trim).filter(|s| !s.is_empty()) {
        return named(value, MaximaSource::Environment, path_var);
    }
    if let Some(found) = candidates.iter().find(|c| c.exists()) {
        return MaximaLocation {
            path: Some(found.clone()),
            source: Some(MaximaSource::StandardLocation),
            problem: (!is_executable(found)).then_some(MaximaProblem::NotExecutable),
        };
    }
    match search("maxima", path_var) {
        Some(found) => MaximaLocation {
            path: Some(found),
            source: Some(MaximaSource::Path),
            problem: None,
        },
        None => MaximaLocation {
            path: None,
            source: None,
            problem: Some(MaximaProblem::NotFound),
        },
    }
}

/// The Maxima the Local backend would run with this Maxima path setting
pub fn locate_maxima(setting: Option<&str>) -> MaximaLocation {
    let env_value = std::env::var(MAXIMA_PATH_VAR).ok();
    let path_var = std::env::var_os("PATH");
    locate_maxima_in(
        setting,
        env_value.as_deref(),
        &standard_locations(),
        path_var.as_ref(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn program(dir: &Path, name: &str) -> PathBuf {
        let file = dir.join(name);
        std::fs::write(&file, "#!/bin/sh\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        file
    }

    fn path_of(dir: &Path) -> OsString {
        dir.as_os_str().to_owned()
    }

    #[test]
    fn the_setting_wins() {
        let dir = tempfile::tempdir().unwrap();
        let mine = program(dir.path(), "my-maxima");
        let env = program(dir.path(), "env-maxima");
        let found = locate_maxima_in(
            Some(mine.to_str().unwrap()),
            Some(env.to_str().unwrap()),
            &[],
            None,
        );
        assert_eq!(found.path, Some(mine));
        assert_eq!(found.source, Some(MaximaSource::Setting));
        assert_eq!(found.problem, None);
    }

    #[test]
    fn the_environment_comes_next() {
        let dir = tempfile::tempdir().unwrap();
        let env = program(dir.path(), "env-maxima");
        let standard = program(dir.path(), "maxima");
        let found = locate_maxima_in(Some("  "), Some(env.to_str().unwrap()), &[standard], None);
        assert_eq!(found.path, Some(env));
        assert_eq!(found.source, Some(MaximaSource::Environment));
    }

    #[test]
    fn a_standard_location_comes_before_path() {
        let standard = tempfile::tempdir().unwrap();
        let on_path = tempfile::tempdir().unwrap();
        let installed = program(standard.path(), "maxima");
        program(on_path.path(), "maxima");
        let found = locate_maxima_in(
            None,
            None,
            &[standard.path().join("missing"), installed.clone()],
            Some(&path_of(on_path.path())),
        );
        assert_eq!(found.path, Some(installed));
        assert_eq!(found.source, Some(MaximaSource::StandardLocation));
    }

    #[cfg(unix)]
    #[test]
    fn path_comes_last() {
        let on_path = tempfile::tempdir().unwrap();
        let installed = program(on_path.path(), "maxima");
        let found = locate_maxima_in(None, None, &[], Some(&path_of(on_path.path())));
        assert_eq!(found.path, Some(installed));
        assert_eq!(found.source, Some(MaximaSource::Path));
    }

    #[test]
    fn a_setting_that_names_no_file_is_a_problem() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("nope").join("maxima");
        let found = locate_maxima_in(Some(missing.to_str().unwrap()), None, &[], None);
        assert_eq!(found.path, Some(missing));
        assert_eq!(found.source, Some(MaximaSource::Setting));
        assert_eq!(found.problem, Some(MaximaProblem::NotFound));
    }

    #[cfg(unix)]
    #[test]
    fn a_file_that_does_not_run_is_a_problem() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("maxima");
        std::fs::write(&file, "").unwrap();
        let found = locate_maxima_in(Some(file.to_str().unwrap()), None, &[], None);
        assert_eq!(found.problem, Some(MaximaProblem::NotExecutable));
    }

    #[cfg(unix)]
    #[test]
    fn a_bare_name_in_the_setting_is_looked_up() {
        let on_path = tempfile::tempdir().unwrap();
        let installed = program(on_path.path(), "maxima-5.48");
        let found = locate_maxima_in(
            Some("maxima-5.48"),
            None,
            &[],
            Some(&path_of(on_path.path())),
        );
        assert_eq!(found.path, Some(installed));
        assert_eq!(found.source, Some(MaximaSource::Setting));
    }

    #[test]
    fn nothing_found() {
        let empty = tempfile::tempdir().unwrap();
        let found = locate_maxima_in(
            None,
            None,
            &[empty.path().join("maxima")],
            Some(&path_of(empty.path())),
        );
        assert_eq!(found.path, None);
        assert_eq!(found.source, None);
        assert_eq!(found.problem, Some(MaximaProblem::NotFound));
        assert_eq!(found.command(), "maxima");
    }
}
