mod mapping;
mod markdown;
mod parser;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use aximar_lib::catalog::types::MaximaFunction;
use clap::{Parser, Subcommand};

const MAXIMA_GIT_URL: &str = "https://git.code.sf.net/p/maxima/code";
const MAXIMA_TEXI_REL: &str = "doc/info/maxima.texi";
const CATALOG_REL: &str = "src-tauri/src/catalog/catalog.json";
const DOCS_REL: &str = "src-tauri/src/catalog/docs.json";
const FIGURES_REL: &str = "public/figures";

/// Generate Maxima function catalog from official documentation.
#[derive(Parser)]
#[command(name = "catalog-gen", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Full pipeline: clone Maxima source (or use local), run makeinfo, parse XML, write catalog.
    Generate {
        /// Path to an existing Maxima source checkout. If omitted, clones/updates /tmp/maxima-src.
        #[arg(long)]
        maxima_src: Option<PathBuf>,

        /// Git ref (branch, tag, commit) to checkout. Only used when cloning/fetching.
        #[arg(long, default_value = "master")]
        git_ref: String,

        /// Output path for catalog.json [default: <workspace>/src-tauri/src/catalog/catalog.json]
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output path for docs.json [default: <workspace>/src-tauri/src/catalog/docs.json]
        #[arg(long)]
        docs_output: Option<PathBuf>,

        /// Merge with an existing catalog (hand-written entries take priority).
        /// Uses the output path as the merge source.
        #[arg(long)]
        merge: bool,

        /// Suppress informational output (e.g. unmapped category warnings)
        #[arg(short, long)]
        quiet: bool,

        /// Skip entries with descriptions shorter than N characters
        #[arg(long, default_value_t = 10)]
        min_description: usize,
    },

    /// Parse a pre-existing Maxima XML file (produced by `makeinfo --xml`).
    FromXml {
        /// Path to the Maxima XML file
        input: PathBuf,

        /// Output path for catalog.json [default: <workspace>/src-tauri/src/catalog/catalog.json]
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output path for docs.json [default: <workspace>/src-tauri/src/catalog/docs.json]
        #[arg(long)]
        docs_output: Option<PathBuf>,

        /// Merge with an existing catalog (hand-written entries take priority).
        /// Uses the output path as the merge source.
        #[arg(long)]
        merge: bool,

        /// Suppress informational output (e.g. unmapped category warnings)
        #[arg(short, long)]
        quiet: bool,

        /// Skip entries with descriptions shorter than N characters
        #[arg(long, default_value_t = 10)]
        min_description: usize,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Generate {
            maxima_src,
            git_ref,
            output,
            docs_output,
            merge,
            quiet,
            min_description,
        } => {
            let output = resolve_output(output, CATALOG_REL);
            let docs_output = resolve_output(docs_output, DOCS_REL);
            let log_unmapped = !quiet;
            let merge_path = if merge { Some(output.clone()) } else { None };

            // Determine maxima source directory — clone or use existing
            let src_dir = match maxima_src {
                Some(ref path) => {
                    if !path.exists() {
                        fatal(&format!("Maxima source path does not exist: {}", path.display()));
                    }
                    path.clone()
                }
                None => {
                    let cache_dir = PathBuf::from("/tmp/maxima-src");
                    ensure_maxima_source(&cache_dir, &git_ref);
                    cache_dir
                }
            };

            // Run makeinfo to convert texinfo to XML
            let texi_path = src_dir.join(MAXIMA_TEXI_REL);
            if !texi_path.exists() {
                fatal(&format!(
                    "Texinfo file not found at {}. Is this a Maxima source tree?",
                    texi_path.display()
                ));
            }

            // Generate .texi files from .texi.in templates (normally done by autoconf)
            let doc_info_dir = texi_path.parent().unwrap();
            generate_texi_includes(doc_info_dir, &src_dir);

            let xml_path = texi_path.with_extension("xml");
            run_makeinfo(&texi_path, &xml_path);

            // Parse and write catalog
            let xml = read_file(&xml_path);
            let functions = parse_and_merge(&xml, merge_path.as_deref(), log_unmapped, min_description);
            write_catalog(&functions, &output);

            // Generate docs.json
            let docs = parser::parse_xml_docs(&xml);
            write_docs(&docs, &docs_output);

            // Copy figures
            let figures_src = src_dir.join("doc/info/figures");
            let figures_dest = resolve_output(None, FIGURES_REL);
            copy_figures(&figures_src, &figures_dest);
        }

        Commands::FromXml {
            input,
            output,
            docs_output,
            merge,
            quiet,
            min_description,
        } => {
            let output = resolve_output(output, CATALOG_REL);
            let docs_output = resolve_output(docs_output, DOCS_REL);
            let log_unmapped = !quiet;
            let merge_path = if merge { Some(output.clone()) } else { None };

            let xml = read_file(&input);
            let functions = parse_and_merge(&xml, merge_path.as_deref(), log_unmapped, min_description);
            write_catalog(&functions, &output);

            // Generate docs.json
            let docs = parser::parse_xml_docs(&xml);
            write_docs(&docs, &docs_output);
        }
    }
}

