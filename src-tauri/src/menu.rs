use tauri::{
    menu::{AboutMetadata, MenuBuilder, MenuItemBuilder, SubmenuBuilder},
    Emitter, Manager,
};

pub fn setup_menu(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // --- Custom menu items ---

    let new_item = MenuItemBuilder::new("New Notebook")
        .id("new")
        .accelerator("CmdOrCtrl+N")
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
        .item(&open_item)
        .separator()
        .item(&save_item)
        .item(&save_as_item)
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
            "new" | "open" | "save" | "save_as" => {
                let _ = app_handle.emit("menu-event", id);
            }
            "print" => {
                if let Some(window) = app_handle.get_webview_window("main") {
                    let _ = window.print();
                }
            }
            _ => {}
        }
    });

    Ok(())
}
