import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api } from '@/api'
import type { AppError, ImageMeta, WallpaperStyle } from '@/api/types'
import { normalizeError } from '@/utils/error'

export const useWallpaperStore = defineStore('wallpaper', () => {
  const today = ref<ImageMeta | null>(null)
  const loading = ref(false)
  const setting = ref(false)
  const error = ref<AppError | null>(null)

  function apply(image: ImageMeta | null) {
    today.value = image
  }

  async function load(force = false) {
    loading.value = true
    error.value = null
    try {
      today.value = await api.today(force)
      return today.value
    } catch (e) {
      error.value = normalizeError(e)
      return null
    } finally {
      loading.value = false
    }
  }

  async function setAsWallpaper(style?: WallpaperStyle) {
    setting.value = true
    error.value = null
    try {
      const result = await api.setWallpaper(today.value?.date, style)
      if (!result.success) throw new Error(result.message)
      return result
    } catch (e) {
      error.value = normalizeError(e)
      throw e
    } finally {
      setting.value = false
    }
  }

  async function toggleFavorite() {
    if (!today.value) return
    const next = !today.value.favorite
    today.value = { ...today.value, favorite: next }
    try {
      today.value = await api.toggleFavorite(today.value.id, next)
      return next
    } catch (e) {
      if (today.value) today.value = { ...today.value, favorite: !next }
      error.value = normalizeError(e)
      throw e
    }
  }

  async function downloadToday(force = false) {
    if (!today.value) return null
    loading.value = true
    try {
      const image = await api.downloadImage(today.value.date, force)
      today.value = image
      return image
    } catch (e) {
      error.value = normalizeError(e)
      return null
    } finally {
      loading.value = false
    }
  }

  return {
    today,
    loading,
    setting,
    error,
    apply,
    load,
    setAsWallpaper,
    toggleFavorite,
    downloadToday,
  }
})
