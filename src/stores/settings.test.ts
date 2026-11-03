import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'

// Isolate the store from the real IPC layer and i18n bootstrap.
vi.mock('@/i18n', () => ({ setI18nLocale: vi.fn() }))
vi.mock('@/api', () => ({
  isTauri: false,
  api: {
    getSettings: vi.fn(),
    updateSettings: vi.fn(),
  },
}))

import { api } from '@/api'
import { DEFAULT_SETTINGS, useSettingsStore } from './settings'

// The mocked functions are untyped `vi.fn()`s at runtime; cast for assertions.
const getSettings = api.getSettings as unknown as ReturnType<typeof vi.fn>
const updateSettings = api.updateSettings as unknown as ReturnType<typeof vi.fn>

describe('settings store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('starts from DEFAULT_SETTINGS with a clean status', () => {
    const store = useSettingsStore()
    expect(store.settings).toEqual(DEFAULT_SETTINGS)
    expect(store.loading).toBe(false)
    expect(store.saving).toBe(false)
    expect(store.error).toBeNull()
    expect(store.lastSavedAt).toBeNull()
  })

  it('load() applies the fetched settings', async () => {
    getSettings.mockResolvedValue({
      ...DEFAULT_SETTINGS,
      locale: 'en-US',
      retentionDays: 10,
    })
    const store = useSettingsStore()
    await store.load()

    expect(getSettings).toHaveBeenCalledTimes(1)
    expect(store.settings.locale).toBe('en-US')
    expect(store.settings.retentionDays).toBe(10)
    expect(store.loading).toBe(false)
    expect(store.error).toBeNull()
  })

  it('load() records the error message and stops loading on failure', async () => {
    getSettings.mockRejectedValue(new Error('boom'))
    const store = useSettingsStore()
    await store.load()

    expect(store.error).toBe('boom')
    expect(store.loading).toBe(false)
  })

  it('save() sends the patch and applies the server result', async () => {
    updateSettings.mockResolvedValue({ ...DEFAULT_SETTINGS, retentionDays: 60 })
    const store = useSettingsStore()
    const result = await store.save({ retentionDays: 60 })

    expect(updateSettings).toHaveBeenCalledWith({ retentionDays: 60 })
    expect(result?.retentionDays).toBe(60)
    expect(store.settings.retentionDays).toBe(60)
    expect(store.saving).toBe(false)
    expect(store.error).toBeNull()
    expect(store.lastSavedAt).not.toBeNull()
  })

  it('save() optimistically merges the patch before the server responds', async () => {
    let resolve!: (value: unknown) => void
    updateSettings.mockReturnValue(new Promise((r) => (resolve = r)))
    const store = useSettingsStore()

    const pending = store.save({ retentionDays: 45 })
    // Optimistic update is visible while the request is in flight.
    expect(store.settings.retentionDays).toBe(45)
    expect(store.saving).toBe(true)

    resolve({ ...DEFAULT_SETTINGS, retentionDays: 45 })
    await pending
    expect(store.saving).toBe(false)
  })

  it('save() returns null and sets error on failure', async () => {
    updateSettings.mockRejectedValue(new Error('nope'))
    const store = useSettingsStore()
    const result = await store.save({ retentionDays: 5 })

    expect(result).toBeNull()
    expect(store.error).toBe('nope')
    expect(store.saving).toBe(false)
  })

  it('reset() restores defaults while keeping the current locale', async () => {
    updateSettings.mockResolvedValue({ ...DEFAULT_SETTINGS })
    const store = useSettingsStore()
    store.settings = { ...DEFAULT_SETTINGS, locale: 'en-US', retentionDays: 99 }

    await store.reset()
    expect(updateSettings).toHaveBeenCalledWith({
      ...DEFAULT_SETTINGS,
      locale: 'en-US',
    })
  })
})
