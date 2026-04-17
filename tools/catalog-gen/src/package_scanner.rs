use std::collections::HashMap;
use std::fs;
use std::path::Path;

use aximar_core::catalog::search::Catalog;
use aximar_core::catalog::types::{MaximaFunction, PackageInfo};
use regex::Regex;

/// Scan a Maxima share directory and produce a list of loadable packages.
/// If `functions_catalog` is provided, signatures from the Texinfo-parsed catalog
/// take priority over regex-extracted ones from .mac files.
pub fn scan_packages(
    share_dir: &Path,
    catalog: &Catalog,
    functions_catalog: Option<&[MaximaFunction]>,
) -> Vec<PackageInfo> {
    if !share_dir.is_dir() {
        eprintln!("Error: share directory does not exist: {}", share_dir.display());
        return Vec::new();
    }

    let func_def_re = Regex::new(r"^([a-zA-Z_]\w*)\s*\(([^)]*)\)\s*:=").unwrap();

    let mut packages = Vec::new();

    let mut entries: Vec<_> = fs::read_dir(share_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let dir_name = entry.file_name().to_string_lossy().to_string();

        // Skip test/internal directories
        if dir_name.starts_with('.')
            || dir_name == "test_batch_encodings"
            || dir_name == "lisp-utils"
            || dir_name == "translators"
        {
            continue;
        }

        // Find all .mac files in this directory (non-recursive)
        let mac_files = find_mac_files(&path);
        if mac_files.is_empty() {
            continue;
        }

        // Determine package structure
        let has_matching_mac = mac_files
            .iter()
            .any(|f| stem(f) == dir_name);

        if has_matching_mac && mac_files.len() <= 3 {
            // Simple package: directory has a .mac file matching its name
            // e.g. distrib/distrib.mac → load("distrib")
            let mac_path = path.join(format!("{}.mac", dir_name));
            let func_sigs = extract_functions(&mac_path, &func_def_re);
            let functions: Vec<String> = func_sigs.iter().map(|(name, _)| name.clone()).collect();
            let signatures: HashMap<String, String> = func_sigs
                .into_iter()
                .filter(|(_, sig)| !sig.is_empty())
                .collect();
            let description = extract_description(&mac_path, &dir_name, catalog);
            packages.push(PackageInfo {
                name: dir_name.clone(),
                description,
                functions,
                signatures,
                builtin: false,
            });
        } else {
            // Multi-package directory: each .mac file is a separate loadable package
            // e.g. simplification/absimp.mac → load("simplification/absimp")
            // But if there's a matching .mac, also register the parent package
            if has_matching_mac {
                let mac_path = path.join(format!("{}.mac", dir_name));
                let func_sigs = extract_functions(&mac_path, &func_def_re);
                let functions: Vec<String> = func_sigs.iter().map(|(name, _)| name.clone()).collect();
                let signatures: HashMap<String, String> = func_sigs
                    .into_iter()
                    .filter(|(_, sig)| !sig.is_empty())
                    .collect();
                let description = extract_description(&mac_path, &dir_name, catalog);
                packages.push(PackageInfo {
                    name: dir_name.clone(),
                    description,
                    functions,
                    signatures,
                    builtin: false,
                });
            }

            for mac_file in &mac_files {
                let file_stem = stem(mac_file);

                // Skip the main package file (already added above), demo files, and test files
                if file_stem == dir_name
                    || file_stem.starts_with("rtest_")
                    || file_stem.starts_with("rtest")
                    || mac_file.ends_with(".dem")
                {
                    continue;
                }

                let mac_path = path.join(mac_file);
                let func_sigs = extract_functions(&mac_path, &func_def_re);

                // Skip files with no exported functions
                if func_sigs.is_empty() {
                    continue;
                }

                let functions: Vec<String> = func_sigs.iter().map(|(name, _)| name.clone()).collect();
                let signatures: HashMap<String, String> = func_sigs
                    .into_iter()
                    .filter(|(_, sig)| !sig.is_empty())
                    .collect();
                let load_name = format!("{}/{}", dir_name, file_stem);
                let description =
                    extract_description(&mac_path, &file_stem, catalog);
                packages.push(PackageInfo {
                    name: load_name,
                    description,
                    functions,
                    signatures,
                    builtin: false,
                });
            }
        }
    }

    // Filter out packages with no extractable functions (pure Lisp packages)
    packages.retain(|p| !p.functions.is_empty());

    // Clean up descriptions
    for p in &mut packages {
        // Remove file-mode lines like "-*- MACSYMA -*-"
        if p.description.contains("-*-") {
            p.description = format!("{} package", p.name.rsplit('/').next().unwrap_or(&p.name));
        }
    }

    // Enrich with catalog signatures (Texinfo takes priority over regex)
    if let Some(catalog_functions) = functions_catalog {
        enrich_with_catalog(&mut packages, catalog_functions);
    }

    packages.sort_by(|a, b| a.name.cmp(&b.name));
    packages
}