// --- Pipeline steps ---

/// Ensure a Maxima source checkout exists at `dest`.
/// If the directory already contains a git repo, fetch and reset to the latest HEAD.
/// Otherwise, clone fresh.
fn ensure_maxima_source(dest: &Path, git_ref: &str) {
    if dest.join(".git").exists() {
        eprintln!("Found existing clone at {}, updating...", dest.display());
        run_cmd_in(
            "git",
            &["fetch", "origin", git_ref],
            dest,
            "git fetch",
        );
        run_cmd_in(
            "git",
            &["reset", "--hard", "FETCH_HEAD"],
            dest,
            "git reset",
        );
        eprintln!("Updated to latest HEAD.");
    } else {
        eprintln!("Cloning {MAXIMA_GIT_URL} (ref: {git_ref}) into {}...", dest.display());
        run_cmd(
            "git",
            &["clone", "--depth", "1", "--branch", git_ref, MAXIMA_GIT_URL, &dest.to_string_lossy()],
            "git clone",
        );
        eprintln!("Clone complete.");
    }
}

/// Generate all derived `.texi` files that Maxima's build system normally produces.
///
/// This replaces the autoconf `configure` + `make` steps with minimal substitutions
/// so that `makeinfo` can run directly from a git checkout.
fn generate_texi_includes(doc_info_dir: &Path, src_dir: &Path) {
    // Extract version from configure.ac: AC_INIT([maxima], [VERSION])
    let version = extract_maxima_version(src_dir);
    eprintln!("Maxima version: {version}");

    // include-maxima.texi: substitute @manual_version@
    generate_from_template(
        &doc_info_dir.join("include-maxima.texi.in"),
        &doc_info_dir.join("include-maxima.texi"),
        &[("@manual_version@", &version)],
    );

    // category-macros.texi: substitute @abs_srcdir@ and patch for XML output
    let abs_srcdir = doc_info_dir.to_string_lossy().to_string();
    generate_from_template(
        &doc_info_dir.join("category-macros.texi.in"),
        &doc_info_dir.join("category-macros.texi"),
        &[("@abs_srcdir@", &abs_srcdir)],
    );

    // Patch: add @ifxml no-op definitions for figure macros that makeinfo's XML mode
    // doesn't handle. The default @macro definitions in category-macros.texi get
    // clobbered by @unmacro in conditional blocks, leaving them undefined in XML mode.
    let cat_macros_path = doc_info_dir.join("category-macros.texi");
    let mut cat_macros = fs::read_to_string(&cat_macros_path).unwrap();
    if !cat_macros.contains("@ifxml") {
        cat_macros.push_str(r#"

@c --- XML output: provide no-op definitions for figure macros ---
@ifxml
@unmacro figure
@macro figure {file}
(Figure \file\)
@end macro

@unmacro altfigure
@macro altfigure {file, text}
(Figure \file\: \text\)
@end macro

@unmacro smallfigure
@macro smallfigure {file, text}
(Figure \file\: \text\)
@end macro
@end ifxml
"#);
        fs::write(&cat_macros_path, &cat_macros).unwrap_or_else(|e| {
            fatal(&format!("Error patching category-macros.texi: {e}"));
        });
    }

    // math.m4: copy from math.m4.in (no substitutions needed)
    let math_m4_in = doc_info_dir.join("math.m4.in");
    let math_m4 = doc_info_dir.join("math.m4");
    if math_m4_in.exists() && !math_m4.exists() {
        fs::copy(&math_m4_in, &math_m4).unwrap_or_else(|e| {
            fatal(&format!("Error copying math.m4.in -> math.m4: {e}"));
        });
    }

    // Process .texi.m4 files: m4 --prefix-builtins math.m4 Foo.texi.m4 > Foo.texi
    generate_m4_texi_files(doc_info_dir);
}

/// Extract the version string from Maxima's configure.ac.
fn extract_maxima_version(src_dir: &Path) -> String {
    let configure_ac = src_dir.join("configure.ac");
    let contents = fs::read_to_string(&configure_ac).unwrap_or_else(|e| {
        fatal(&format!(
            "Cannot read {}: {e}. Is this a Maxima source tree?",
            configure_ac.display()
        ));
    });

    // Look for: AC_INIT([maxima], [5.49post])
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("AC_INIT(") {
            // Extract second bracketed argument
            let mut brackets = trimmed.match_indices('[');
            brackets.next(); // skip first [maxima]
            if let Some((start, _)) = brackets.next() {
                if let Some(end) = trimmed[start + 1..].find(']') {
                    let version = &trimmed[start + 1..start + 1 + end];
                    // manual_version applies: s/_/./ (same transform as configure.ac)
                    return version.replace('_', ".");
                }
            }
        }
    }

    eprintln!("Warning: could not extract version from configure.ac, using \"unknown\"");
    "unknown".to_string()
}

