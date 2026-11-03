use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::domain::models::{AutoEvent, ImageMeta};
use crate::state::AppState;

const CHECK_INTERVAL_SECS: u64 = 60;
const STARTUP_DELAY_SECS: u64 = 20;

pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(STARTUP_DELAY_SECS)).await;
        let mut ticker = tokio::time::interval(Duration::from_secs(CHECK_INTERVAL_SECS));
        loop {
            ticker.tick().await;
            if let Err(error) = tick(&app).await {
                log::warn!("自动任务检查失败：{error}");
            }
        }
    });
}

async fn tick(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let settings = state.settings();
    if !settings.auto_set {
        return Ok(());
    }

    let now = chrono::Local::now();
    let today = now.format("%Y-%m-%d").to_string();

    let Some((hour, minute)) = parse_time(&settings.auto_set_time) else {
        return Ok(());
    };
    let target = now
        .date_naive()
        .and_hms_opt(hour, minute, 0)
        .and_then(|naive| naive.and_local_timezone(chrono::Local).single())
        .ok_or_else(|| "无法计算自动任务时间".to_string())?;

    if now < target {
        return Ok(());
    }

    if let Ok(guard) = state.last_auto_run.lock() {
        if guard.as_deref() == Some(today.as_str()) {
            return Ok(());
        }
    }

    if let Ok(Some(last)) = state.db.last_successful_run("daily_set") {
        if last.starts_with(&today) {
            mark_run(&state, &today);
            return Ok(());
        }
    }

    if should_skip_passed_window(
        settings.catch_up,
        now.signed_duration_since(target).num_minutes(),
    ) {
        // The scheduled window has passed and catch-up is disabled.
        mark_run(&state, &today);
        return Ok(());
    }

    let _ = app.emit(
        "auto://started",
        AutoEvent {
            kind: "daily".to_string(),
            success: None,
            message: None,
        },
    );

    let outcome = run_daily(&state).await;

    match &outcome {
        Ok(image) => {
            let message = format!("已自动设置 {} 的壁纸", image.date);
            log::info!("{message}");
            let _ = app.emit(
                "auto://finished",
                AutoEvent {
                    kind: "daily".to_string(),
                    success: Some(true),
                    message: Some(message.clone()),
                },
            );
            if settings.notifications {
                notify(app, &settings.locale, &message);
            }
        }
        Err(error) => {
            let message = format!("自动设置失败：{error}");
            log::warn!("{message}");
            let _ = app.emit(
                "auto://finished",
                AutoEvent {
                    kind: "daily".to_string(),
                    success: Some(false),
                    message: Some(message.clone()),
                },
            );
            let _ = state.db.record_task(
                "daily_set",
                "failed",
                &message,
                &AppState::now_rfc3339(),
                &AppState::now_rfc3339(),
            );
            if settings.notifications {
                notify(app, &settings.locale, &message);
            }
        }
    }

    mark_run(&state, &today);
    Ok(())
}

async fn run_daily(state: &AppState) -> crate::domain::AppResult<ImageMeta> {
    let settings = state.settings();
    let started_at = AppState::now_rfc3339();
    let image = state
        .library()
        .latest(false, settings.image_size, &settings.market)
        .await?;
    let result = state
        .wallpaper()
        .apply_meta(&image, settings.wallpaper_style)?;

    state
        .app
        .emit(
            "wallpaper://changed",
            serde_json::json!({ "date": result.date, "path": result.path }),
        )
        .ok();

    let _ = state.db.record_task(
        "daily_set",
        "success",
        &result.message,
        &started_at,
        &AppState::now_rfc3339(),
    );

    Ok(image)
}

fn mark_run(state: &tauri::State<'_, AppState>, today: &str) {
    if let Ok(mut guard) = state.last_auto_run.lock() {
        *guard = Some(today.to_string());
    }
}

fn notify(app: &AppHandle, locale: &str, message: &str) {
    let title = if locale.starts_with("zh") {
        "必应每日壁纸"
    } else {
        "Bing Wallpaper"
    };
    let _ = app
        .notification()
        .builder()
        .title(title)
        .body(message)
        .show();
}

/// Decide whether a passed schedule window should be skipped instead of run
/// late. When catch-up is disabled and the target time is more than ten minutes
/// in the past, the run is considered missed and is not executed.
fn should_skip_passed_window(catch_up: bool, minutes_past_target: i64) -> bool {
    !catch_up && minutes_past_target > 10
}

fn parse_time(value: &str) -> Option<(u32, u32)> {
    let mut parts = value.trim().split(':');
    let hour = parts.next()?.parse::<u32>().ok()?;
    let minute = parts.next()?.parse::<u32>().ok()?;
    if parts.next().is_some() || hour > 23 || minute > 59 {
        return None;
    }
    Some((hour, minute))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_time_accepts_valid_values() {
        assert_eq!(parse_time("00:05"), Some((0, 5)));
        assert_eq!(parse_time("23:59"), Some((23, 59)));
        assert_eq!(parse_time(" 9:5 "), Some((9, 5)));
    }

    #[test]
    fn parse_time_rejects_invalid_values() {
        assert_eq!(parse_time("24:00"), None);
        assert_eq!(parse_time("12:60"), None);
        assert_eq!(parse_time("12:30:45"), None);
        assert_eq!(parse_time("abc"), None);
        assert_eq!(parse_time(""), None);
        assert_eq!(parse_time("12"), None);
    }

    #[test]
    fn skip_passed_window_only_without_catch_up() {
        // Catch-up enabled: never skipped, even far past the window.
        assert!(!should_skip_passed_window(true, 0));
        assert!(!should_skip_passed_window(true, 600));

        // Catch-up disabled: the 10-minute grace window still runs.
        assert!(!should_skip_passed_window(false, 0));
        assert!(!should_skip_passed_window(false, 10));

        // Past the grace window with catch-up disabled: skipped.
        assert!(should_skip_passed_window(false, 11));
        assert!(should_skip_passed_window(false, 600));
    }
}
