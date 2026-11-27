use aximar_core::msg;
use tauri::{
    menu::{AboutMetadata, MenuBuilder, MenuItemBuilder, SubmenuBuilder},
    webview::WebviewWindowBuilder,
    AppHandle, Emitter, Manager, WebviewUrl,
};

use crate::i18n::Translations;

use std::sync::atomic::{AtomicU32, Ordering};

static WINDOW_COUNTER: AtomicU32 = AtomicU32::new(1);

/// The menu bar's text in one language
struct MenuLabels {
    about: String,
    services: String,
    hide: String,
    hide_others: String,
    show_all: String,
    quit: String,
    file: String,
    new_notebook: String,
    new_window: String,
    open: String,
    save: String,
    save_as: String,
    export_latex: String,
    print: String,
    close_window: String,
    edit: String,
    undo: String,
    redo: String,
    cut: String,
    copy: String,
    paste: String,
    select_all: String,
    window: String,
    minimize: String,
}

impl MenuLabels {
    fn new(tr: &Translations) -> Self {
        let t = |m| tr.translate(&m);
        MenuLabels {
            // i18n: App menu (macOS). max-length:30
            about: t(msg!("About Aximar")),
            // i18n: App menu (macOS): the system's Services submenu. max-length:30
            services: t(msg!("Services")),
            // i18n: App menu (macOS). max-length:30
            hide: t(msg!("Hide Aximar")),
            // i18n: App menu (macOS): hides every other app. max-length:30
            hide_others: t(msg!("Hide Others")),
            // i18n: App menu (macOS). max-length:30
            show_all: t(msg!("Show All")),
            // i18n: App menu. max-length:30
            quit: t(msg!("Quit Aximar")),
            // i18n: Menu bar. max-length:16
            file: t(msg!("File")),
            // i18n: File menu. max-length:30
            new_notebook: t(msg!("New Notebook")),
            // i18n: File menu. max-length:30
            new_window: t(msg!("New Window")),
            // i18n: File menu: opens a notebook file. max-length:30
            open: t(msg!("Open...")),
            // i18n: File menu. max-length:30
            save: t(msg!("Save")),
            // i18n: File menu. max-length:30
            save_as: t(msg!("Save As...")),
            // i18n: File menu: saves the notebook as a LaTeX document. max-length:30
            export_latex: t(msg!("Export as LaTeX...")),
            // i18n: File menu. max-length:30
            print: t(msg!("Print...")),
            // i18n: File menu. max-length:30
            close_window: t(msg!("Close Window")),
            // i18n: Menu bar. max-length:16
            edit: t(msg!("Edit")),
            // i18n: Edit menu. max-length:30
            undo: t(msg!("Undo")),
            // i18n: Edit menu. max-length:30
            redo: t(msg!("Redo")),
            // i18n: Edit menu. max-length:30
            cut: t(msg!("Cut")),
            // i18n: Edit menu. max-length:30
            copy: t(msg!("Copy")),
            // i18n: Edit menu. max-length:30
            paste: t(msg!("Paste")),
            // i18n: Edit menu. max-length:30
            select_all: t(msg!("Select All")),
            // i18n: Menu bar. max-length:16
            window: t(msg!("Window")),
            // i18n: Window menu. max-length:30
            minimize: t(msg!("Minimize")),
        }
    }
}

/// Build the menu bar in `language` and use it: at start, and again when
/// the language changes
pub fn set_menu_language(app: &AppHandle, language: &str) -> tauri::Result<()> {
    set_menu_translations(app, &Translations::for_language(language))
}

/// Build the menu bar from `translations` and use it, as for a translation
/// preview
pub fn set_menu_translations(app: &AppHandle, translations: &Translations) -> tauri::Result<()> {
    let l = MenuLabels::new(translations);

    // --- Custom menu items ---

    let new_item = MenuItemBuilder::new(&l.new_notebook)
        .id("new")
        .accelerator("CmdOrCtrl+N")
        .build(app)?;

    let new_window_item = MenuItemBuilder::new(&l.new_window)
        .id("new_window")
        .accelerator("CmdOrCtrl+Shift+N")
        .build(app)?;

    let open_item = MenuItemBuilder::new(&l.open)
        .id("open")
        .accelerator("CmdOrCtrl+O")
        .build(app)?;

    let save_item = MenuItemBuilder::new(&l.save)
        .id("save")
        .accelerator("CmdOrCtrl+S")
        .build(app)?;

    let save_as_item = MenuItemBuilder::new(&l.save_as)
        .id("save_as")
        .accelerator("CmdOrCtrl+Shift+S")
        .build(app)?;

    let export_latex_item = MenuItemBuilder::new(&l.export_latex)
        .id("export_latex")
        .accelerator("CmdOrCtrl+Shift+E")
        .build(app)?;

    let print_item = MenuItemBuilder::new(&l.print)
        .id("print")
        .accelerator("CmdOrCtrl+P")
        .build(app)?;

    // --- App submenu (macOS: appears under app name) ---

    let app_submenu = SubmenuBuilder::new(app, "Aximar")
        .about_with_text(
            &l.about,
            Some(AboutMetadata {
                ..Default::default()
            }),
        )
        .separator()
        .services_with_text(&l.services)
        .separator()
        .hide_with_text(&l.hide)
        .hide_others_with_text(&l.hide_others)
        .show_all_with_text(&l.show_all)
        .separator()
        .quit_with_text(&l.quit)
        .build()?;

    // --- File submenu ---

    let file_submenu = SubmenuBuilder::new(app, &l.file)
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
        .close_window_with_text(&l.close_window)
        .build()?;

    // --- Edit submenu ---

    let edit_submenu = SubmenuBuilder::new(app, &l.edit)
        .undo_with_text(&l.undo)
        .redo_with_text(&l.redo)
        .separator()
        .cut_with_text(&l.cut)
        .copy_with_text(&l.copy)
        .paste_with_text(&l.paste)
        .select_all_with_text(&l.select_all)
        .build()?;

    // --- Window submenu ---

    let window_submenu = SubmenuBuilder::new(app, &l.window)
        .minimize_with_text(&l.minimize)
        .build()?;

    // --- Assemble menu bar ---

    let menu = MenuBuilder::new(app)
        .items(&[&app_submenu, &file_submenu, &edit_submenu, &window_submenu])
        .build()?;

    app.set_menu(menu)?;
    Ok(())
}

pub fn setup_menu(app: &mut tauri::App, language: &str) -> Result<(), Box<dyn std::error::Error>> {
    set_menu_language(app.handle(), language)?;

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

#[cfg(test)]
mod tests {
    use super::*;

    const ES: &str = r#"msgid ""
msgstr ""
"Language: es\n"
"Content-Type: text/plain; charset=UTF-8\n"

msgid "File"
msgstr "Archivo"

msgid "Save As..."
msgstr "Guardar como..."

msgid "Quit Aximar"
msgstr "Salir de Aximar"
"#;

    #[test]
    fn menu_labels_come_from_the_catalog() {
        let l = MenuLabels::new(&Translations::parse(ES));
        assert_eq!(l.file, "Archivo");
        assert_eq!(l.save_as, "Guardar como...");
        assert_eq!(l.quit, "Salir de Aximar");
        // Not translated yet: English
        assert_eq!(l.edit, "Edit");
    }

    #[test]
    fn english_is_the_source_text() {
        let l = MenuLabels::new(&Translations::for_language("en"));
        assert_eq!(l.file, "File");
        assert_eq!(l.export_latex, "Export as LaTeX...");
    }
}
