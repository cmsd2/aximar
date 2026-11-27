use aximar_core::msg;
use std::sync::atomic::{AtomicU32, Ordering};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::state::AppState;

static WINDOW_COUNTER: AtomicU32 = AtomicU32::new(1);

#[tauri::command]
pub async fn create_window(app: AppHandle, notebook_id: Option<String>) -> Result<String, String> {
    let n = WINDOW_COUNTER.fetch_add(1, Ordering::Relaxed);
    let label = format!("window-{n}");

    let url = match &notebook_id {
        Some(id) => WebviewUrl::App(format!("index.html?notebook={id}").into()),
        None => WebviewUrl::App("index.html".into()),
    };

    WebviewWindowBuilder::new(&app, &label, url)
        .title("Aximar")
        .inner_size(1200.0, 800.0)
        .min_inner_size(800.0, 600.0)
        .build()
        .map_err(|e| format!("Failed to create window: {e}"))?;

    Ok(label)
}

/// Return the file paths passed as CLI arguments on initial launch.
/// Returns a clone each time — the React strict mode double-mount pattern
/// requires the value to survive across mount/cleanup/remount cycles.
#[tauri::command]
pub async fn get_initial_file_args(
    state: tauri::State<'_, AppState>,
) -> Result<Option<Vec<String>>, String> {
    Ok(state.initial_file_args.lock().await.clone())
}

/// The licences window: its label, page and size
pub const LICENCES_LABEL: &str = "licences";
pub const LICENCES_PAGE: &str = "licenses/index.html";
pub const LICENCES_SIZE: (f64, f64) = (720.0, 640.0);

fn licences_url() -> WebviewUrl {
    WebviewUrl::App(LICENCES_PAGE.into())
}

/// Show the open-source licences Aximar ships, in a window of their own:
/// focus it if it is already open.  Every page in it is in the app
/// bundle, so it works offline.  See docs/licences.md.
#[tauri::command]
pub async fn open_licences(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(LICENCES_LABEL) {
        let _ = window.unminimize();
        return window.set_focus().map_err(|e| e.to_string());
    }
    let language = crate::commands::config::read_language(&app);
    let tr = crate::i18n::Translations::for_language(&language);
    // i18n: The title of the window that shows the open-source licences
    let title = tr.translate(&msg!("Licences"));
    let (width, height) = LICENCES_SIZE;
    WebviewWindowBuilder::new(&app, LICENCES_LABEL, licences_url())
        .title(title)
        .inner_size(width, height)
        .min_inner_size(480.0, 360.0)
        .build()
        .map_err(|e| format!("Failed to open the licences: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_licences_window_shows_the_bundled_index() {
        match licences_url() {
            WebviewUrl::App(path) => assert_eq!(path.to_str(), Some("licenses/index.html")),
            other => panic!("expected an app page, got {other:?}"),
        }
        assert_eq!(LICENCES_SIZE, (720.0, 640.0));
        let index = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../public")
            .join(LICENCES_PAGE);
        assert!(index.exists(), "{} is missing", index.display());
    }
}
