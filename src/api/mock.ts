import dayjs from 'dayjs'
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
import { mockBus } from './events'

const SETTINGS_KEY = 'bingwallpaper.mock.settings'
const FAVORITES_KEY = 'bingwallpaper.mock.favorites'
const DELETED_KEY = 'bingwallpaper.mock.deleted'

const TITLES = [
  ['冰川与河流的交响', '克尼克冰川，阿拉斯加'],
  ['雪落无声', '北海道森林，日本'],
  ['秋日层林', '喀纳斯，新疆'],
  ['云海之上', '黄山，安徽'],
  ['极光之夜', '罗弗敦群岛，挪威'],
  ['沙丘的曲线', '撒哈拉沙漠，摩洛哥'],
  ['海岸线的清晨', '十二使徒岩，澳大利亚'],
  ['湖心的倒影', '班夫国家公园，加拿大'],
  ['花田与远山', '普罗旺斯，法国'],
  ['雾中森林', '黑森林，德国'],
  ['星空下的帐篷', '巴塔哥尼亚，智利'],
  ['梯田的纹理', '元阳，云南'],
]

const PALETTES = [
  ['#0b1d3a', '#1f4e79', '#ffd166', '#12314f', '#0a2138'],
  ['#1b1b2f', '#3a506b', '#f4a261', '#22344a', '#151d2e'],
  ['#2d1b0e', '#7c4a1e', '#f2c14e', '#4a2c12', '#231407'],
  ['#0f2027', '#2c5364', '#9be7ff', '#16323a', '#0b1a20'],
  ['#141e30', '#243b55', '#7ef9c4', '#1b2f45', '#0d1626'],
  ['#3e1f0d', '#b5651d', '#ffd6a5', '#6b3a16', '#2a1308'],
  ['#001f3f', '#0074d9', '#7fdbff', '#0a2f52', '#001426'],
  ['#1a2a6c', '#2a4d8f', '#a8d8ea', '#16305c', '#0d1b3a'],
  ['#2b1055', '#7597de', '#ffc8dd', '#3a1b6e', '#1c0b38'],
  ['#0b3d2e', '#1e6f50', '#d8f3dc', '#14513c', '#082b20'],
  ['#221c35', '#4b3f72', '#ffd166', '#2f2749', '#161126'],
  ['#3d2b1f', '#8a5a2b', '#f4d35e', '#5c3d22', '#26180f'],
]

const MOCK_IMAGE_DIR = 'C:\\Users\\Demo\\Pictures\\BingWallpaper'

const DEFAULT_SETTINGS: Settings = {
  imageSize: '1920x1080',
  wallpaperStyle: 'fill',
  imageDir: null,
  locale: 'zh-CN',
  market: 'zh-CN',
  theme: 'system',
  closeToTray: true,
  autoStart: false,
  autoSet: true,
  autoSetTime: '00:05',
  catchUp: true,
  notifications: true,
  autoCheckUpdate: true,
  updateChannel: 'stable',
  retentionDays: 30,
  proxy: null,
}

function readJson<T>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key)
    return raw ? (JSON.parse(raw) as T) : fallback
  } catch {
    return fallback
  }
}

function writeJson(key: string, value: unknown) {
  try {
    localStorage.setItem(key, JSON.stringify(value))
  } catch {
    /* storage may be unavailable */
  }
}