/// Find .mac files in a directory (non-recursive, sorted).
fn find_mac_files(dir: &Path) -> Vec<String> {
    let mut files: Vec<String> = fs::read_dir(dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if name.ends_with(".mac") && !name.starts_with("rtest") {
                Some(name)
            } else {
                None
            }
        })
        .collect();
    files.sort();
    files
}

/// Extract function definitions from a .mac file.
/// Returns `(name, signature)` pairs where signature is e.g. "pdf_normal(x, m, s)".
fn extract_functions(path: &Path, func_def_re: &Regex) -> Vec<(String, String)> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut seen = std::collections::HashSet::new();
    let mut functions: Vec<(String, String)> = content
        .lines()
        .filter_map(|line| {
            func_def_re.captures(line).map(|caps| {
                let name = caps[1].to_string();
                let params = caps[2].trim().to_string();
                let sig = if params.is_empty() {
                    format!("{}()", name)
                } else {
                    format!("{}({})", name, params)
                };
                (name, sig)
            })
        })
        .filter(|(name, _)| {
            // Filter out internal names
            !name.starts_with('_')
                && !name.starts_with("simpcheck")
                && !name.ends_with("_internal")
                && !name.ends_with("_aux")
                && name.len() > 1
                // Filter out ALL_CAPS names (Lisp-level leakage)
                && !name.chars().all(|c| c.is_uppercase() || c == '_')
        })
        .filter(|(name, _)| seen.insert(name.clone()))
        .collect();

    functions.sort_by(|a, b| a.0.cmp(&b.0));
    functions
}

/// Enrich package function signatures with data from the parsed Texinfo catalog.
/// Catalog signatures take priority over regex-extracted ones.
fn enrich_with_catalog(packages: &mut [PackageInfo], catalog: &[MaximaFunction]) {
    // Build a map from function name (lowercase) → first signature from the catalog
    let catalog_sigs: HashMap<String, String> = catalog
        .iter()
        .filter(|f| !f.signatures.is_empty())
        .map(|f| (f.name.to_lowercase(), f.signatures[0].clone()))
        .collect();

    for pkg in packages.iter_mut() {
        for func_name in &pkg.functions {
            if let Some(sig) = catalog_sigs.get(&func_name.to_lowercase()) {
                // Catalog signature overrides regex
                pkg.signatures.insert(func_name.clone(), sig.clone());
            }
        }
    }
}

