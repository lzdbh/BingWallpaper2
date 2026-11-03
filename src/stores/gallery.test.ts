import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import type { ImageMeta, Page } from '@/api/types'

const { listeners } = vi.hoisted(() => ({
  listeners: new Map<string, Array<(payload: never) => void>>(),
}))

vi.mock('@/api', () => ({
  isTauri: false,
  api: {
    listImages: vi.fn(),
    rescanLibrary: vi.fn(),
  },
}))

vi.mock('@/api/events', () => ({
  isTauri: false,
  onEvent: async (event: string, handler: (payload: never) => void) => {
    listeners.set(event, [...(listeners.get(event) ?? []), handler])
    return () => {
      listeners.set(
        event,
        (listeners.get(event) ?? []).filter((item) => item !== handler),
      )
    }
  },
}))

// The gallery store reaches for the app store only to refresh the storage stats.
const refreshStorage = vi.fn()
vi.mock('./app', () => ({
  useAppStore: () => ({ scanning: false, refreshStorage }),
}))

import { api } from '@/api'
import { useGalleryStore } from './gallery'

const listImages = api.listImages as unknown as ReturnType<typeof vi.fn>
const rescanLibrary = api.rescanLibrary as unknown as ReturnType<typeof vi.fn>

function image(id: number): ImageMeta {
  return {
    id,
    date: `2026-11-0${id}`,
    title: '',
    copyright: '',
    copyrightLink: '',
    filePath: `D:\\bing_pic\\2026110${id}.jpg`,
    thumbPath: '',
    width: 1920,
    height: 1080,
    bytes: 1_000_000,
    favorite: false,
    external: true,
    downloaded: true,
    createdAt: '2026-11-01T08:00:00',
  }
}

function emit<T>(event: string, payload: T) {
  for (const handler of listeners.get(event) ?? []) {
    ;(handler as (value: T) => void)(payload)
  }
}

/** Let the fire-and-forget reload behind `load()` settle. */
async function flush() {
  await new Promise((resolve) => setTimeout(resolve, 0))
}

describe('gallery store scan state', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    listeners.clear()
    listImages.mockResolvedValue({
      items: [image(1)],
      total: 1,
      page: 1,
      pageSize: 12,
    } satisfies Page<ImageMeta>)
  })

  it('subscribes once and mirrors the backend progress events', async () => {
    const store = useGalleryStore()
    await store.init()
    await store.init()

    expect(listeners.get('library://scan-progress')).toHaveLength(1)

    emit('library://scan-progress', {
      phase: 'thumbnail',
      current: 120,
      total: 2948,
      percent: 4,
      message: '正在生成缩略图 120/2948',
    })
    expect(store.scanning).toBe(true)
    expect(store.scanProgress?.percent).toBe(4)

    emit('library://scan-finished', {
      imported: 2948,
      relinked: 0,
      thumbnails: 120,
      failed: 0,
      error: null,
    })
    await flush()
    expect(store.scanning).toBe(false)
    expect(store.scanProgress).toBeNull()
    expect(store.scanResult?.imported).toBe(2948)
    expect(store.items).toHaveLength(1)
  })

  it('records a failed scan as an error but keeps the list usable', async () => {
    const store = useGalleryStore()
    await store.init()

    emit('library://scan-finished', {
      imported: 0,
      relinked: 0,
      thumbnails: 0,
      failed: 0,
      error: '图库目录不可读',
    })
    expect(store.error?.message).toBe('图库目录不可读')
    expect(store.scanning).toBe(false)
  })

  it('coalesces the burst of library changes into one reload per window', async () => {
    vi.useFakeTimers()
    try {
      const store = useGalleryStore()
      await store.init()
      listImages.mockClear()

      for (let i = 0; i < 20; i += 1) emit('library://changed', undefined)
      expect(listImages).not.toHaveBeenCalled()

      await vi.advanceTimersByTimeAsync(900)
      expect(listImages).toHaveBeenCalledTimes(1)
    } finally {
      vi.useRealTimers()
    }
  })

  it('reports a busy backend instead of starting a second pass', async () => {
    const store = useGalleryStore()
    rescanLibrary.mockResolvedValue(false)

    expect(await store.rescan()).toBe(false)
    expect(store.scanResult).toBeNull()

    rescanLibrary.mockResolvedValue(true)
    expect(await store.rescan()).toBe(true)
    expect(store.scanning).toBe(true)
  })
})
