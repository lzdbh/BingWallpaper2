import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { api } from '@/api'
import type { AppError, ImageFilter, ImageMeta } from '@/api/types'
import { normalizeError } from '@/utils/error'
import { useAppStore } from './app'

export const useGalleryStore = defineStore('gallery', () => {
  const items = ref<ImageMeta[]>([])
  const total = ref(0)
  const page = ref(1)
  const pageSize = ref(12)
  const filter = ref<ImageFilter>('all')
  const query = ref('')
  const loading = ref(false)
  const error = ref<AppError | null>(null)

  const pageCount = computed(() => Math.max(1, Math.ceil(total.value / pageSize.value)))
  const isEmpty = computed(() => !loading.value && items.value.length === 0)

  async function load() {
    loading.value = true
    error.value = null
    try {
      const result = await api.listImages(page.value, pageSize.value, filter.value, query.value)
      items.value = result.items
      total.value = result.total
      if (page.value > pageCount.value) {
        page.value = pageCount.value
        await load()
      }
    } catch (e) {
      error.value = normalizeError(e)
    } finally {
      loading.value = false
    }
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
    pageCount,
    isEmpty,
    load,
    setFilter,
    setPage,
    setQuery,
    setPageSize,
    toggleFavorite,
    remove,
  }
})
