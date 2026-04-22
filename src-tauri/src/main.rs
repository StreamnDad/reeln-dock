#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod dock_log;
mod menu;
mod models;
mod orchestration;
mod state;
#[cfg(test)]
mod test_utils;
mod update_check;

use std::sync::{Arc, Mutex};

use reeln_media::LibavBackend;
use reeln_sport::SportRegistry;
use tauri::{Emitter, Manager};

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

            // Clean up old proxy cache files (>7 days)
            let proxy_dir = app_data_dir.join("proxies");
            orchestration::render_ops::cleanup_proxy_cache(
                &proxy_dir,
                std::time::Duration::from_secs(7 * 24 * 60 * 60),
            );

            app.manage(AppState {
                config: Mutex::new(None),
                sport_registry: Mutex::new(SportRegistry::default()),
                dock_settings: Mutex::new(dock_settings),
                app_data_dir,
                media_backend: Arc::new(LibavBackend::new()),
                auth_child_pid: Arc::new(Mutex::new(None)),
            });

            // Build and set custom menu bar
            let app_handle = app.handle().clone();
            let app_menu = menu::build(&app_handle)?;
            app.set_menu(app_menu)?;
            app.on_menu_event(move |app_ref, event| {
                let id = event.id().0.as_str();
                if id == "check_updates" {
                    let ah = app_ref.clone();
                    std::thread::spawn(move || {
                        let dock_ver = env!("CARGO_PKG_VERSION");
                        let (cli_ver, plugins) = update_check::detect_cli_and_plugins(&ah);
                        let result = update_check::check(dock_ver, cli_ver.as_deref(), &plugins);
                        if result.updates.is_empty() {
                            let _ = ah.emit("update:none", ());
                        } else {
                            let _ = ah.emit("update:available", &result);
                        }
                    });
                } else {
                    menu::handle_event(&app_handle, &event);
                }
            });

            // Background update check (once per day)
            update_check::spawn_startup_check(app.handle());

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
            commands::games::update_game_info,
            commands::games::set_game_livestream,
            commands::games::remove_game_livestream,
            commands::games::discover_game_image,
            // Games (execution)
            commands::games::init_game,
            commands::games::process_segment,
            commands::games::merge_highlights,
            commands::games::finish_game,
            commands::games::prune_renders,
            commands::games::prune_game_preview,
            commands::games::prune_game_execute,
            commands::games::delete_game_preview,
            commands::games::delete_game,
            // Sports
            commands::sports::list_sports,
            // Media
            commands::media::probe_clip,
            commands::media::get_platform,
            commands::media::open_in_finder,
            commands::media::open_file,
            commands::media::file_exists,
            commands::media::prepare_preview_proxy,
            commands::media::get_proxy_cache_stats,
            commands::media::clear_proxy_cache,
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
