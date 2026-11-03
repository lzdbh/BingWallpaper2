import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { api } from '@/api'
import type {
  PlatformCapabilities,
  StorageStats,
  UpdateInfo,
} from '@/api/types'
import { useSettingsStore } from './settings'
import { useWallpaperStore } from './wallpaper'

const FALLBACK_CAPABILITIES: PlatformCapabilities = {
  os: 'unknown',
  arch: 'unknown',
  styles: ['fill', 'fit', 'stretch', 'tile', 'center', 'span'],
  multiMonitor: false,
  desktopEnvironment: null,
}

export const useAppStore = defineStore('app', () => {
  const ready = ref(false)
  const booting = ref(false)
  const version = ref('2.1.0')
  const capabilities = ref<PlatformCapabilities>({ ...FALLBACK_CAPABILITIES })
  const imageDir = ref('')
  const storage = ref<StorageStats>({ count: 0, favoriteCount: 0, bytes: 0 })
  const isFirstRun = ref(false)
  /** Whether the backend was still indexing the gallery directory at bootstrap. */
  const scanning = ref(false)
  const update = ref<UpdateInfo | null>(null)
  const checkingUpdate = ref(false)
  const installing = ref(false)
  const bootError = ref<string | null>(null)

  const platformLabel = computed(() => {
    const os = capabilities.value.os
    const map: Record<string, string> = {
      windows: 'Windows',
      macos: 'macOS',
      linux: 'Linux',
    }
    return `${map[os] ?? os} · ${capabilities.value.arch}`
  })

  async function bootstrap() {
    if (booting.value) return
    booting.value = true
    bootError.value = null
    try {
      const state = await api.bootstrap()
      version.value = state.version
      capabilities.value = state.capabilities
      imageDir.value = state.imageDir
      storage.value = state.storage
      isFirstRun.value = state.isFirstRun
      scanning.value = state.scanning
      useSettingsStore().apply(state.settings)
      if (state.today) useWallpaperStore().apply(state.today)
      ready.value = true
    } catch (e) {
      bootError.value = e instanceof Error ? e.message : String(e)
      throw e
    } finally {
      booting.value = false
    }
  }

  async function refreshStorage() {
    try {
      const state = await api.bootstrap()
      storage.value = state.storage
      imageDir.value = state.imageDir
      if (state.today) useWallpaperStore().apply(state.today)
    } catch {
      /* non-critical */
    }
  }

  async function checkUpdate(silent = false): Promise<UpdateInfo | null> {
    if (checkingUpdate.value) return update.value
    checkingUpdate.value = true
    try {
      update.value = await api.checkUpdate()
      if (update.value) {
        return update.value
      }
      if (!silent) return null
      return null
    } finally {
      checkingUpdate.value = false
    }
  }

  async function installUpdate() {
    installing.value = true
    try {
      await api.installUpdate()
      update.value = null
    } finally {
      installing.value = false
    }
  }

  return {
    ready,
    booting,
    version,
    capabilities,
    imageDir,
    storage,
    isFirstRun,
    scanning,
    update,
    checkingUpdate,
    installing,
    bootError,
    platformLabel,
    bootstrap,
    refreshStorage,
    checkUpdate,
    installUpdate,
  }
})
