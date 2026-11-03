use std::path::Path;
use std::sync::atomic::Ordering;
use std::time::Duration;

use chrono::NaiveDate;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::domain::errors::{AppError, AppResult};
use crate::domain::models::{DownloadFinished, DownloadProgress, ImageMeta};
use crate::services::bing::BingClient;
use crate::state::AppState;

#[tauri::command]
pub async fn download_image(
    state: State<'_, AppState>,
    date: String,
    force: bool,
) -> AppResult<ImageMeta> {
    parse_date(&date)?;
    let settings = state.settings();
    state
        .library()
        .ensure(&date, force, settings.image_size, &settings.market)
        .await
}

#[tauri::command]
pub async fn download_range(
    app: AppHandle,
    start: String,
    end: String,
) -> AppResult<String> {
    let start_date = parse_date(&start)?;
    let end_date = parse_date(&end)?;
    if start_date > end_date {
        return Err(AppError::InvalidInput("开始日期不能晚于结束日期".into()));
    }
    let span = (end_date - start_date).num_days();
    if span > 366 {
        return Err(AppError::InvalidInput("单次最多下载 366 天".into()));
    }

    let task_id = format!("task-{}", chrono::Local::now().timestamp_millis());
    let cancel = {
        let state = app.state::<AppState>();
        state.register_download(&task_id)
    };

    let spawned_task = task_id.clone();
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        run_task(handle, spawned_task, start_date, end_date, cancel).await;
    });

    Ok(task_id)
}

#[tauri::command]
pub async fn cancel_download(
    state: State<'_, AppState>,
    task_id: String,
) -> AppResult<bool> {
    Ok(state.cancel_download(&task_id))
}

async fn run_task(
    app: AppHandle,
    task_id: String,
    start: NaiveDate,
    end: NaiveDate,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    let state = app.state::<AppState>();
    let settings = state.settings();
    let library = state.library();
    let started_at = AppState::now_rfc3339();

    let dates = date_range(start, end);
    let total = dates.len() as u32;

    let list = match BingClient::new(&state.http).fetch(&settings.market).await {
        Ok(list) => list,
        Err(error) => {
            state.finish_download(&task_id);
            let finished = DownloadFinished {
                task_id: task_id.clone(),
                success: 0,
                failed: total,
                skipped: 0,
                message: format!("无法获取 Bing 图片列表：{error}"),
            };
            let _ = app.emit("download://finished", &finished);
            let _ = state.db.record_task(
                "download",
                "failed",
                &finished.message,
                &started_at,
                &AppState::now_rfc3339(),
            );
            return;
        }
    };

    let mut success = 0u32;
    let mut failed = 0u32;
    let mut skipped = 0u32;
    let mut message = "任务完成".to_string();

    for (index, date) in dates.iter().enumerate() {
        if cancel.load(Ordering::SeqCst) {
            message = "任务已取消".to_string();
            break;
        }

        let current = index as u32 + 1;

        let cached = state
            .db
            .get_by_date(date)
            .ok()
            .flatten()
            .filter(|meta| Path::new(&meta.file_path).is_file());

        if cached.is_some() {
            skipped += 1;
            emit_progress(
                &app,
                &task_id,
                current,
                total,
                format!("{date} 已存在，跳过"),
            );
            continue;
        }

        match list.iter().find(|item| item.date == *date) {
            Some(bing) => match library.download_one(bing, settings.image_size).await {
                Ok(_) => {
                    success += 1;
                    emit_progress(&app, &task_id, current, total, format!("已下载 {date}"));
                }
                Err(error) => {
                    failed += 1;
                    log::warn!("下载 {date} 失败：{error}");
                    emit_progress(
                        &app,
                        &task_id,
                        current,
                        total,
                        format!("下载 {date} 失败：{error}"),
                    );
                }
            },
            None => {
                failed += 1;
                emit_progress(
                    &app,
                    &task_id,
                    current,
                    total,
                    format!("{date} 超出 Bing 最近 8 天范围"),
                );
            }
        }

        tokio::time::sleep(Duration::from_millis(150)).await;
    }

    state.finish_download(&task_id);

    let finished = DownloadFinished {
        task_id: task_id.clone(),
        success,
        failed,
        skipped,
        message,
    };
    let _ = app.emit("download://finished", &finished);

    let status = if finished.failed > 0 { "failed" } else { "success" };
    let _ = state.db.record_task(
        "download",
        status,
        &finished.message,
        &started_at,
        &AppState::now_rfc3339(),
    );
    let _ = app.emit("library://changed", ());
}

fn emit_progress(
    app: &AppHandle,
    task_id: &str,
    current: u32,
    total: u32,
    message: String,
) {
    let percent = if total == 0 {
        0
    } else {
        ((current as f64 / total as f64) * 100.0).round() as u32
    };
    let payload = DownloadProgress {
        task_id: task_id.to_string(),
        current,
        total,
        percent: percent.min(100),
        message,
    };
    let _ = app.emit("download://progress", payload);
}

fn date_range(start: NaiveDate, end: NaiveDate) -> Vec<String> {
    let mut dates = Vec::new();
    let mut cursor = start;
    while cursor <= end {
        dates.push(cursor.format("%Y-%m-%d").to_string());
        cursor += chrono::Duration::days(1);
    }
    dates
}

fn parse_date(value: &str) -> AppResult<NaiveDate> {
    NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d")
        .map_err(|error| AppError::InvalidInput(format!("日期格式无效：{value}（{error}）")))
}
