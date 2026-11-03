import { describe, expect, it } from 'vitest'
import type { ImageSize } from '@/api/types'
import {
  assetUrl,
  buildStyleOptions,
  formatBytes,
  formatDate,
  formatDateShort,
  formatWeekday,
  sizeLabel,
  styleLabelKey,
  WALLPAPER_STYLE_VALUES,
} from './format'

describe('formatBytes', () => {
  it('handles zero, negative and non-finite input', () => {
    expect(formatBytes(0)).toBe('0 B')
    expect(formatBytes(-5)).toBe('0 B')
    expect(formatBytes(Number.NaN)).toBe('0 B')
  })

  it('picks the right unit and precision', () => {
    expect(formatBytes(512)).toBe('512 B')
    expect(formatBytes(1024)).toBe('1.0 KB')
    expect(formatBytes(1536)).toBe('1.5 KB')
    expect(formatBytes(1024 ** 2)).toBe('1.0 MB')
    expect(formatBytes(1024 ** 3)).toBe('1.0 GB')
    expect(formatBytes(1024 ** 4)).toBe('1.0 TB')
  })

  it('honours a custom digit count', () => {
    expect(formatBytes(1024, 2)).toBe('1.00 KB')
  })
})

describe('formatDate / formatDateShort', () => {
  it('formats for zh-CN and en-US', () => {
    expect(formatDate('2026-11-02', 'zh-CN')).toBe('2026年11月2日')
    expect(formatDate('2026-11-02', 'en-US')).toBe('Nov 2, 2026')
    expect(formatDateShort('2026-11-02', 'zh-CN')).toBe('11月2日')
    expect(formatDateShort('2026-11-02', 'en-US')).toBe('Nov 2')
  })

  it('returns the raw value when the date is invalid', () => {
    expect(formatDate('not-a-date')).toBe('not-a-date')
    expect(formatDateShort('not-a-date')).toBe('not-a-date')
  })
})

describe('formatWeekday', () => {
  it('maps the weekday name in both locales', () => {
    // 2024-01-01 was a Monday, 2024-01-07 a Sunday.
    expect(formatWeekday('2024-01-01', 'zh-CN')).toBe('星期一')
    expect(formatWeekday('2024-01-01', 'en-US')).toBe('Monday')
    expect(formatWeekday('2024-01-07', 'zh-CN')).toBe('星期日')
    expect(formatWeekday('2024-01-07', 'en-US')).toBe('Sunday')
  })

  it('returns an empty string for an invalid date', () => {
    expect(formatWeekday('nope')).toBe('')
  })
})

describe('styleLabelKey', () => {
  it('builds the i18n key with a capitalised style', () => {
    expect(styleLabelKey('fill')).toBe('home.styleFill')
    expect(styleLabelKey('stretch')).toBe('home.styleStretch')
    expect(styleLabelKey('center')).toBe('home.styleCenter')
  })
})

describe('buildStyleOptions', () => {
  const label = (style: string) => style

  it('keeps every style visible and preserves the canonical order', () => {
    const options = buildStyleOptions(WALLPAPER_STYLE_VALUES, label)
    expect(options.map((option) => option.value)).toEqual(WALLPAPER_STYLE_VALUES)
    expect(options.every((option) => option.disabled === false)).toBe(true)
  })

  it('greys out styles the platform cannot apply', () => {
    const options = buildStyleOptions(['fill'], label)
    expect(options.filter((option) => !option.disabled).map((option) => option.value)).toEqual([
      'fill',
    ])
    expect(options.filter((option) => option.disabled).map((option) => option.value)).toEqual([
      'fit',
      'stretch',
      'tile',
      'center',
      'span',
    ])
  })

  it('uses the supplied label builder for each option', () => {
    const options = buildStyleOptions(['fill'], (style) => `home.style:${style}`)
    expect(options[0]?.label).toBe('home.style:fill')
  })
})

describe('sizeLabel', () => {
  it('returns the human label for a known size', () => {
    expect(sizeLabel('uhd')).toBe('UHD 4K')
    expect(sizeLabel('1920x1080')).toBe('1920 × 1080')
    expect(sizeLabel('1366x768')).toBe('1366 × 768')
  })

  it('falls back to the raw value for an unknown size', () => {
    expect(sizeLabel('weird' as unknown as ImageSize)).toBe('weird')
  })
})

describe('assetUrl', () => {
  it('returns empty for nullish input', () => {
    expect(assetUrl(null)).toBe('')
    expect(assetUrl(undefined)).toBe('')
    expect(assetUrl('')).toBe('')
  })

  it('passes through remote/data URLs unchanged', () => {
    expect(assetUrl('https://example.com/a.jpg')).toBe('https://example.com/a.jpg')
    expect(assetUrl('data:image/png;base64,AAA')).toBe('data:image/png;base64,AAA')
    expect(assetUrl('blob:xyz')).toBe('blob:xyz')
  })

  it('returns the local path as-is outside Tauri (jsdom has no __TAURI_INTERNALS__)', () => {
    expect(assetUrl('C:\\images\\bing.jpg')).toBe('C:\\images\\bing.jpg')
  })
})
