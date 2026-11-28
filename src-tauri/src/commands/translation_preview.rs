//! Translation preview, for translators: load a `.po` from disk, show it in
//! the window and the menus, and show it again each time the file is saved.
//!
//! In every build, so a translator can check a translation in the app as
//! released.  See docs/i18n.md.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use notify_debouncer_mini::notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, DebouncedEventBatch, Debouncer};

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

/// Show the file again each time it changes, until `token` is cancelled.  A
/// save that doesn't parse keeps the last good catalog, and the window says
/// why.
async fn watch(app: AppHandle, path: PathBuf, token: CancellationToken) {
    watch_file(&path, token, || match read_catalog(&path) {
        Ok((catalog, text)) => apply(&app, &catalog, &text),
        Err(e) => {
            tracing::warn!("translation preview: {e}");
            let _ = app.emit("translation-preview-error", e.to_string());
        }
    })
    .await;
}

/// How long the native watcher waits for an editor's burst of events to end
const DEBOUNCE: Duration = Duration::from_millis(200);
/// How often to check the modified time where the native watcher is running:
/// a safety net for filesystems that send no events, such as network mounts,
/// WSL2's view of Windows drives, or files another user owns on macOS
const SAFETY_NET: Duration = Duration::from_secs(2);
/// How often to check it where the native watcher couldn't start
const POLL: Duration = Duration::from_millis(500);

/// Call `changed` each time the file at `path` changes, until `token` is
/// cancelled.
///
/// The operating system's own events come through `notify` (FSEvents on
/// macOS, inotify on Linux, ReadDirectoryChangesW on Windows, kqueue on the
/// BSDs).  It watches the file's folder rather than the file: many editors
/// save by writing a new file and renaming it over the old one, which ends a
/// watch on the file itself under inotify and kqueue.  The modified time is
/// checked too, every 2 seconds, for filesystems that send no events; where
/// the native watcher can't start, every half second.
async fn watch_file(path: &Path, token: CancellationToken, mut changed: impl FnMut()) {
    let modified = |p: &Path| std::fs::metadata(p).and_then(|m| m.modified()).ok();
    let (tx, mut events) = tokio::sync::mpsc::unbounded_channel::<()>();
    // Held for as long as the loop runs: dropping it stops the watcher
    let native = native_watcher(path, tx);
    let interval = if native.is_some() { SAFETY_NET } else { POLL };
    let mut last = modified(path);
    loop {
        let event = tokio::select! {
            _ = token.cancelled() => return,
            Some(()) = events.recv() => true,
            _ = tokio::time::sleep(interval) => false,
        };
        let now = modified(path);
        // An event means a change even if the modified time can't show it,
        // as on a filesystem with coarse times; a timer tick needs the time
        // to have moved
        if now.is_some() && (event || now != last) {
            last = now;
            changed();
        }
    }
}

