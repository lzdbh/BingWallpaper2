import { convertFileSrc } from '@tauri-apps/api/core'
import dayjs from 'dayjs'
import { isTauri } from '@/api/events'
import type { ImageSize, WallpaperStyle } from '@/api/types'

export function assetUrl(path?: string | null): string {
  if (!path) return ''
  if (!isTauri || path.startsWith('data:') || path.startsWith('http') || path.startsWith('blob:')) {
    return path
  }
  return convertFileSrc(path)
}

export function formatBytes(bytes: number, digits = 1): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const index = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1)
  const value = bytes / 1024 ** index
  return `${value.toFixed(index === 0 ? 0 : digits)} ${units[index]}`
}

export function formatDate(date: string, locale = 'zh-CN'): string {
  const value = dayjs(date)
  if (!value.isValid()) return date
  return locale === 'zh-CN' ? value.format('YYYY年M月D日') : value.format('MMM D, YYYY')
}

export function formatDateShort(date: string, locale = 'zh-CN'): string {
  const value = dayjs(date)
  if (!value.isValid()) return date
  return locale === 'zh-CN' ? value.format('M月D日') : value.format('MMM D')
}

export function formatWeekday(date: string, locale = 'zh-CN'): string {
  const value = dayjs(date)
  if (!value.isValid()) return ''
  const weekdays =
    locale === 'zh-CN'
      ? ['星期日', '星期一', '星期二', '星期三', '星期四', '星期五', '星期六']
      : ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday']
  return weekdays[value.day()]
}

export function formatDateTime(value: Date | number = Date.now()): string {
  return dayjs(value).format('HH:mm:ss')
}

export const IMAGE_SIZE_OPTIONS: { value: ImageSize; label: string }[] = [
  { value: 'uhd', label: 'UHD 4K' },
  { value: '1920x1200', label: '1920 × 1200' },
  { value: '1920x1080', label: '1920 × 1080' },
  { value: '1366x768', label: '1366 × 768' },
]

export const WALLPAPER_STYLE_VALUES: WallpaperStyle[] = [
  'fill',
  'fit',
  'stretch',
  'tile',
  'center',
  'span',
]

export function styleLabelKey(style: WallpaperStyle): string {
  return `home.style${style.charAt(0).toUpperCase()}${style.slice(1)}`
}

export interface StyleOption {
  value: WallpaperStyle
  label: string
  disabled: boolean
}

/**
 * Options for the wallpaper-style select. `supported` comes from the platform
 * capability matrix; styles the current OS cannot apply stay visible but disabled.
 */
export function buildStyleOptions(
  supported: WallpaperStyle[],
  labelOf: (style: WallpaperStyle) => string,
): StyleOption[] {
  return WALLPAPER_STYLE_VALUES.map((style) => ({
    value: style,
    label: labelOf(style),
    disabled: !supported.includes(style),
  }))
}

export function sizeLabel(size: ImageSize): string {
  return IMAGE_SIZE_OPTIONS.find((option) => option.value === size)?.label ?? size
}
