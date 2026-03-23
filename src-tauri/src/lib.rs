pub mod catalog;
mod commands;
mod error;
mod maxima;
mod menu;
mod notebooks;
mod state;
mod suggestions;

use tauri::Manager;
use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::new())
        .setup(|app| {
            menu::setup_menu(app)?;
            let state = app.state::<AppState>();
            let handle = app.handle().clone();
            tauri::async_runtime::block_on(async {
                *state.app_handle.lock().await = Some(handle);
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::evaluate::evaluate_expression,
            commands::session::start_session,
            commands::session::stop_session,
            commands::session::restart_session,
            commands::session::get_session_status,
            commands::config::get_theme,
            commands::config::set_theme,
            commands::catalog::search_functions,
            commands::catalog::complete_function,
            commands::catalog::get_function,
            commands::catalog::list_categories,
            commands::catalog::get_function_docs,
            commands::suggestions::get_suggestions,
            commands::notebooks::list_templates,
            commands::notebooks::get_template,
            commands::notebooks::save_notebook,
            commands::notebooks::open_notebook,
            commands::config::get_has_seen_welcome,
            commands::config::set_has_seen_welcome,
            commands::config::get_config,
            commands::config::set_config,
            commands::variables::list_variables,
            commands::variables::kill_variable,
            commands::variables::kill_all_variables,
            commands::plot::write_plot_svg,
            commands::config::list_wsl_distros,
            commands::config::check_wsl_maxima,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
