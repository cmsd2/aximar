use std::sync::atomic::{AtomicU32, Ordering};
use tauri::{AppHandle, WebviewUrl, WebviewWindowBuilder};

use crate::state::AppState;

static WINDOW_COUNTER: AtomicU32 = AtomicU32::new(1);

#[tauri::command]
pub async fn create_window(
    app: AppHandle,
    notebook_id: Option<String>,
) -> Result<String, String> {
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

/// Return and consume the file paths passed as CLI arguments on initial launch.
/// Returns `None` on all subsequent calls (take-once semantics).
#[tauri::command]
pub async fn get_initial_file_args(
    state: tauri::State<'_, AppState>,
) -> Result<Option<Vec<String>>, String> {
    Ok(state.initial_file_args.lock().await.take())
}
