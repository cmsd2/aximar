use aximar_core::msg;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{
    menu::{
        AboutMetadata, CheckMenuItemBuilder, IsMenuItem, Menu, MenuBuilder, MenuItemBuilder,
        MenuItemKind, PredefinedMenuItem, Submenu, SubmenuBuilder,
    },
    AppHandle, Emitter, Manager, Runtime,
};

use crate::i18n::Translations;

/// Set once a window has sent its menu: from then on the windows own the
/// menu bar, and the built-in menu isn't rebuilt over it
static WINDOW_MENU: AtomicBool = AtomicBool::new(false);

/// The label of the window that last gained focus.  On Windows a window
/// stops counting as focused once its page takes the keyboard, as WebView2
/// does on a click, so the menu falls back to this one.
static LAST_FOCUSED: Mutex<Option<String>> = Mutex::new(None);

/// Note that a window gained focus (lib.rs's window events)
pub fn note_focus(label: &str) {
    if let Ok(mut last) = LAST_FOCUSED.lock() {
        *last = Some(label.to_string());
    }
}

/// The window a menu item runs its command in: the focused one, or else
/// the one that last gained focus, if it's still open
fn menu_target<R: Runtime>(app: &AppHandle<R>) -> Option<tauri::WebviewWindow<R>> {
    let windows = app.webview_windows();
    if let Some(window) = windows.values().find(|w| w.is_focused().unwrap_or(false)) {
        return Some(window.clone());
    }
    let last = LAST_FOCUSED.lock().ok()?.clone()?;
    windows.get(&last).cloned()
}

/// The menu bar's text in one language
struct MenuLabels {
    about: String,
    licences_credit: String,
    settings: String,
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
            // i18n: The About panel (macOS and Linux): where to find the open-source licences. Settings, General and Licences name the window, section and row
            licences_credit: t(msg!("Open-source licences: Settings, General, Licences")),
            // i18n: App menu on macOS, Edit menu elsewhere: opens the Settings window. max-length:24
            settings: t(msg!("Settings...")),
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

/// Build the built-in menu bar from `translations` and use it, as for a
/// translation preview.  It stands in until the first window sends its own
/// menu; after that a language change reaches the menu through the windows.
pub fn set_menu_translations(app: &AppHandle, translations: &Translations) -> tauri::Result<()> {
    // Where the page draws the menu bar there is none (commands/window.rs)
    if crate::commands::window::page_draws_frame() || WINDOW_MENU.load(Ordering::Acquire) {
        return Ok(());
    }
    let l = MenuLabels::new(translations);

    // --- Custom menu items ---

    let new_item = MenuItemBuilder::new(&l.new_notebook)
        .id("file.new")
        .accelerator("CmdOrCtrl+N")
        .build(app)?;

    let new_window_item = MenuItemBuilder::new(&l.new_window)
        .id("file.new_window")
        .accelerator("CmdOrCtrl+Shift+N")
        .build(app)?;

    let open_item = MenuItemBuilder::new(&l.open)
        .id("file.open")
        .accelerator("CmdOrCtrl+O")
        .build(app)?;

    let save_item = MenuItemBuilder::new(&l.save)
        .id("file.save")
        .accelerator("CmdOrCtrl+S")
        .build(app)?;

    let save_as_item = MenuItemBuilder::new(&l.save_as)
        .id("file.save_as")
        .accelerator("CmdOrCtrl+Shift+S")
        .build(app)?;

    let export_latex_item = MenuItemBuilder::new(&l.export_latex)
        .id("file.export_latex")
        .accelerator("CmdOrCtrl+Shift+E")
        .build(app)?;

    let settings_item = MenuItemBuilder::new(&l.settings)
        .id("app.settings")
        .accelerator("CmdOrCtrl+,")
        .build(app)?;

    let print_item = MenuItemBuilder::new(&l.print)
        .id("file.print")
        .accelerator("CmdOrCtrl+P")
        .build(app)?;

    // --- App submenu (macOS: appears under app name) ---

    // Settings goes in the app menu on macOS, as Mac apps' do, and at the
    // end of the Edit menu elsewhere
    let mac = cfg!(target_os = "macos");

    let mut app_submenu = SubmenuBuilder::new(app, "Aximar")
        .about_with_text(
            &l.about,
            Some(AboutMetadata {
                // macOS shows credits, Linux comments
                credits: Some(l.licences_credit.clone()),
                comments: Some(l.licences_credit.clone()),
                license: Some("GPL-3.0-or-later".into()),
                ..Default::default()
            }),
        )
        .separator();
    if mac {
        app_submenu = app_submenu.item(&settings_item).separator();
    }
    let app_submenu = app_submenu
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

    let mut edit_submenu = SubmenuBuilder::new(app, &l.edit)
        .undo_with_text(&l.undo)
        .redo_with_text(&l.redo)
        .separator()
        .cut_with_text(&l.cut)
        .copy_with_text(&l.copy)
        .paste_with_text(&l.paste)
        .select_all_with_text(&l.select_all);
    if !mac {
        edit_submenu = edit_submenu.separator().item(&settings_item);
    }
    let edit_submenu = edit_submenu.build()?;

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

    // Every item runs its command in the focused window, which owns the
    // command list (src/commands/registry.ts).  With no window open, New
    // Window makes one.
    app.on_menu_event(move |app_handle, event| {
        let id = event.id().0.clone();
        match menu_target(app_handle) {
            Some(window) => {
                let _ = window.emit("menu-command", MenuCommand { id });
            }
            None if id == "file.new_window" => {
                let app_handle = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = crate::commands::window::create_window(app_handle, None).await;
                });
            }
            None => {}
        }
    });

    Ok(())
}

