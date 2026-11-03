import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { api } from '@/api'
import { onEvent } from '@/api/events'
import type {
  AppError,
  ImageFilter,
  ImageMeta,
  ScanFinished,
  ScanProgress,
} from '@/api/types'
import { normalizeError } from '@/utils/error'
import { useAppStore } from './app'

/**
 * A scan commits thumbnails in batches, so `library://changed` arrives several
 * times a second. One reload per window is enough to keep the grid fresh.
 */
const REFRESH_WINDOW_MS = 900

export const useGalleryStore = defineStore('gallery', () => {
  const items = ref<ImageMeta[]>([])
  const total = ref(0)
  const page = ref(1)
  const pageSize = ref(12)
  const filter = ref<ImageFilter>('all')
  const query = ref('')
  const loading = ref(false)
  const error = ref<AppError | null>(null)
  const scanning = ref(false)
  const scanProgress = ref<ScanProgress | null>(null)
  const scanResult = ref<ScanFinished | null>(null)
  let initialized = false
  let loadToken = 0
  let refreshTimer: ReturnType<typeof setTimeout> | undefined

  const pageCount = computed(() => Math.max(1, Math.ceil(total.value / pageSize.value)))
  const isEmpty = computed(() => !loading.value && items.value.length === 0)

  function scheduleRefresh() {
    if (refreshTimer !== undefined) return
    refreshTimer = setTimeout(() => {
      refreshTimer = undefined
      void load()
    }, REFRESH_WINDOW_MS)
  }

  async function load() {
    // A scan refresh can land while the user is paging; only the newest request
    // is allowed to write the list.
    const request = ++loadToken
    loading.value = true
    error.value = null
    try {
      const result = await api.listImages(page.value, pageSize.value, filter.value, query.value)
      if (request !== loadToken) return
      items.value = result.items
      total.value = result.total
      if (page.value > pageCount.value) {
        page.value = pageCount.value
        await load()
      }
    } catch (e) {
      if (request === loadToken) error.value = normalizeError(e)
    } finally {
      if (request === loadToken) loading.value = false
    }
  }

  async function init() {
    if (initialized) return
    initialized = true
    scanning.value = useAppStore().scanning

    await onEvent<ScanProgress>('library://scan-progress', (payload) => {
      scanning.value = true
      scanProgress.value = payload
    })
    await onEvent<ScanFinished>('library://scan-finished', (payload) => {
      scanning.value = false
      scanProgress.value = null
      scanResult.value = payload
      void load()
      void useAppStore().refreshStorage()
      // After `load()`: its synchronous prologue clears the error slot.
      if (payload.error) {
        error.value = { kind: 'Scan', message: payload.error }
      }
    })
    await onEvent('library://changed', () => scheduleRefresh())
  }

  /** `false` means the backend is still busy with a previous pass. */
  async function rescan(): Promise<boolean> {
    const started = await api.rescanLibrary()
    scanning.value = scanning.value || started
    if (started) {
      error.value = null
      scanResult.value = null
    }
    return started
  }

  async function setFilter(next: ImageFilter) {
    filter.value = next
    page.value = 1
    await load()
  }

  async function setPage(next: number) {
    page.value = next
    await load()
  }

  async function setQuery(next: string) {
    query.value = next
    page.value = 1
    await load()
  }

  async function setPageSize(next: number) {
    pageSize.value = next
    page.value = 1
    await load()
  }

  async function toggleFavorite(image: ImageMeta) {
    const next = !image.favorite
    const target = items.value.find((item) => item.id === image.id)
    if (target) target.favorite = next
    try {
      const updated = await api.toggleFavorite(image.id, next)
      if (target) Object.assign(target, updated)
      void useAppStore().refreshStorage()
      return next
    } catch (e) {
      if (target) target.favorite = !next
      error.value = normalizeError(e)
      throw e
    }
  }

  async function remove(image: ImageMeta) {
    await api.deleteImage(image.id)
    items.value = items.value.filter((item) => item.id !== image.id)
    total.value = Math.max(0, total.value - 1)
    if (items.value.length === 0 && page.value > 1) {
      page.value -= 1
      await load()
    }
    void useAppStore().refreshStorage()
  }

  return {
    items,
    total,
    page,
    pageSize,
    filter,
    query,
    loading,
    error,
    scanning,
    scanProgress,
    scanResult,
    pageCount,
    isEmpty,
    init,
    rescan,
    load,
    setFilter,
    setPage,
    setQuery,
    setPageSize,
    toggleFavorite,
    remove,
  }
})