/// Copy a .texi.in template to its output, performing simple string substitutions.
fn generate_from_template(template: &Path, output: &Path, substitutions: &[(&str, &str)]) {
    if !template.exists() {
        fatal(&format!("Template not found: {}", template.display()));
    }

    let mut contents = fs::read_to_string(template).unwrap_or_else(|e| {
        fatal(&format!("Error reading {}: {e}", template.display()));
    });

    for (from, to) in substitutions {
        contents = contents.replace(from, to);
    }

    fs::write(output, &contents).unwrap_or_else(|e| {
        fatal(&format!("Error writing {}: {e}", output.display()));
    });

    eprintln!("Generated {}", output.display());
}

/// Process all `.texi.m4` files in the doc/info directory using m4.
///
/// Equivalent to the Makefile rule: `m4 --prefix-builtins math.m4 Foo.texi.m4 > Foo.texi`
fn generate_m4_texi_files(doc_info_dir: &Path) {
    let math_m4 = doc_info_dir.join("math.m4");
    if !math_m4.exists() {
        eprintln!("Warning: math.m4 not found, skipping .texi.m4 processing");
        return;
    }

    let entries = fs::read_dir(doc_info_dir).unwrap_or_else(|e| {
        fatal(&format!("Cannot read {}: {e}", doc_info_dir.display()));
    });

    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if name.ends_with(".texi.m4") {
            let output_name = name.trim_end_matches(".m4");
            let output_path = doc_info_dir.join(output_name);

            // Skip if output already exists and is newer than source
            if output_path.exists() {
                if let (Ok(src_meta), Ok(dst_meta)) = (path.metadata(), output_path.metadata()) {
                    if let (Ok(src_time), Ok(dst_time)) = (src_meta.modified(), dst_meta.modified()) {
                        if dst_time >= src_time {
                            continue;
                        }
                    }
                }
            }

            eprintln!("m4: {} -> {}", name, output_name);
            let output = Command::new("m4")
                .args(["--prefix-builtins"])
                .arg(&math_m4)
                .arg(&path)
                .current_dir(doc_info_dir)
                .output()
                .unwrap_or_else(|e| {
                    fatal(&format!("Failed to run m4 (is m4 installed?): {e}"));
                });

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                fatal(&format!("m4 failed for {name}: {stderr}"));
            }

            fs::write(&output_path, &output.stdout).unwrap_or_else(|e| {
                fatal(&format!("Error writing {}: {e}", output_path.display()));
            });
        }
    }
}

fn run_makeinfo(texi_path: &Path, xml_output: &Path) {
    eprintln!(
        "Running makeinfo: {} -> {}",
        texi_path.display(),
        xml_output.display()
    );

    // makeinfo needs to run from the directory containing the .texi file
    // so that @include directives resolve correctly
    let texi_dir = texi_path.parent().unwrap_or(Path::new("."));
    let texi_filename = texi_path.file_name().unwrap();

    let status = Command::new("makeinfo")
        .args(["--xml", "--no-warn"])
        .arg(texi_filename)
        .arg("-o")
        .arg(xml_output)
        .current_dir(texi_dir)
        .status()
        .unwrap_or_else(|e| {
            fatal(&format!(
                "Failed to run makeinfo (is GNU Texinfo installed?): {e}"
            ));
        });

    if !status.success() {
        fatal(&format!("makeinfo exited with status: {status}"));
    }

    eprintln!("makeinfo complete.");
}

fn parse_and_merge(
    xml: &str,
    merge_path: Option<&Path>,
    log_unmapped: bool,
    min_description: usize,
) -> Vec<MaximaFunction> {
    eprintln!("XML size: {} bytes", xml.len());
    eprintln!("Parsing function definitions...");

    let mut functions = parser::parse_xml(xml, log_unmapped, min_description);
    eprintln!("Extracted {} definitions from XML", functions.len());

    if let Some(merge_path) = merge_path {
        eprintln!("Merging with existing catalog from {}...", merge_path.display());
        let existing_json = fs::read_to_string(merge_path).unwrap_or_else(|e| {
            fatal(&format!("Error reading {}: {e}", merge_path.display()));
        });
        let existing: Vec<MaximaFunction> =
            serde_json::from_str(&existing_json).unwrap_or_else(|e| {
                fatal(&format!("Error parsing {}: {e}", merge_path.display()));
            });

        functions = merge_catalogs(existing, functions);
        eprintln!("After merge: {} total entries", functions.len());
    }

    functions
}

