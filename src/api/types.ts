export type ImageSize = 'uhd' | '1920x1200' | '1920x1080' | '1366x768'
export type WallpaperStyle = 'fill' | 'fit' | 'stretch' | 'tile' | 'center' | 'span'
export type ThemeMode = 'system' | 'light' | 'dark'
export type UpdateChannel = 'stable' | 'beta'
export type ImageFilter = 'all' | 'favorites' | 'recent'

export interface ImageMeta {
  id: number
  date: string
  title: string
  copyright: string
  copyrightLink: string
  filePath: string
  thumbPath: string
  width: number
  height: number
  bytes: number
  favorite: boolean
  createdAt: string
}

export interface Settings {
  imageSize: ImageSize
  wallpaperStyle: WallpaperStyle
  imageDir: string | null
  locale: string
  market: string
  theme: ThemeMode
  closeToTray: boolean
  autoStart: boolean
  autoSet: boolean
  autoSetTime: string
  catchUp: boolean
  notifications: boolean
  autoCheckUpdate: boolean
  updateChannel: UpdateChannel
  retentionDays: number
  proxy: string | null
}

export type SettingsPatch = Partial<Settings>

export interface Page<T> {
  items: T[]
  total: number
  page: number
  pageSize: number
}

export interface StorageStats {
  count: number
  favoriteCount: number
  bytes: number
}

export interface PlatformCapabilities {
  os: string
  arch: string
  styles: WallpaperStyle[]
  multiMonitor: boolean
  desktopEnvironment: string | null
}

export interface BootstrapState {
  settings: Settings
  capabilities: PlatformCapabilities
  today: ImageMeta | null
  version: string
  imageDir: string
  storage: StorageStats
  isFirstRun: boolean
}

export interface SetWallpaperResult {
  success: boolean
  message: string
  path: string
  date: string
}

export interface DownloadProgress {
  taskId: string
  current: number
  total: number
  percent: number
  message: string
}

export interface DownloadFinished {
  taskId: string
  success: number
  failed: number
  skipped: number
  message: string
}

export interface UpdateInfo {
  version: string
  currentVersion: string
  notes: string
  url: string
  publishedAt: string
  mandatory: boolean
}

export interface ClearCacheResult {
  removed: number
  bytesFreed: number
}

export interface AppError {
  kind: string
  message: string
}

export interface Api {
  bootstrap(): Promise<BootstrapState>
  today(force?: boolean): Promise<ImageMeta>
  listImages(
    page: number,
    pageSize: number,
    filter: ImageFilter,
    query?: string,
  ): Promise<Page<ImageMeta>>
  toggleFavorite(id: number, favorite: boolean): Promise<ImageMeta>
  deleteImage(id: number): Promise<void>
  downloadImage(date: string, force?: boolean): Promise<ImageMeta>
  downloadRange(start: string, end: string): Promise<string>
  cancelDownload(taskId: string): Promise<boolean>
  setWallpaper(date?: string, style?: WallpaperStyle): Promise<SetWallpaperResult>
  setWallpaperFromPath(path: string, style: WallpaperStyle): Promise<void>
  getSettings(): Promise<Settings>
  updateSettings(patch: SettingsPatch): Promise<Settings>
  getCapabilities(): Promise<PlatformCapabilities>
  openImageFolder(): Promise<void>
  openExternal(url: string): Promise<void>
  checkUpdate(): Promise<UpdateInfo | null>
  installUpdate(): Promise<void>
  clearCache(): Promise<ClearCacheResult>
  showMainWindow(): Promise<void>
  quitApp(): Promise<void>
}
