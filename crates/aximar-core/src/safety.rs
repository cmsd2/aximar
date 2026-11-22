use crate::catalog::packages::PackageCatalog;
use crate::notebook::Notebook;

/// Who decides whether dangerous calls may run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustMode {
    /// The desktop app, and MCP connected to it: the notebook's trust flag
    /// decides, and the user can trust the notebook from the window.
    Interactive,
    /// Standalone MCP: no one to ask, so dangerous calls run only when the
    /// server was started with `--allow-dangerous`.  Batch runs
    /// (`aximar-mcp run`) pass `allow_dangerous: true`: running the command
    /// is the decision to run the notebook.
    Headless { allow_dangerous: bool },
}

/// Whether input may run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrustDecision {
    Allowed,
    /// The input calls these dangerous functions, and the mode doesn't allow them.
    NeedsTrust { functions: Vec<String> },
}

/// The one trust check for cells, run-all and expressions, from either
/// front end.  A trusted notebook (interactive) or `--allow-dangerous`
/// (headless) allows everything; otherwise any dangerous call blocks the run.
pub fn check_trust(
    notebook: &Notebook,
    input: &str,
    packages: &PackageCatalog,
    mode: TrustMode,
) -> TrustDecision {
    let allowed = match mode {
        TrustMode::Interactive => notebook.trusted(),
        TrustMode::Headless { allow_dangerous } => allow_dangerous,
    };
    if allowed {
        return TrustDecision::Allowed;
    }
    let functions: Vec<String> = detect_dangerous_calls(input, Some(packages))
        .into_iter()
        .map(|call| call.function_name)
        .collect();
    if functions.is_empty() {
        TrustDecision::Allowed
    } else {
        TrustDecision::NeedsTrust { functions }
    }
}

/// A dangerous function call detected in Maxima input.
#[derive(Debug, Clone, PartialEq)]
pub struct DangerousCall {
    pub function_name: String,
    pub description: String,
}

/// Functions that are always dangerous (OS interaction, file I/O).
const DANGEROUS_FUNCTIONS: &[(&str, &str)] = &[
    ("system", "executes shell commands"),
    ("batch", "loads and executes a file"),
    ("batchload", "loads and executes a file"),
    ("writefile", "writes to a file"),
    ("appendfile", "appends to a file"),
    ("closefile", "closes a file handle"),
    ("stringout", "writes expressions to a file"),
    ("with_stdout", "redirects output to a file"),
    ("save", "saves session data to a file"),
    ("store", "stores function definitions to a file"),
];

/// Detect dangerous function calls in Maxima input.
///
/// When `packages` is provided, `load("known_package")` calls are considered
/// safe and not flagged. Unknown `load()` arguments are flagged.
pub fn detect_dangerous_calls(
    input: &str,
    packages: Option<&PackageCatalog>,
) -> Vec<DangerousCall> {
    let mut results = Vec::new();
    let mut seen = std::collections::HashSet::new();

    // Check each dangerous function
    for &(name, description) in DANGEROUS_FUNCTIONS {
        if has_function_call(input, name) && seen.insert(name) {
            results.push(DangerousCall {
                function_name: name.to_string(),
                description: description.to_string(),
            });
        }
    }

    // Smart load() detection
    if has_function_call(input, "load") {
        let is_dangerous = check_load_calls(input, packages);
        if is_dangerous && seen.insert("load") {
            results.push(DangerousCall {
                function_name: "load".to_string(),
                description: "loads an unknown file".to_string(),
            });
        }
    }

    results
}

/// Check if `input` contains a function call like `name(`.
fn has_function_call(input: &str, name: &str) -> bool {
    let bytes = input.as_bytes();
    let name_bytes = name.as_bytes();
    let name_len = name_bytes.len();

    for i in 0..bytes.len() {
        // Check word boundary before the name
        if i > 0 && is_word_char(bytes[i - 1]) {
            continue;
        }
        // Check if the name matches at this position
        if i + name_len > bytes.len() {
            break;
        }
        if &bytes[i..i + name_len] != name_bytes {
            continue;
        }
        // Check for `(` after optional whitespace
        let rest = &input[i + name_len..];
        let trimmed = rest.trim_start();
        if trimmed.starts_with('(') {
            return true;
        }
    }
    false
}

fn is_word_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Check if any `load()` call in the input loads an unknown (non-package) file.
/// Returns `true` if at least one load call is dangerous.
fn check_load_calls(input: &str, packages: Option<&PackageCatalog>) -> bool {
    // Extract string arguments from load("...") calls
    let bytes = input.as_bytes();
    let load_bytes = b"load";

    for i in 0..bytes.len() {
        if i > 0 && is_word_char(bytes[i - 1]) {
            continue;
        }
        if i + 4 > bytes.len() || &bytes[i..i + 4] != load_bytes {
            continue;
        }
        let rest = &input[i + 4..];
        let trimmed = rest.trim_start();
        if !trimmed.starts_with('(') {
            continue;
        }
        // Found load( — extract string argument
        let after_paren = trimmed[1..].trim_start();
        if let Some(arg) = extract_string_arg(after_paren) {
            match packages {
                Some(pkg_catalog) if pkg_catalog.get(&arg).is_some() => {
                    // Known package — safe
                }
                _ => {
                    return true;
                }
            }
        } else {
            // Non-string argument (variable, expression) — flag as dangerous
            return true;
        }
    }
    false
}

