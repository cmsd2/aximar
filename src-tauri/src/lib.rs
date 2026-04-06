pub use aximar_core::catalog;
mod commands;
pub use aximar_core::error;
pub use aximar_core::maxima;
mod mcp;
mod menu;
pub use aximar_core::notebooks;
pub use aximar_core::session;
mod state;
pub use aximar_core::suggestions;
mod tauri_output;

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

            // Read MCP config in a single pass (persists generated defaults
            // like mcp_token so the server and frontend always agree).
            let mcp_config = commands::config::read_mcp_config(app.handle());

            if mcp_config.enabled {
                let mcp_state = AppState {
                    registry: state.registry.clone(),
                    catalog: state.catalog.clone(),
                    docs: state.docs.clone(),
                    packages: state.packages.clone(),
                    app_handle: state.app_handle.clone(),
                    mcp_controller: state.mcp_controller.clone(),
                    app_log: state.app_log.clone(),
                };
                let ct = tokio_util::sync::CancellationToken::new();
                let controller = state.mcp_controller.clone();
                tauri::async_runtime::spawn(async move {
                    controller.set_running(ct.clone()).await;
                    mcp::start_mcp_server(mcp_state, mcp_config.listen_address, mcp_config.token, ct).await;
                });
            }

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
            commands::catalog::search_packages,
            commands::catalog::complete_packages,
            commands::catalog::get_package,
            commands::catalog::package_for_function,
            commands::catalog::search_package_functions,
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
            commands::plot::write_binary_file,
            commands::plot::write_text_file,
            commands::plot::ensure_directory,
            commands::config::list_wsl_distros,
            commands::config::check_wsl_maxima,
            commands::config::get_buffered_logs,
            commands::config::claude_mcp_status,
            commands::config::claude_mcp_configure,
            commands::config::codex_mcp_status,
            commands::config::codex_mcp_configure,
            commands::config::gemini_mcp_status,
            commands::config::gemini_mcp_configure,
            commands::notebook::nb_get_state,
            commands::notebook::nb_add_cell,
            commands::notebook::nb_delete_cell,
            commands::notebook::nb_move_cell,
            commands::notebook::nb_toggle_cell_type,
            commands::notebook::nb_update_cell_input,
            commands::notebook::nb_undo,
            commands::notebook::nb_redo,
            commands::notebook::nb_new_notebook,
            commands::notebook::nb_load_cells,
            commands::notebook::nb_run_cell,
            commands::notebook::nb_approve_cell,
            commands::notebook::nb_abort_cell,
            commands::notebook::nb_create,
            commands::notebook::nb_close,
            commands::notebook::nb_list,
            commands::notebook::nb_set_active,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