/// The `menu-command` event: the id of the item the user chose
#[derive(Clone, serde::Serialize)]
struct MenuCommand {
    id: String,
}

/// A menu bar as a window describes it (src/commands/menu-model.ts)
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MenuNode {
    Submenu {
        label: String,
        items: Vec<MenuNode>,
    },
    Item {
        id: String,
        label: String,
        accelerator: Option<String>,
        enabled: bool,
        /// Present for a check item
        checked: Option<bool>,
    },
    Predefined {
        which: Predefined,
        /// The item's text, where the system lets an app set it
        label: Option<String>,
    },
}

/// The system's own items: the clipboard, the app menu, and window sizing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Predefined {
    Separator,
    Cut,
    Copy,
    Paste,
    SelectAll,
    Services,
    Hide,
    HideOthers,
    ShowAll,
    Quit,
    About,
    Minimize,
    Maximize,
}

/// One item's state, sent when the focused window's state changes
#[derive(Debug, Deserialize)]
pub struct ItemState {
    pub id: String,
    pub enabled: bool,
    pub checked: Option<bool>,
}

/// A built menu's custom items by command id, as `Menu::get` finds only
/// top-level items
pub type ItemsById<R> = HashMap<String, MenuItemKind<R>>;

/// The custom items of the window-built menu, kept for `set_menu_state`
#[derive(Default)]
pub struct MenuItems(Mutex<ItemsById<tauri::Wry>>);

fn predefined<R: Runtime, M: Manager<R>>(
    app: &M,
    which: Predefined,
    label: Option<&str>,
) -> tauri::Result<PredefinedMenuItem<R>> {
    use PredefinedMenuItem as P;
    match which {
        Predefined::Separator => P::separator(app),
        Predefined::Cut => P::cut(app, label),
        Predefined::Copy => P::copy(app, label),
        Predefined::Paste => P::paste(app, label),
        Predefined::SelectAll => P::select_all(app, label),
        Predefined::Services => P::services(app, label),
        Predefined::Hide => P::hide(app, label),
        Predefined::HideOthers => P::hide_others(app, label),
        Predefined::ShowAll => P::show_all(app, label),
        Predefined::Quit => P::quit(app, label),
        Predefined::About => P::about(
            app,
            label,
            Some(AboutMetadata {
                license: Some("GPL-3.0-or-later".into()),
                ..Default::default()
            }),
        ),
        Predefined::Minimize => P::minimize(app, label),
        Predefined::Maximize => P::maximize(app, label),
    }
}

