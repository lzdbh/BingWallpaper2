import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api } from '@/api'
import type { Settings, SettingsPatch } from '@/api/types'
import { setI18nLocale } from '@/i18n'

export const DEFAULT_SETTINGS: Settings = {
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

export const useSettingsStore = defineStore('settings', () => {
  const settings = ref<Settings>({ ...DEFAULT_SETTINGS })
  const loading = ref(false)
  const saving = ref(false)
  const lastSavedAt = ref<number | null>(null)
  const error = ref<string | null>(null)

  function apply(next: Settings) {
    settings.value = next
    setI18nLocale(next.locale)
  }

  async function load() {
    loading.value = true
    error.value = null
    try {
      apply(await api.getSettings())
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    } finally {
      loading.value = false
    }
  }

  async function save(patch: SettingsPatch): Promise<Settings | null> {
    settings.value = { ...settings.value, ...patch }
    saving.value = true
    error.value = null
    try {
      const next = await api.updateSettings(patch)
      apply(next)
      lastSavedAt.value = Date.now()
      return next
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
      return null
    } finally {
      saving.value = false
    }
  }

  async function reset() {
    const { locale } = settings.value
    return save({ ...DEFAULT_SETTINGS, locale })
  }

  return {
    settings,
    loading,
    saving,
    lastSavedAt,
    error,
    apply,
    load,
    save,
    reset,
  }
})
