import { invoke } from '@tauri-apps/api/core'
import type {
  Api,
  BootstrapState,
  ClearCacheResult,
  ImageFilter,
  ImageMeta,
  Page,
  PlatformCapabilities,
  Settings,
  SettingsPatch,
  SetWallpaperResult,
  UpdateInfo,
  WallpaperStyle,
} from './types'

/** Thin wrapper around the Rust command surface. */
export const realApi: Api = {
  bootstrap: () => invoke<BootstrapState>('get_bootstrap_state'),
  today: (force = false) => invoke<ImageMeta>('get_today_image', { force }),
  listImages: (page, pageSize, filter, query) =>
    invoke<Page<ImageMeta>>('list_images', { page, pageSize, filter, query }),
  toggleFavorite: (id, favorite) =>
    invoke<ImageMeta>('toggle_favorite', { id, favorite }),
  deleteImage: (id) => invoke<void>('delete_image', { id }),
  downloadImage: (date, force = false) =>
    invoke<ImageMeta>('download_image', { date, force }),
  downloadRange: (start, end) =>
    invoke<string>('download_range', { start, end }),
  cancelDownload: (taskId) => invoke<boolean>('cancel_download', { taskId }),
  setWallpaper: (date, style) =>
    invoke<SetWallpaperResult>('set_wallpaper', { date, style }),
  setWallpaperFromPath: (path, style) =>
    invoke<void>('set_wallpaper_from_path', { path, style }),
  getSettings: () => invoke<Settings>('get_settings'),
  updateSettings: (patch: SettingsPatch) =>
    invoke<Settings>('update_settings', { patch }),
  getCapabilities: () =>
    invoke<PlatformCapabilities>('get_platform_capabilities'),
  openImageFolder: () => invoke<void>('open_image_folder'),
  openExternal: (url) => invoke<void>('open_external', { url }),
  checkUpdate: () => invoke<UpdateInfo | null>('check_update'),
  installUpdate: () => invoke<void>('install_update'),
  clearCache: () => invoke<ClearCacheResult>('clear_cache'),
  showMainWindow: () => invoke<void>('show_main_window'),
  quitApp: () => invoke<void>('quit_app'),
}

export type { Api, ImageFilter, WallpaperStyle }
