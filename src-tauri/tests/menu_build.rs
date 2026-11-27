//! Builds menu bars from window models against Tauri's mock runtime.  macOS
//! lets only the main thread make menus, so this runs as a plain program
//! (`harness = false`): each check is a function, and a failure panics.

use aximar_lib::menu::{apply_state, build_menu, ItemState, MenuNode};
use tauri::menu::MenuItemKind;
use tauri::test::MockRuntime;

/// One check, run against the mock app
type Check = fn(&tauri::App<MockRuntime>);

fn model(json: &str) -> Vec<MenuNode> {
    serde_json::from_str(json).expect("model parses")
}

fn builds_every_predefined_kind(app: &tauri::App<MockRuntime>) {
    let kinds = [
        "separator",
        "cut",
        "copy",
        "paste",
        "select_all",
        "services",
        "hide",
        "hide_others",
        "show_all",
        "quit",
        "about",
        "minimize",
        "maximize",
    ];
    let items: Vec<String> = kinds
        .iter()
        .map(|k| format!(r#"{{"kind": "predefined", "which": "{k}", "label": null}}"#))
        .collect();
    let json = format!(
        r#"[{{"kind": "submenu", "label": "All", "items": [{}]}}]"#,
        items.join(",")
    );
    let (menu, by_id) = build_menu(app, &model(&json)).unwrap();
    assert_eq!(menu.items().unwrap().len(), 1);
    assert!(by_id.is_empty(), "predefined items have no command id");
}

fn builds_a_check_item_checked(app: &tauri::App<MockRuntime>) {
    let json = r#"[{"kind": "submenu", "label": "View", "items": [
        {"kind": "item", "id": "view.docs", "label": "Docs", "accelerator": null, "enabled": true, "checked": true}
    ]}]"#;
    let (_, by_id) = build_menu(app, &model(json)).unwrap();
    match &by_id["view.docs"] {
        MenuItemKind::Check(item) => assert!(item.is_checked().unwrap()),
        _ => panic!("view.docs should be a check item"),
    }
}

fn builds_a_disabled_item(app: &tauri::App<MockRuntime>) {
    let json = r#"[{"kind": "submenu", "label": "Session", "items": [
        {"kind": "item", "id": "session.interrupt", "label": "Interrupt", "accelerator": "CmdOrCtrl+Period", "enabled": false, "checked": null}
    ]}]"#;
    let (_, by_id) = build_menu(app, &model(json)).unwrap();
    match &by_id["session.interrupt"] {
        MenuItemKind::MenuItem(item) => assert!(!item.is_enabled().unwrap()),
        _ => panic!("session.interrupt should be a plain item"),
    }
}

fn state_updates_find_items_by_id(app: &tauri::App<MockRuntime>) {
    let json = r#"[{"kind": "submenu", "label": "View", "items": [
        {"kind": "submenu", "label": "Nested", "items": [
            {"kind": "item", "id": "view.log", "label": "Log", "accelerator": null, "enabled": true, "checked": false}
        ]}
    ]}]"#;
    let (_, by_id) = build_menu(app, &model(json)).unwrap();
    let state = ItemState {
        id: "view.log".into(),
        enabled: false,
        checked: Some(true),
    };
    assert!(apply_state(&by_id["view.log"], &state).unwrap());
    match &by_id["view.log"] {
        MenuItemKind::Check(item) => {
            assert!(item.is_checked().unwrap());
            assert!(!item.is_enabled().unwrap());
        }
        _ => panic!("view.log should be a check item"),
    }
}

fn main() {
    let app = tauri::test::mock_app();
    let checks: [(&str, Check); 4] = [
        ("builds_every_predefined_kind", builds_every_predefined_kind),
        ("builds_a_check_item_checked", builds_a_check_item_checked),
        ("builds_a_disabled_item", builds_a_disabled_item),
        (
            "state_updates_find_items_by_id",
            state_updates_find_items_by_id,
        ),
    ];
    for (name, check) in checks {
        check(&app);
        println!("test {name} ... ok");
    }
}
