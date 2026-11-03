pub mod commands;
pub mod domain;
pub mod infra;
pub mod services;
pub mod state;
pub mod tray;

use tauri::Manager;

pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            commands::app::reveal_main_window(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .setup(|app| {
            let handle = app.handle().clone();
            let state = state::AppState::new(&handle)?;
            app.manage(state);
            tray::create(&handle)?;
            services::scheduler::spawn(handle.clone());
            // Index whatever already sits in the gallery directory before the
            // window asks for it; a short delay keeps first paint fast.
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                handle.state::<state::AppState>().spawn_library_scan();
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::get_bootstrap_state,
            commands::app::get_platform_capabilities,
            commands::app::open_image_folder,
            commands::app::open_external,
            commands::app::show_main_window,
            commands::app::quit_app,
            commands::app::clear_cache,
            commands::wallpaper::get_today_image,
            commands::wallpaper::set_wallpaper,
            commands::wallpaper::set_wallpaper_from_path,
            commands::gallery::list_images,
            commands::gallery::toggle_favorite,
            commands::gallery::delete_image,
            commands::gallery::rescan_library,
            commands::download::download_image,
            commands::download::download_range,
            commands::download::cancel_download,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::update::check_update,
            commands::update::install_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