/// Well-known package descriptions that can't easily be extracted automatically.
/// These override any auto-detected descriptions.
fn well_known_description(name: &str) -> Option<&'static str> {
    match name {
        "distrib" => Some("Probability distributions (normal, t, chi-squared, F, beta, etc.)"),
        "draw" => Some("Advanced plotting using Gnuplot (draw2d, draw3d, etc.)"),
        "descriptive" => Some("Descriptive statistics (mean, median, variance, etc.)"),
        "stats" => Some("Statistical inference and hypothesis testing"),
        "linearalgebra" => Some("Linear algebra (determinant, eigenvalues, matrix operations)"),
        "simplex" => Some("Linear programming using the simplex method"),
        "lsquares" => Some("Least squares curve fitting"),
        "mnewton" => Some("Multidimensional Newton's method for nonlinear systems"),
        "lapack" => Some("LAPACK numerical linear algebra routines"),
        "ezunits" => Some("Physical units and dimensional analysis"),
        "to_poly_solve" => Some("Polynomial equation solver with enhanced methods"),
        "orthopoly" => Some("Orthogonal polynomials (Legendre, Chebyshev, Hermite, etc.)"),
        "stringproc" => Some("String processing and character manipulation"),
        "numericalio" => Some("Reading and writing numerical data files"),
        "fourier_elim" => Some("Fourier elimination for real linear inequalities"),
        "solve_rec" => Some("Solving linear recurrence relations"),
        "bernstein" => Some("Bernstein polynomial basis for approximation"),
        "graphs" => Some("Graph theory algorithms and data structures"),
        "dynamics" => Some("Dynamical systems, orbits, and bifurcation diagrams"),
        "fractals" => Some("Fractal sets (Mandelbrot, Julia, IFS)"),
        "cobyla" => Some("Constrained optimization by linear approximation"),
        "lbfgs" => Some("L-BFGS quasi-Newton optimization method"),
        "nelder_mead" => Some("Nelder-Mead simplex optimization"),
        "pslq" => Some("PSLQ integer relation detection algorithm"),
        "raddenest" => Some("Denesting nested radical expressions"),
        "hypergeometric" => Some("Hypergeometric functions and simplification"),
        "finance" => Some("Financial mathematics (annuity, amortization, etc.)"),
        "physics" => Some("Physics package (Pauli matrices, Dirac equation, etc.)"),
        "tensor" => Some("Tensor algebra for general relativity (itensor, ctensor)"),
        _ => None,
    }
}

/// Extract a description for a package.
/// Uses well-known descriptions, then header comments, then catalog search as fallback.
fn extract_description(mac_path: &Path, name: &str, catalog: &Catalog) -> String {
    // Check well-known packages first
    if let Some(desc) = well_known_description(name) {
        return desc.to_string();
    }

    // Try header comment in .mac file
    if let Ok(content) = fs::read_to_string(mac_path) {
        if let Some(desc) = extract_header_description(&content) {
            return desc;
        }
    }

    // Try to infer from the functions this package provides —
    // search for the package name but only use the result if it looks
    // like a description of the package, not a conflicting built-in function.
    let results = catalog.search(name);
    if let Some(first) = results.first() {
        // Only use if the catalog entry name doesn't exactly match (which would be a built-in)
        // OR if the description mentions "package" or "library"
        let desc_lower = first.summary.to_lowercase();
        if desc_lower.contains("package") || desc_lower.contains("library") {
            return first_sentence(&first.summary);
        }
    }

    // Last resort: use the package name itself
    format!("{} package", name)
}

/// Extract the first sentence from a string.
fn first_sentence(s: &str) -> String {
    // Take up to the first period followed by a space or end of string
    if let Some(pos) = s.find(". ") {
        s[..=pos].to_string()
    } else if s.len() > 200 {
        format!("{}...", &s[..200])
    } else {
        s.to_string()
    }
}

/// Try to extract a description from the header comment of a .mac file.
fn extract_header_description(content: &str) -> Option<String> {
    // Look for a /* ... */ block comment at the top of the file
    let trimmed = content.trim_start();
    if !trimmed.starts_with("/*") {
        return None;
    }

    if let Some(end) = trimmed.find("*/") {
        let comment_body = &trimmed[2..end];
        // Skip copyright notices
        let lower = comment_body.to_lowercase();
        if lower.contains("copyright") || lower.contains("license") {
            return None;
        }
        // Clean up and take first meaningful line
        let meaningful: Vec<&str> = comment_body
            .lines()
            .map(|l| l.trim().trim_start_matches('*').trim())
            .filter(|l| !l.is_empty())
            .collect();
        if !meaningful.is_empty() {
            return Some(first_sentence(meaningful[0]));
        }
    }

    None
}

/// Get the stem (filename without extension) from a filename.
fn stem(filename: &str) -> &str {
    filename
        .rfind('.')
        .map(|i| &filename[..i])
        .unwrap_or(filename)
}
