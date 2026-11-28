use std::fmt::Write;
use std::path::Path;

fn main() {
    embed_catalogs();
    // tauri_build embeds icons/icon.ico in the Windows executable, and the
    // crate's generate_context! embeds the window icon.  Our own
    // rerun-if-changed above turns off Cargo's rerun on any change, so
    // without this a new icon only shows up after `cargo clean`.
    println!("cargo:rerun-if-changed=icons");
    tauri_build::build()
}

/// Embeds each language's catalog from `locales/` for the native menus, and
/// lists the languages with one.  `pseudo` is for the window's development
/// builds only.
fn embed_catalogs() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../locales");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut catalogs: Vec<(String, String)> = std::fs::read_dir(&dir)
        .expect("locales/")
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            let lang = path.file_stem()?.to_str()?.to_string();
            (path.extension()? == "po" && lang != "pseudo")
                .then(|| (lang, path.canonicalize().unwrap().display().to_string()))
        })
        .collect();
    catalogs.sort();
    let mut code = String::from("pub static CATALOGS: &[(&str, &str)] = &[\n");
    for (lang, path) in catalogs {
        writeln!(code, "    ({lang:?}, include_str!({path:?})),").unwrap();
    }
    code.push_str("];\n");
    let out = Path::new(&std::env::var("OUT_DIR").unwrap()).join("catalogs.rs");
    std::fs::write(out, code).unwrap();
}