function delay(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

function makeSvg(seed: number, width: number, height: number): string {
  const [skyA, skyB, glow, ridgeA, ridgeB] = PALETTES[seed % PALETTES.length]
  const [title] = TITLES[seed % TITLES.length]
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 1920 1080">
    <defs>
      <linearGradient id="sky" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" stop-color="${skyA}"/><stop offset="1" stop-color="${skyB}"/>
      </linearGradient>
      <radialGradient id="glow" cx="0.72" cy="0.24" r="0.34">
        <stop offset="0" stop-color="${glow}" stop-opacity="0.9"/>
        <stop offset="1" stop-color="${glow}" stop-opacity="0"/>
      </radialGradient>
    </defs>
    <rect width="1920" height="1080" fill="url(#sky)"/>
    <circle cx="1382" cy="258" r="340" fill="url(#glow)"/>
    <path d="M0 772 L262 526 L520 726 L778 470 L1096 764 L1320 606 L1618 802 L1920 642 L1920 1080 L0 1080 Z" fill="${ridgeA}" opacity="0.82"/>
    <path d="M0 886 L352 704 L642 864 L936 646 L1258 884 L1560 724 L1920 902 L1920 1080 L0 1080 Z" fill="${ridgeB}" opacity="0.94"/>
    <text x="96" y="990" font-family="Segoe UI, Helvetica, sans-serif" font-size="54" fill="#ffffff" fill-opacity="0.22">${title}</text>
  </svg>`
  return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`
}

function mockImages(): ImageMeta[] {
  const favorites = new Set(readJson<number[]>(FAVORITES_KEY, []))
  const deleted = new Set(readJson<string[]>(DELETED_KEY, []))
  const today = dayjs()
  const images: ImageMeta[] = []

  for (let i = 0; i < 40; i += 1) {
    const date = today.subtract(i, 'day').format('YYYY-MM-DD')
    if (deleted.has(date)) continue
    const seed = i + 3
    const [title, copyright] = TITLES[seed % TITLES.length]
    const id = 5000 - i
    images.push({
      id,
      date,
      title,
      copyright: `${copyright} (© Bing)`,
      copyrightLink: 'https://www.bing.com',
      filePath: makeSvg(seed, 1920, 1080),
      thumbPath: makeSvg(seed, 960, 540),
      width: 1920,
      height: 1080,
      bytes: 620_000 + seed * 41_337,
      favorite: favorites.has(id),
      createdAt: `${date}T08:00:00`,
    })
  }
  return images
}

let images = mockImages()
const activeTasks = new Set<string>()

function stats(settings: Settings) {
  return {
    count: images.length,
    favoriteCount: images.filter((i) => i.favorite).length,
    bytes: images.reduce((sum, i) => sum + i.bytes, 0),
  }
}

function loadSettings(): Settings {
  return { ...DEFAULT_SETTINGS, ...readJson<Partial<Settings>>(SETTINGS_KEY, {}) }
}

function capabilities(): PlatformCapabilities {
  const platform = navigator.userAgent.toLowerCase()
  const os = platform.includes('mac')
    ? 'macos'
    : platform.includes('linux')
      ? 'linux'
      : 'windows'
  return {
    os,
    arch: 'x86_64',
    styles: ['fill', 'fit', 'stretch', 'tile', 'center', 'span'],
    multiMonitor: false,
    desktopEnvironment: null,
  }
}

function findImage(date?: string): ImageMeta {
  if (!date) return images[0]
  const found = images.find((image) => image.date === date)
  if (!found) throw { kind: 'InvalidInput', message: `没有找到 ${date} 的壁纸` }
  return found
}

async function runDownload(start: string, end: string): Promise<string> {
  const taskId = `mock-${Date.now()}`
  const startDate = dayjs(start)
  const endDate = dayjs(end)
  if (!startDate.isValid() || !endDate.isValid() || startDate.isAfter(endDate, 'day')) {
    throw { kind: 'InvalidInput', message: '日期范围无效' }
  }

  const dates: string[] = []
  for (let d = startDate; !d.isAfter(endDate, 'day'); d = d.add(1, 'day')) {
    dates.push(d.format('YYYY-MM-DD'))
  }

  activeTasks.add(taskId)
  void (async () => {
    let current = 0
    let success = 0
    let failed = 0
    let skipped = 0
    let cancelled = false
    for (const date of dates) {
      if (!activeTasks.has(taskId)) {
        cancelled = true
        break
      }
      await delay(220)
      current += 1
      const exists = images.some((image) => image.date === date)
      if (exists) skipped += 1
      else success += 1
      mockBus.emit('download://progress', {
        taskId,
        current,
        total: dates.length,
        percent: Math.round((current / dates.length) * 100),
        message: exists ? `${date} 已存在，跳过` : `已下载 ${date}`,
      })
    }
    activeTasks.delete(taskId)
    mockBus.emit('download://finished', {
      taskId,
      success,
      failed,
      skipped,
      message: cancelled ? '任务已取消' : '任务完成',
    })
  })()

  return taskId
}

/** A fully functional in-browser backend used by `npm run dev` outside Tauri. */
export const mockApi: Api = {
  async bootstrap(): Promise<BootstrapState> {
    await delay(180)
    const settings = loadSettings()
    return {
      settings,
      capabilities: capabilities(),
      today: images[0] ?? null,
      version: '2.1.0',
      imageDir: settings.imageDir ?? MOCK_IMAGE_DIR,
      storage: stats(settings),
      isFirstRun: false,
    }
  },

  async today(force = false): Promise<ImageMeta> {
    await delay(force ? 420 : 120)
    if (!images.length) throw { kind: 'Bing', message: '暂无图片' }
    return images[0]
  },

  async listImages(
    page: number,
    pageSize: number,
    filter: ImageFilter,
    query?: string,
  ): Promise<Page<ImageMeta>> {
    await delay(160)
    let list = images
    if (filter === 'favorites') list = list.filter((image) => image.favorite)
    if (filter === 'recent') {
      const cutoff = dayjs().subtract(7, 'day')
      list = list.filter((image) => !dayjs(image.date).isBefore(cutoff, 'day'))
    }
    if (query?.trim()) {
      const q = query.trim().toLowerCase()
      list = list.filter(
        (image) =>
          image.title.toLowerCase().includes(q) ||
          image.copyright.toLowerCase().includes(q) ||
          image.date.includes(q),
      )
    }
    const start = (page - 1) * pageSize
    return {
      items: list.slice(start, start + pageSize),
      total: list.length,
      page,
      pageSize,
    }
  },

  async toggleFavorite(id: number, favorite: boolean): Promise<ImageMeta> {
    await delay(90)
    const image = images.find((item) => item.id === id)
    if (!image) throw { kind: 'InvalidInput', message: '图片不存在' }
    image.favorite = favorite
    const ids = images.filter((item) => item.favorite).map((item) => item.id)
    writeJson(FAVORITES_KEY, ids)
    return { ...image }
  },

  async deleteImage(id: number): Promise<void> {
    await delay(120)
    const image = images.find((item) => item.id === id)
    if (!image) return
    const deleted = new Set(readJson<string[]>(DELETED_KEY, []))
    deleted.add(image.date)
    writeJson(DELETED_KEY, [...deleted])
    images = images.filter((item) => item.id !== id)
  },

  async downloadImage(date: string, force = false): Promise<ImageMeta> {
    await delay(500)
    const existing = images.find((image) => image.date === date)
    if (existing && !force) return { ...existing }
    const index = Math.abs(dayjs(date).diff(dayjs(), 'day'))
    const [title, copyright] = TITLES[index % TITLES.length]
    const image: ImageMeta = {
      id: 9000 - index,
      date,
      title,
      copyright: `${copyright} (© Bing)`,
      copyrightLink: 'https://www.bing.com',
      filePath: makeSvg(index + 7, 1920, 1080),
      thumbPath: makeSvg(index + 7, 960, 540),
      width: 1920,
      height: 1080,
      bytes: 700_000 + index * 13_337,
      favorite: false,
      createdAt: dayjs().toISOString(),
    }
    images = [image, ...images.filter((item) => item.date !== date)].sort((a, b) =>
      b.date.localeCompare(a.date),
    )
    return { ...image }
  },

  downloadRange: (start, end) => runDownload(start, end),

  async cancelDownload(taskId: string): Promise<boolean> {
    const existed = activeTasks.delete(taskId)
    if (existed) {
      mockBus.emit('download://finished', {
        taskId,
        success: 0,
        failed: 0,
        skipped: 0,
        message: '任务已取消',
      })
    }
    return existed
  },

  async setWallpaper(date?: string, style?: WallpaperStyle): Promise<SetWallpaperResult> {
    await delay(520)
    const image = findImage(date)
    if (style) {
      const settings = loadSettings()
      settings.wallpaperStyle = style
      writeJson(SETTINGS_KEY, settings)
      mockBus.emit('settings://changed', settings)
    }
    mockBus.emit('wallpaper://changed', { date: image.date, path: image.filePath })
    return {
      success: true,
      message: '壁纸已更新（浏览器演示模式）',
      path: image.filePath,
      date: image.date,
    }
  },

  async setWallpaperFromPath(path: string, style: WallpaperStyle): Promise<void> {
    await delay(400)
    const settings = loadSettings()
    settings.wallpaperStyle = style
    writeJson(SETTINGS_KEY, settings)
    mockBus.emit('wallpaper://changed', { date: dayjs().format('YYYY-MM-DD'), path })
  },

  async getSettings(): Promise<Settings> {
    await delay(60)
    return loadSettings()
  },

  async updateSettings(patch: SettingsPatch): Promise<Settings> {
    await delay(140)
    const next = { ...loadSettings(), ...patch }
    writeJson(SETTINGS_KEY, next)
    mockBus.emit('settings://changed', next)
    return next
  },

  async getCapabilities(): Promise<PlatformCapabilities> {
    await delay(40)
    return capabilities()
  },

  async openImageFolder(): Promise<void> {
    console.info('[mock] open image folder', loadSettings().imageDir ?? MOCK_IMAGE_DIR)
  },

  async openExternal(url: string): Promise<void> {
    window.open(url, '_blank', 'noopener,noreferrer')
  },

  async checkUpdate(): Promise<UpdateInfo | null> {
    await delay(900)
    return null
  },

  async installUpdate(): Promise<void> {
    await delay(300)
  },

  async clearCache(): Promise<ClearCacheResult> {
    await delay(600)
    const settings = loadSettings()
    const cutoff = dayjs().subtract(settings.retentionDays, 'day')
    const removable = images.filter(
      (image) => !image.favorite && dayjs(image.date).isBefore(cutoff, 'day'),
    )
    const deleted = new Set(readJson<string[]>(DELETED_KEY, []))
    removable.forEach((image) => deleted.add(image.date))
    writeJson(DELETED_KEY, [...deleted])
    images = images.filter((image) => !removable.includes(image))
    return {
      removed: removable.length,
      bytesFreed: removable.reduce((sum, image) => sum + image.bytes, 0),
    }
  },

  async showMainWindow(): Promise<void> {
    console.info('[mock] show main window')
  },

  async quitApp(): Promise<void> {
    console.info('[mock] quit app')
  },
}
