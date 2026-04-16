use tauri::{
    menu::{AboutMetadata, MenuBuilder, MenuItemBuilder, SubmenuBuilder},
    webview::WebviewWindowBuilder,
    Emitter, Manager, WebviewUrl,
};

use std::sync::atomic::{AtomicU32, Ordering};

static WINDOW_COUNTER: AtomicU32 = AtomicU32::new(1);

pub fn setup_menu(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // --- Custom menu items ---

    let new_item = MenuItemBuilder::new("New Notebook")
        .id("new")
        .accelerator("CmdOrCtrl+N")
        .build(app)?;

    let new_window_item = MenuItemBuilder::new("New Window")
        .id("new_window")
        .accelerator("CmdOrCtrl+Shift+N")
        .build(app)?;

    let open_item = MenuItemBuilder::new("Open...")
        .id("open")
        .accelerator("CmdOrCtrl+O")
        .build(app)?;

    let save_item = MenuItemBuilder::new("Save")
        .id("save")
        .accelerator("CmdOrCtrl+S")
        .build(app)?;

    let save_as_item = MenuItemBuilder::new("Save As...")
        .id("save_as")
        .accelerator("CmdOrCtrl+Shift+S")
        .build(app)?;

    let export_latex_item = MenuItemBuilder::new("Export as LaTeX...")
        .id("export_latex")
        .accelerator("CmdOrCtrl+Shift+E")
        .build(app)?;

    let print_item = MenuItemBuilder::new("Print...")
        .id("print")
        .accelerator("CmdOrCtrl+P")
        .build(app)?;

    // --- App submenu (macOS: appears under app name) ---

    let app_submenu = SubmenuBuilder::new(app, "Aximar")
        .about(Some(AboutMetadata {
            ..Default::default()
        }))
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .quit()
        .build()?;

    // --- File submenu ---

    let file_submenu = SubmenuBuilder::new(app, "File")
        .item(&new_item)
        .item(&new_window_item)
        .item(&open_item)
        .separator()
        .item(&save_item)
        .item(&save_as_item)
        .separator()
        .item(&export_latex_item)
        .separator()
        .item(&print_item)
        .separator()
        .close_window()
        .build()?;

    // --- Edit submenu ---

    let edit_submenu = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;

    // --- Window submenu ---

    let window_submenu = SubmenuBuilder::new(app, "Window")
        .minimize()
        .build()?;

    // --- Assemble menu bar ---

    let menu = MenuBuilder::new(app)
        .items(&[&app_submenu, &file_submenu, &edit_submenu, &window_submenu])
        .build()?;

    app.set_menu(menu)?;

    // --- Handle menu events → emit to frontend ---

    app.on_menu_event(move |app_handle, event| {
        let id = event.id().0.as_str();
        match id {
            "new_window" => {
                let app_handle = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    let n = WINDOW_COUNTER.fetch_add(1, Ordering::Relaxed);
                    let label = format!("window-{n}");
                    let _ = WebviewWindowBuilder::new(
                        &app_handle,
                        &label,
                        WebviewUrl::App("index.html".into()),
                    )
                    .title("Aximar")
                    .inner_size(1200.0, 800.0)
                    .min_inner_size(800.0, 600.0)
                    .build();
                });
            }
            "new" | "open" | "save" | "save_as" | "export_latex" => {
                // Emit only to the focused window so actions don't leak across windows
                let focused = app_handle
                    .webview_windows()
                    .into_values()
                    .find(|w| w.is_focused().unwrap_or(false));
                if let Some(window) = focused {
                    let _ = window.emit("menu-event", id);
                }
            }
            "print" => {
                let focused = app_handle
                    .webview_windows()
                    .into_values()
                    .find(|w| w.is_focused().unwrap_or(false));
                if let Some(window) = focused {
                    let _ = window.print();
                }
            }
            _ => {}
        }
    });

    Ok(())
}