fn write_catalog(functions: &[MaximaFunction], output: &Path) {
    let json = serde_json::to_string_pretty(functions).expect("failed to serialize catalog");
    fs::write(output, &json).unwrap_or_else(|e| {
        fatal(&format!("Error writing {}: {e}", output.display()));
    });
    eprintln!("Wrote {} functions to {}", functions.len(), output.display());
}

fn write_docs(docs: &HashMap<String, String>, output: &Path) {
    // Sort keys for stable output
    let mut sorted: std::collections::BTreeMap<&str, &str> = std::collections::BTreeMap::new();
    for (k, v) in docs {
        sorted.insert(k.as_str(), v.as_str());
    }
    let json = serde_json::to_string_pretty(&sorted).expect("failed to serialize docs");
    fs::write(output, &json).unwrap_or_else(|e| {
        fatal(&format!("Error writing {}: {e}", output.display()));
    });
    eprintln!("Wrote {} doc entries to {}", docs.len(), output.display());
}

fn copy_figures(src: &Path, dest: &Path) {
    if !src.exists() {
        eprintln!("Warning: figures directory not found at {}", src.display());
        return;
    }

    fs::create_dir_all(dest).unwrap_or_else(|e| {
        fatal(&format!("Error creating {}: {e}", dest.display()));
    });

    let mut count = 0;
    let entries = fs::read_dir(src).unwrap_or_else(|e| {
        fatal(&format!("Cannot read {}: {e}", src.display()));
    });

    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if ext == "png" {
                let dest_path = dest.join(entry.file_name());
                if let Err(e) = fs::copy(&path, &dest_path) {
                    eprintln!("Warning: failed to copy {}: {e}", path.display());
                }
                count += 1;
            }
        }
    }

    eprintln!("Copied {count} figure PNG files to {}", dest.display());
}

// --- Helpers ---

fn read_file(path: &Path) -> String {
    eprintln!("Reading {}...", path.display());
    fs::read_to_string(path).unwrap_or_else(|e| {
        fatal(&format!("Error reading {}: {e}", path.display()));
    })
}

fn run_cmd(program: &str, args: &[&str], label: &str) {
    let status = Command::new(program)
        .args(args)
        .status()
        .unwrap_or_else(|e| {
            fatal(&format!("Failed to run {label}: {e}"));
        });

    if !status.success() {
        fatal(&format!("{label} exited with status: {status}"));
    }
}

fn run_cmd_in(program: &str, args: &[&str], dir: &Path, label: &str) {
    let status = Command::new(program)
        .args(args)
        .current_dir(dir)
        .status()
        .unwrap_or_else(|e| {
            fatal(&format!("Failed to run {label}: {e}"));
        });

    if !status.success() {
        fatal(&format!("{label} exited with status: {status}"));
    }
}

/// Merge two catalogs. Entries from `existing` take priority over `generated` (by name).
fn merge_catalogs(
    existing: Vec<MaximaFunction>,
    generated: Vec<MaximaFunction>,
) -> Vec<MaximaFunction> {
    let mut by_name: HashMap<String, MaximaFunction> = HashMap::new();

    // Insert generated entries first
    for func in generated {
        by_name.insert(func.name.to_lowercase(), func);
    }

    // Existing entries override generated ones
    let existing_count = existing.len();
    for func in existing {
        by_name.insert(func.name.to_lowercase(), func);
    }

    eprintln!(
        "  Existing (priority): {existing_count}, total unique: {}",
        by_name.len()
    );

    let mut result: Vec<MaximaFunction> = by_name.into_values().collect();
    result.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    result
}

/// Resolve the output path, defaulting to `<workspace_root>/<default_rel>`.
fn resolve_output(explicit: Option<PathBuf>, default_rel: &str) -> PathBuf {
    if let Some(path) = explicit {
        return path;
    }
    let root = find_workspace_root();
    root.join(default_rel)
}

/// Walk up from the current directory to find the workspace root (contains `Cargo.toml` with [workspace]).
fn find_workspace_root() -> PathBuf {
    let mut dir = std::env::current_dir().unwrap_or_else(|e| {
        fatal(&format!("Cannot determine current directory: {e}"));
    });
    loop {
        let candidate = dir.join("Cargo.toml");
        if candidate.exists() {
            if let Ok(contents) = fs::read_to_string(&candidate) {
                if contents.contains("[workspace]") {
                    return dir;
                }
            }
        }
        if !dir.pop() {
            fatal("Cannot find workspace root (no Cargo.toml with [workspace] found in parent directories)");
        }
    }
}

fn fatal(msg: &str) -> ! {
    eprintln!("Error: {msg}");
    std::process::exit(1);
}