/// Extract a string literal from the start of `s`, e.g. `"foo")...` → Some("foo").
fn extract_string_arg(s: &str) -> Option<String> {
    if s.starts_with('"') {
        let rest = &s[1..];
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notebook(trusted: bool) -> Notebook {
        let mut nb = Notebook::new();
        nb.set_trusted(trusted);
        nb
    }

    fn check(trusted: bool, input: &str, mode: TrustMode) -> TrustDecision {
        check_trust(&notebook(trusted), input, &PackageCatalog::load(), mode)
    }

    fn needs(functions: &[&str]) -> TrustDecision {
        TrustDecision::NeedsTrust { functions: functions.iter().map(|f| f.to_string()).collect() }
    }

    #[test]
    fn interactive_follows_the_notebook_trust_flag() {
        let mode = TrustMode::Interactive;
        assert_eq!(check(true, "system(\"ls\");", mode), TrustDecision::Allowed);
        assert_eq!(check(false, "system(\"ls\");", mode), needs(&["system"]));
        assert_eq!(check(false, "load(\"helpers.mac\");", mode), needs(&["load"]));
        assert_eq!(check(false, "integrate(x^2, x);", mode), TrustDecision::Allowed);
    }

    #[test]
    fn headless_ignores_the_notebook_and_follows_allow_dangerous() {
        let blocked = TrustMode::Headless { allow_dangerous: false };
        let allowed = TrustMode::Headless { allow_dangerous: true };
        assert_eq!(check(true, "system(\"ls\");", blocked), needs(&["system"]));
        assert_eq!(check(false, "system(\"ls\");", blocked), needs(&["system"]));
        assert_eq!(check(true, "system(\"ls\");", allowed), TrustDecision::Allowed);
        assert_eq!(check(false, "system(\"ls\");", allowed), TrustDecision::Allowed);
        assert_eq!(check(false, "diff(sin(x), x);", blocked), TrustDecision::Allowed);
    }

    #[test]
    fn detects_system_call() {
        let calls = detect_dangerous_calls("system(\"ls\")", None);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].function_name, "system");
        assert_eq!(calls[0].description, "executes shell commands");
    }

    #[test]
    fn detects_writefile() {
        let calls = detect_dangerous_calls("writefile(\"out.txt\")", None);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].function_name, "writefile");
    }

    #[test]
    fn detects_batch() {
        let calls = detect_dangerous_calls("batch(\"evil.mac\")", None);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].function_name, "batch");
    }

    #[test]
    fn does_not_flag_safe_functions() {
        let calls = detect_dangerous_calls("integrate(x^2, x)", None);
        assert!(calls.is_empty());
    }

    #[test]
    fn does_not_flag_diff() {
        let calls = detect_dangerous_calls("diff(sin(x), x)", None);
        assert!(calls.is_empty());
    }

    #[test]
    fn does_not_flag_substring_match() {
        // "ecosystem" contains "system" but should not be flagged
        let calls = detect_dangerous_calls("ecosystem(x)", None);
        assert!(calls.is_empty());
    }

    #[test]
    fn load_known_package_is_safe() {
        let packages = PackageCatalog::load();
        let calls = detect_dangerous_calls("load(\"distrib\")", Some(&packages));
        assert!(calls.is_empty());
    }

    #[test]
    fn load_unknown_file_is_dangerous() {
        let packages = PackageCatalog::load();
        let calls = detect_dangerous_calls("load(\"/tmp/evil.mac\")", Some(&packages));
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].function_name, "load");
    }

    #[test]
    fn load_without_packages_catalog_is_dangerous() {
        let calls = detect_dangerous_calls("load(\"distrib\")", None);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].function_name, "load");
    }

    #[test]
    fn multiple_dangerous_calls() {
        let calls = detect_dangerous_calls("system(\"ls\"); writefile(\"out\")", None);
        assert_eq!(calls.len(), 2);
        let names: Vec<&str> = calls.iter().map(|c| c.function_name.as_str()).collect();
        assert!(names.contains(&"system"));
        assert!(names.contains(&"writefile"));
    }

    #[test]
    fn deduplicates_same_function() {
        let calls = detect_dangerous_calls("system(\"ls\"); system(\"pwd\")", None);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].function_name, "system");
    }

    #[test]
    fn handles_whitespace_before_paren() {
        let calls = detect_dangerous_calls("system  (\"ls\")", None);
        assert_eq!(calls.len(), 1);
    }

    #[test]
    fn load_with_variable_arg_is_dangerous() {
        let packages = PackageCatalog::load();
        let calls = detect_dangerous_calls("load(filename)", Some(&packages));
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].function_name, "load");
    }
}
