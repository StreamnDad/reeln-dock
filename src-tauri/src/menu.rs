use tauri::image::Image;
use tauri::menu::{AboutMetadataBuilder, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, Wry};

/// Build the application menu bar with custom Help and View menus.
pub fn build(app: &AppHandle<Wry>) -> tauri::Result<Menu<Wry>> {
    let menu = Menu::new(app)?;

    // ── App menu (macOS) ─────────────────────────────────────
    let icon = Image::from_bytes(include_bytes!("../icons/128x128.png"))
        .map(|i| i.to_owned())
        .ok();
    let about = PredefinedMenuItem::about(
        app,
        Some("About reeln dock"),
        Some(
            AboutMetadataBuilder::new()
                .name(Some("reeln dock"))
                .version(Some(env!("CARGO_PKG_VERSION")))
                .authors(Some(vec!["Streamn Dad".into()]))
                .comments(Some(
                    "Desktop companion for reeln — visual render profiles, \
                     clip review, and game management for youth sports livestreamers.",
                ))
                .copyright(Some("© 2026 Streamn Dad"))
                .license(Some("AGPL-3.0-only"))
                .website(Some("https://streamn.dad"))
                .website_label(Some("streamn.dad"))
                .credits(Some(
                    "Built with Tauri, Svelte, and reeln-core.\n\
                     Powered by FFmpeg and the reeln plugin ecosystem.\n\n\
                     https://streamn.dad\n\
                     https://github.com/StreamnDad/reeln-dock",
                ))
                .icon(icon)
                .build(),
        ),
    )?;
    let app_menu = Submenu::with_items(
        app,
        "reeln dock",
        true,
        &[
            &about,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::hide(app, None)?,
            &PredefinedMenuItem::hide_others(app, None)?,
            &PredefinedMenuItem::show_all(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::quit(app, None)?,
        ],
    )?;

    // ── File menu ────────────────────────────────────────────
    let settings_item = MenuItem::with_id(
        app,
        "nav_settings",
        "Settings...",
        true,
        Some("CmdOrCtrl+,"),
    )?;
    let file_menu = Submenu::with_items(
        app,
        "File",
        true,
        &[
            &settings_item,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::close_window(app, None)?,
        ],
    )?;

    // ── Edit menu ────────────────────────────────────────────
    let edit_menu = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &PredefinedMenuItem::undo(app, None)?,
            &PredefinedMenuItem::redo(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
        ],
    )?;

    // ── View menu ────────────────────────────────────────────
    let nav_games = MenuItem::with_id(app, "nav_games", "Games", true, Some("CmdOrCtrl+1"))?;
    let nav_queue = MenuItem::with_id(app, "nav_queue", "Queue", true, Some("CmdOrCtrl+2"))?;
    let nav_plugins = MenuItem::with_id(app, "nav_plugins", "Plugins", true, Some("CmdOrCtrl+3"))?;
    let nav_registry = MenuItem::with_id(
        app,
        "nav_registry",
        "Plugin Registry",
        true,
        Some("CmdOrCtrl+4"),
    )?;
    let view_menu = Submenu::with_items(
        app,
        "View",
        true,
        &[
            &nav_games,
            &nav_queue,
            &nav_plugins,
            &nav_registry,
            &PredefinedMenuItem::separator(app)?,
            &settings_item,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::fullscreen(app, None)?,
        ],
    )?;

    // ── Window menu ──────────────────────────────────────────
    let window_menu = Submenu::with_items(
        app,
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(app, None)?,
            &PredefinedMenuItem::maximize(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::close_window(app, None)?,
        ],
    )?;

    // ── Help menu ────────────────────────────────────────────
    let help_docs = MenuItem::with_id(app, "help_docs", "Documentation", true, None::<&str>)?;
    let help_quickstart = MenuItem::with_id(
        app,
        "help_quickstart",
        "Quick Start Guide",
        true,
        None::<&str>,
    )?;
    let help_cli = MenuItem::with_id(app, "help_cli", "CLI Reference", true, None::<&str>)?;
    let help_examples = MenuItem::with_id(app, "help_examples", "Examples", true, None::<&str>)?;
    let help_issue =
        MenuItem::with_id(app, "help_issue", "Report an Issue...", true, None::<&str>)?;
    let help_discuss = MenuItem::with_id(
        app,
        "help_discuss",
        "Join Discussion...",
        true,
        None::<&str>,
    )?;
    let help_website = MenuItem::with_id(app, "help_website", "streamn.dad", true, None::<&str>)?;
    let help_menu = Submenu::with_items(
        app,
        "Help",
        true,
        &[
            &help_docs,
            &help_quickstart,
            &help_cli,
            &help_examples,
            &PredefinedMenuItem::separator(app)?,
            &help_issue,
            &help_discuss,
            &PredefinedMenuItem::separator(app)?,
            &help_website,
        ],
    )?;

    menu.append_items(&[
        &app_menu,
        &file_menu,
        &edit_menu,
        &view_menu,
        &window_menu,
        &help_menu,
    ])?;

    Ok(menu)
}

/// URL table for help menu items that open external links.
fn menu_url(id: &str) -> Option<&'static str> {
    match id {
        "help_docs" => Some("https://reeln-cli.readthedocs.io/en/latest/index.html"),
        "help_quickstart" => Some("https://reeln-cli.readthedocs.io/en/latest/quickstart.html"),
        "help_cli" => Some("https://reeln-cli.readthedocs.io/en/latest/cli/index.html"),
        "help_examples" => Some("https://reeln-cli.readthedocs.io/en/latest/examples/index.html"),
        "help_issue" => Some("https://github.com/StreamnDad/reeln-dock/issues"),
        "help_discuss" => Some("https://github.com/StreamnDad/reeln-dock/discussions"),
        "help_website" => Some("https://streamn.dad"),
        _ => None,
    }
}

/// Handle menu events — open URLs or emit navigation events to the frontend.
pub fn handle_event(app: &AppHandle<Wry>, event: &tauri::menu::MenuEvent) {
    let id = event.id().0.as_str();

    // External links
    if let Some(url) = menu_url(id) {
        let _ = open::that(url);
        return;
    }

    // In-app navigation — emit to frontend
    match id {
        "nav_games" | "nav_queue" | "nav_plugins" | "nav_registry" | "nav_settings" => {
            let _ = app.emit("menu:navigate", id);
        }
        _ => {}
    }
}
