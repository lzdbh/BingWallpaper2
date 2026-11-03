use serde_json::json;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

use crate::state::AppState;

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
    let set_now = MenuItem::with_id(app, "set_now", "立即设置壁纸", true, None::<&str>)?;
    let gallery = MenuItem::with_id(app, "gallery", "打开图库", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[&show, &set_now, &gallery, &settings, &separator, &quit],
    )?;

    let mut builder = TrayIconBuilder::with_id("main-tray")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("必应每日壁纸")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => crate::commands::app::reveal_main_window(app),
            "set_now" => spawn_set_now(app.clone()),
            "gallery" => {
                crate::commands::app::reveal_main_window(app);
                let _ = app.emit("app://navigate", "/gallery");
            }
            "settings" => {
                crate::commands::app::reveal_main_window(app);
                let _ = app.emit("app://navigate", "/settings");
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                crate::commands::app::reveal_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }

    builder.build(app)?;
    Ok(())
}

fn spawn_set_now(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        let settings = state.settings();
        let image = match state
            .library()
            .latest(false, settings.image_size, &settings.market)
            .await
        {
            Ok(image) => image,
            Err(error) => {
                log::warn!("托盘设置壁纸失败：{error}");
                return;
            }
        };

        match state
            .wallpaper()
            .apply_meta(&image, settings.wallpaper_style)
        {
            Ok(result) => {
                let _ = app.emit(
                    "wallpaper://changed",
                    json!({ "date": result.date, "path": result.path }),
                );
            }
            Err(error) => log::warn!("托盘设置壁纸失败：{error}"),
        }
    });
}
