use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt;

use crate::domain::errors::{AppError, AppResult};
use crate::domain::models::{Settings, UpdateInfo};
use crate::infra::fs;
use crate::infra::platform;
use crate::infra::{Db, HttpClient};
use crate::services::library::LibraryService;
use crate::services::settings::{resolve_image_dir, SettingsStore};
use crate::services::wallpaper::WallpaperService;

pub struct AppState {
    pub app: AppHandle,
    pub data_dir: PathBuf,
    pub thumbs_dir: PathBuf,
    pub db: Db,
    pub http: HttpClient,
    pub settings: SettingsStore,
    pub backend: Box<dyn crate::domain::traits::WallpaperBackend>,
    pub downloads: Mutex<HashMap<String, Arc<AtomicBool>>>,
    pub last_auto_run: Mutex<Option<String>>,
    pub last_update: Mutex<Option<UpdateInfo>>,
    pub version: String,
    images_dir: RwLock<PathBuf>,
}

impl AppState {
    pub fn new(app: &AppHandle) -> AppResult<Self> {
        let data_dir = app
            .path()
            .app_data_dir()
            .map_err(|error| AppError::Io(format!("无法确定应用数据目录：{error}")))?;
        fs::ensure_dir(&data_dir)?;

        let thumbs_dir = data_dir.join("thumbnails");
        fs::ensure_dir(&thumbs_dir)?;

        let db = Db::open(&data_dir.join("library.db"))?;
        let settings = SettingsStore::load(data_dir.join("settings.json"));
        let current = settings.get();
        let images_dir = resolve_image_dir(&data_dir, &current);
        fs::ensure_dir(&images_dir)?;

        let http = HttpClient::new(current.proxy.as_deref())?;
        let backend = platform::create_backend();
        let version = app.package_info().version.to_string();

        let state = Self {
            app: app.clone(),
            data_dir: data_dir.clone(),
            thumbs_dir,
            db,
            http,
            settings,
            backend,
            downloads: Mutex::new(HashMap::new()),
            last_auto_run: Mutex::new(None),
            last_update: Mutex::new(None),
            version,
            images_dir: RwLock::new(images_dir.clone()),
        };

        state.allow_asset_dir(&data_dir);
        state.allow_asset_dir(&images_dir);

        Ok(state)
    }

    pub fn settings(&self) -> Settings {
        self.settings.get()
    }

    pub fn images_dir(&self) -> PathBuf {
        self.images_dir
            .read()
            .map(|guard| guard.clone())
            .unwrap_or_else(|_| self.data_dir.join("images"))
    }

    pub fn library(&self) -> LibraryService<'_> {
        LibraryService::new(
            &self.http,
            &self.db,
            self.images_dir(),
            self.thumbs_dir.clone(),
        )
    }

    pub fn wallpaper(&self) -> WallpaperService<'_> {
        WallpaperService::new(self.backend.as_ref(), &self.db)
    }

    pub fn register_download(&self, task_id: &str) -> Arc<AtomicBool> {
        let flag = Arc::new(AtomicBool::new(false));
        if let Ok(mut guard) = self.downloads.lock() {
            guard.insert(task_id.to_string(), flag.clone());
        }
        flag
    }

    pub fn cancel_download(&self, task_id: &str) -> bool {
        if let Ok(guard) = self.downloads.lock() {
            if let Some(flag) = guard.get(task_id) {
                flag.store(true, Ordering::SeqCst);
                return true;
            }
        }
        false
    }

    pub fn finish_download(&self, task_id: &str) {
        if let Ok(mut guard) = self.downloads.lock() {
            guard.remove(task_id);
        }
    }

    /// Apply side effects after settings changed.
    pub fn apply_settings_side_effects(
        &self,
        previous: &Settings,
        next: &Settings,
    ) -> AppResult<()> {
        if previous.proxy != next.proxy {
            self.http.rebuild(next.proxy.as_deref())?;
        }

        if previous.image_dir != next.image_dir {
            let dir = resolve_image_dir(&self.data_dir, next);
            fs::ensure_dir(&dir)?;
            if let Ok(mut guard) = self.images_dir.write() {
                *guard = dir.clone();
            }
            self.allow_asset_dir(&dir);
        }

        if previous.auto_start != next.auto_start {
            let autolaunch = self.app.autolaunch();
            let result = if next.auto_start {
                autolaunch.enable()
            } else {
                autolaunch.disable()
            };
            if let Err(error) = result {
                log::warn!("更新开机自启状态失败：{error}");
            }
        }

        Ok(())
    }

    pub fn allow_asset_dir(&self, path: &std::path::Path) {
        if let Err(error) = self.app.asset_protocol_scope().allow_directory(path, true) {
            log::warn!("无法授权资源目录 {}：{error}", path.display());
        }
    }

    pub fn now_string() -> String {
        chrono::Local::now().format("%Y-%m-%d").to_string()
    }

    pub fn now_rfc3339() -> String {
        chrono::Local::now().to_rfc3339()
    }
}