/// Build one node; custom items are recorded in `items` by id
fn build_node<R: Runtime, M: Manager<R>>(
    app: &M,
    node: &MenuNode,
    items: &mut ItemsById<R>,
) -> tauri::Result<Box<dyn IsMenuItem<R>>> {
    Ok(match node {
        MenuNode::Submenu {
            label,
            items: children,
        } => {
            let built = children
                .iter()
                .map(|c| build_node(app, c, items))
                .collect::<tauri::Result<Vec<_>>>()?;
            let refs: Vec<&dyn IsMenuItem<R>> = built.iter().map(|b| b.as_ref()).collect();
            Box::new(Submenu::with_items(app, label, true, &refs)?)
        }
        MenuNode::Item {
            id,
            label,
            accelerator,
            enabled,
            checked: Some(checked),
        } => {
            let mut b = CheckMenuItemBuilder::with_id(id.as_str(), label)
                .enabled(*enabled)
                .checked(*checked);
            if let Some(a) = accelerator {
                b = b.accelerator(a);
            }
            let item = b.build(app)?;
            items.insert(id.clone(), MenuItemKind::Check(item.clone()));
            Box::new(item)
        }
        MenuNode::Item {
            id,
            label,
            accelerator,
            enabled,
            checked: None,
        } => {
            let mut b = MenuItemBuilder::with_id(id.as_str(), label).enabled(*enabled);
            if let Some(a) = accelerator {
                b = b.accelerator(a);
            }
            let item = b.build(app)?;
            items.insert(id.clone(), MenuItemKind::MenuItem(item.clone()));
            Box::new(item)
        }
        MenuNode::Predefined { which, label } => {
            Box::new(predefined(app, *which, label.as_deref())?)
        }
    })
}

/// Build a menu bar from a window's model, with its custom items by id
pub fn build_menu<R: Runtime, M: Manager<R>>(
    app: &M,
    model: &[MenuNode],
) -> tauri::Result<(Menu<R>, ItemsById<R>)> {
    let mut items = HashMap::new();
    let built = model
        .iter()
        .map(|n| build_node(app, n, &mut items))
        .collect::<tauri::Result<Vec<_>>>()?;
    let refs: Vec<&dyn IsMenuItem<R>> = built.iter().map(|b| b.as_ref()).collect();
    let menu = MenuBuilder::new(app).items(&refs).build()?;
    Ok((menu, items))
}

/// Set one built item's state; false when it is a predefined item or unknown
pub fn apply_state<R: Runtime>(item: &MenuItemKind<R>, state: &ItemState) -> tauri::Result<bool> {
    match item {
        MenuItemKind::MenuItem(i) => i.set_enabled(state.enabled)?,
        MenuItemKind::Check(i) => {
            i.set_enabled(state.enabled)?;
            if let Some(c) = state.checked {
                i.set_checked(c)?;
            }
        }
        _ => return Ok(false),
    }
    Ok(true)
}

/// Replace the menu bar with one a window built from its command list
#[tauri::command]
pub fn set_menu(
    app: AppHandle,
    items: tauri::State<'_, MenuItems>,
    model: Vec<MenuNode>,
) -> Result<(), String> {
    // Where the page draws the menu bar there is none (commands/window.rs)
    if crate::commands::window::page_draws_frame() {
        return Ok(());
    }
    let (menu, by_id) = build_menu(&app, &model).map_err(|e| e.to_string())?;
    app.set_menu(menu).map_err(|e| e.to_string())?;
    WINDOW_MENU.store(true, Ordering::Release);
    *items.0.lock().map_err(|e| e.to_string())? = by_id;
    Ok(())
}

/// Update items' enabled and checked state in place, for the focused window
#[tauri::command]
pub fn set_menu_state(
    items: tauri::State<'_, MenuItems>,
    states: Vec<ItemState>,
) -> Result<(), String> {
    let items = items.0.lock().map_err(|e| e.to_string())?;
    for state in &states {
        if let Some(item) = items.get(&state.id) {
            apply_state(item, state).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Print the window, for File > Print and the palette
#[tauri::command]
pub fn print_window(window: tauri::WebviewWindow) -> Result<(), String> {
    window.print().map_err(|e| e.to_string())
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

    // Building menus needs the main thread on macOS: see tests/menu_build.rs

    #[test]
    fn an_unknown_predefined_kind_is_an_error() {
        let parsed: Result<Vec<MenuNode>, _> = serde_json::from_str(
            r#"[{"kind": "predefined", "which": "fly_to_the_moon", "label": null}]"#,
        );
        assert!(parsed.is_err());
    }

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
        assert_eq!(l.settings, "Settings...");
        assert_eq!(
            l.licences_credit,
            "Open-source licences: Settings, General, Licences"
        );
    }
}
