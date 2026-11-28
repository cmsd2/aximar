//! Translation preview, for translators: load a `.po` from disk, show it in
//! the window and the menus, and show it again each time the file is saved.
//!
//! In every build, so a translator can check a translation in the app as
//! released.  See docs/i18n.md.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tokio_util::sync::CancellationToken;

use aximar_core::error::AppError;

use crate::i18n::Translations;

/// One translated entry of a catalog
#[derive(Debug, Clone, Serialize)]
pub struct PreviewEntry {
    pub context: Option<String>,
    pub msgid: String,
    pub msgid_plural: Option<String>,
    pub msgstr: String,
    pub msgstr_plural: Vec<String>,
    /// Extracted comments: Lingui keeps a plural's ICU message in one
    pub comments: Vec<String>,
}

/// A catalog read from disk, for the window to compile
#[derive(Debug, Clone, Serialize)]
pub struct PreviewCatalog {
    pub path: String,
    /// From the `Language` header, such as `es`
    pub language: String,
    pub plural_forms: Option<String>,
    /// Translated entries only: untranslated and fuzzy ones show English
    pub entries: Vec<PreviewEntry>,
}

/// The preview in progress, if any
#[derive(Default)]
pub struct TranslationPreview {
    watch: Mutex<Option<CancellationToken>>,
}

/// Read a catalog: its language and translated entries, and its text for
/// the menus
fn read_catalog(path: &Path) -> Result<(PreviewCatalog, String), AppError> {
    let text = std::fs::read_to_string(path)?;
    let file = rspolib::pofile(text.as_str()).map_err(|e| {
        AppError::InvalidConfig(format!("{} isn't a valid catalog: {e}", path.display()))
    })?;
    let language = file
        .metadata
        .get("Language")
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .ok_or_else(|| {
            AppError::InvalidConfig(format!("{} has no Language header", path.display()))
        })?;
    let entries = file
        .entries
        .into_iter()
        .filter(|e| !e.obsolete && !e.fuzzy())
        .filter_map(|e| {
            let msgstr = e.msgstr.clone().unwrap_or_default();
            let translated = !msgstr.is_empty() || e.msgstr_plural.iter().any(|s| !s.is_empty());
            translated.then(|| PreviewEntry {
                context: e.msgctxt,
                msgid: e.msgid,
                msgid_plural: e.msgid_plural,
                msgstr,
                msgstr_plural: e.msgstr_plural,
                comments: e
                    .comment
                    .map(|c| c.lines().map(str::to_string).collect())
                    .unwrap_or_default(),
            })
        })
        .collect();
    let catalog = PreviewCatalog {
        path: path.display().to_string(),
        language,
        plural_forms: file.metadata.get("Plural-Forms").cloned(),
        entries,
    };
    Ok((catalog, text))
}

/// Show a catalog in the menus and every window
fn apply(app: &AppHandle, catalog: &PreviewCatalog, text: &str) {
    if let Err(e) = crate::menu::set_menu_translations(app, &Translations::parse(text)) {
        tracing::warn!("menu from {}: {e}", catalog.path);
    }
    let _ = app.emit("translation-preview", catalog);
}

/// Load a catalog from disk, show it, and show it again whenever the file
/// changes, until `stop_translation_preview`
#[tauri::command]
pub async fn load_translation_preview(
    app: AppHandle,
    preview: State<'_, TranslationPreview>,
    path: String,
) -> Result<PreviewCatalog, AppError> {
    let path = PathBuf::from(path);
    let (catalog, text) = read_catalog(&path)?;
    apply(&app, &catalog, &text);

    let token = CancellationToken::new();
    if let Some(old) = preview.watch.lock().unwrap().replace(token.clone()) {
        old.cancel();
    }
    tauri::async_runtime::spawn(watch(app, path, token));
    Ok(catalog)
}

/// Stop previewing: the menus and windows go back to the language setting
#[tauri::command]
pub async fn stop_translation_preview(
    app: AppHandle,
    preview: State<'_, TranslationPreview>,
) -> Result<(), AppError> {
    if let Some(token) = preview.watch.lock().unwrap().take() {
        token.cancel();
    }
    let language = crate::commands::config::read_language(&app);
    crate::commands::config::apply_language(&app, &language).await;
    Ok(())
}

/// Check the file's modification time every half second, and show it again
/// when it changes.  A save that leaves the file unreadable for a moment
/// keeps the last good catalog.
async fn watch(app: AppHandle, path: PathBuf, token: CancellationToken) {
    let modified = |p: &Path| std::fs::metadata(p).and_then(|m| m.modified()).ok();
    let mut last = modified(&path);
    loop {
        tokio::select! {
            _ = token.cancelled() => return,
            _ = tokio::time::sleep(std::time::Duration::from_millis(500)) => {}
        }
        let now = modified(&path);
        if now.is_some() && now != last {
            last = now;
            match read_catalog(&path) {
                Ok((catalog, text)) => apply(&app, &catalog, &text),
                Err(e) => tracing::warn!("translation preview: {e}"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_catalog_gives_its_translated_entries() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("es.po");
        std::fs::write(
            &path,
            r##"msgid ""
msgstr ""
"Language: es\n"
"Content-Type: text/plain; charset=UTF-8\n"
"Plural-Forms: nplurals=2; plural=(n != 1);\n"

msgid "Run all"
msgstr "Ejecutar todo"

msgctxt "error.division_by_zero.title"
msgid "Division by Zero"
msgstr "División por cero"

#, fuzzy
msgid "Save"
msgstr "Salvar"

msgid "Edit"
msgstr ""

#. js-lingui:icu=%7Bcount%2C+plural%2C+one+%7B%23+cell%7D+other+%7B%23+cells%7D%7D&pluralize_on=count
msgid "# cell"
msgid_plural "# cells"
msgstr[0] "# celda"
msgstr[1] "# celdas"
"##,
        )
        .unwrap();
        let (catalog, _) = read_catalog(&path).unwrap();
        assert_eq!(catalog.language, "es");
        let ids: Vec<_> = catalog.entries.iter().map(|e| e.msgid.as_str()).collect();
        assert_eq!(ids, ["Run all", "Division by Zero", "# cell"]);
        assert_eq!(
            catalog.entries[1].context.as_deref(),
            Some("error.division_by_zero.title")
        );
        let plural = &catalog.entries[2];
        assert_eq!(plural.msgstr_plural, ["# celda", "# celdas"]);
        assert!(plural.comments[0].starts_with("js-lingui:icu="));
    }
}
