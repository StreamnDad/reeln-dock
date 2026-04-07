#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod models;
mod orchestration;
mod state;
#[cfg(test)]
mod test_utils;

use std::sync::{Arc, Mutex};

use reeln_media::LibavBackend;
use reeln_sport::SportRegistry;
use tauri::Manager;

use state::{AppState, DockSettings};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data dir");

            let dock_settings = DockSettings::load(&app_data_dir).unwrap_or_default();

            app.manage(AppState {
                config: Mutex::new(None),
                sport_registry: Mutex::new(SportRegistry::default()),
                dock_settings: Mutex::new(dock_settings),
                app_data_dir,
                media_backend: Arc::new(LibavBackend::new()),
                auth_child_pid: Arc::new(Mutex::new(None)),
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Config
            commands::config::load_dock_settings,
            commands::config::save_dock_settings,
            commands::config::load_config_from_path,
            commands::config::get_config_path,
            commands::config::save_event_types,
            commands::config::save_render_profile,
            commands::config::delete_render_profile,
            commands::config::rename_render_profile,
            commands::config::save_render_stage,
            commands::config::load_render_stage,
            // Games (read)
            commands::games::list_games,
            commands::games::get_game_state,
            commands::games::set_game_tournament,
            commands::games::update_game_event,
            commands::games::bulk_update_event_type,
            commands::games::get_event_types,
            commands::games::quick_tag_event,
            // Games (execution)
            commands::games::init_game,
            commands::games::process_segment,
            commands::games::merge_highlights,
            commands::games::finish_game,
            commands::games::prune_renders,
            // Sports
            commands::sports::list_sports,
            // Media
            commands::media::probe_clip,
            commands::media::open_in_finder,
            commands::media::open_file,
            commands::media::file_exists,
            // Render
            commands::render::render_short,
            commands::render::render_iteration,
            commands::render::render_preview,
            commands::render::delete_preview,
            commands::render::render_reel,
            commands::render::render_profile_preview,
            commands::render::suggest_preview_clip,
            commands::render::list_render_profiles,
            commands::render::get_iteration_profiles,
            // Teams
            commands::teams::list_team_levels,
            commands::teams::list_team_profiles,
            commands::teams::save_team_profile,
            commands::teams::delete_team_profile,
            commands::teams::clone_team_profile,
            commands::teams::rename_team_level,
            commands::teams::delete_team_level,
            commands::teams::load_roster,
            commands::teams::load_team_roster,
            // Prompts
            commands::prompts::list_prompt_templates,
            commands::prompts::get_prompt_template,
            commands::prompts::preview_prompt,
            commands::prompts::save_prompt_template,
            // Plugins
            commands::plugins::list_config_profiles,
            commands::plugins::list_plugins_for_profile,
            commands::plugins::toggle_plugin_in_config,
            commands::plugins::update_plugin_in_config,
            commands::plugins::fetch_plugin_registry,
            commands::plugins::add_plugin_to_config,
            commands::plugins::remove_plugin_from_config,
            commands::plugins::create_config_profile,
            commands::plugins::get_version_info,
            commands::plugins::get_enforce_hooks,
            commands::plugins::set_enforce_hooks,
            // Queue
            commands::queue::queue_list,
            commands::queue::queue_list_all,
            commands::queue::queue_show,
            commands::queue::queue_targets,
            commands::queue::queue_edit,
            commands::queue::queue_publish,
            commands::queue::queue_publish_all,
            commands::queue::queue_remove,
            // Hooks (plugin execution via CLI)
            commands::hooks::detect_reeln_cli,
            commands::hooks::get_cli_version,
            commands::hooks::install_plugin_via_cli,
            commands::hooks::execute_plugin_hook,
            commands::hooks::check_plugin_auth,
            commands::hooks::refresh_plugin_auth,
            commands::hooks::cancel_plugin_auth,
            // Tournaments
            commands::tournaments::list_tournament_metadata,
            commands::tournaments::set_tournament_archived,
            commands::tournaments::update_tournament_metadata,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