/// A native watcher on `path`'s folder that sends on `tx` when `path` itself
/// changes, or None where it can't start (no inotify watches left, an
/// emulated system without the native backend)
fn native_watcher(
    path: &Path,
    tx: tokio::sync::mpsc::UnboundedSender<()>,
) -> Option<Debouncer<RecommendedWatcher>> {
    let name = path.file_name()?.to_owned();
    let folder = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let handler = move |result: DebounceEventResult| match result {
        // Compare names alone: macOS reports the folder's canonical path,
        // such as /private/var for /var
        Ok(DebouncedEventBatch::Events(events)) => {
            if events
                .iter()
                .any(|e| e.path.file_name() == Some(name.as_os_str()))
            {
                let _ = tx.send(());
            }
        }
        // Events were lost, or a kind of batch added since: the file may
        // have changed
        Ok(_) => {
            let _ = tx.send(());
        }
        Err(e) => tracing::warn!("translation preview watcher: {e}"),
    };
    let mut debouncer = match new_debouncer(DEBOUNCE, handler) {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!("translation preview: no file watcher ({e}); checking the file instead");
            return None;
        }
    };
    if let Err(e) = debouncer
        .watcher()
        .watch(folder, RecursiveMode::NonRecursive)
    {
        tracing::warn!(
            "translation preview: can't watch {} ({e}); checking the file instead",
            folder.display()
        );
        return None;
    }
    Some(debouncer)
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

    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// Watch `path` in the background, counting the changes seen
    fn start(
        path: PathBuf,
    ) -> (
        Arc<AtomicUsize>,
        CancellationToken,
        tokio::task::JoinHandle<()>,
    ) {
        let seen = Arc::new(AtomicUsize::new(0));
        let token = CancellationToken::new();
        let task = {
            let (seen, token) = (seen.clone(), token.clone());
            tokio::spawn(async move {
                watch_file(&path, token, || {
                    seen.fetch_add(1, Ordering::SeqCst);
                })
                .await
            })
        };
        (seen, token, task)
    }

    /// Wait up to 10 seconds for `seen` to reach `n`
    async fn until(seen: &AtomicUsize, n: usize) {
        for _ in 0..100 {
            if seen.load(Ordering::SeqCst) >= n {
                return;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        panic!("saw {} changes, wanted {n}", seen.load(Ordering::SeqCst));
    }

    /// The operating system's events, without the safety net's checks
    #[tokio::test]
    async fn the_native_watcher_reports_saves_of_both_kinds() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("es.po");
        std::fs::write(&path, "one").unwrap();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let _watcher = native_watcher(&path, tx).expect("a native watcher on this platform");
        tokio::time::sleep(Duration::from_millis(500)).await;
        let wait = Duration::from_secs(10);

        std::fs::write(&path, "two").unwrap();
        tokio::time::timeout(wait, rx.recv())
            .await
            .expect("an event for an in-place save");
        // Let the debouncer settle, and drop anything left from that save
        tokio::time::sleep(Duration::from_millis(500)).await;
        while rx.try_recv().is_ok() {}

        let tmp = dir.path().join(".es.po.tmp");
        std::fs::write(&tmp, "three").unwrap();
        std::fs::rename(&tmp, &path).unwrap();
        tokio::time::timeout(wait, rx.recv())
            .await
            .expect("an event for a save that renames over the file");
    }

    #[tokio::test]
    async fn an_in_place_save_is_seen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("es.po");
        std::fs::write(&path, "one").unwrap();
        let (seen, token, task) = start(path.clone());
        // Let the watcher start
        tokio::time::sleep(Duration::from_millis(500)).await;

        std::fs::write(&path, "two").unwrap();
        until(&seen, 1).await;
        token.cancel();
        task.await.unwrap();
    }

    #[tokio::test]
    async fn saves_that_rename_a_new_file_over_the_old_are_seen_each_time() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("es.po");
        std::fs::write(&path, "one").unwrap();
        let (seen, token, task) = start(path.clone());
        tokio::time::sleep(Duration::from_millis(500)).await;

        // As editors that write a temporary file and rename it into place
        for (n, text) in [(1, "two"), (2, "three")] {
            let tmp = dir.path().join(format!(".es.po.{n}.tmp"));
            std::fs::write(&tmp, text).unwrap();
            std::fs::rename(&tmp, &path).unwrap();
            until(&seen, n).await;
        }
        token.cancel();
        task.await.unwrap();
    }

    #[tokio::test]
    async fn other_files_in_the_folder_are_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("es.po");
        std::fs::write(&path, "one").unwrap();
        let (seen, token, task) = start(path.clone());
        tokio::time::sleep(Duration::from_millis(500)).await;

        std::fs::write(dir.path().join("de.po"), "anders").unwrap();
        // Longer than the debounce and the safety net's check
        tokio::time::sleep(SAFETY_NET + Duration::from_millis(500)).await;
        assert_eq!(seen.load(Ordering::SeqCst), 0);
        token.cancel();
        task.await.unwrap();
    }

    #[tokio::test]
    async fn cancelling_stops_the_watch() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("es.po");
        std::fs::write(&path, "one").unwrap();
        let (_seen, token, task) = start(path);
        token.cancel();
        tokio::time::timeout(Duration::from_secs(5), task)
            .await
            .unwrap()
            .unwrap();
    }
}
