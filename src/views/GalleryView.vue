<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { message, Modal } from 'ant-design-vue'
import {
  ClockCircleOutlined,
  CloudDownloadOutlined,
  FolderOpenOutlined,
  HeartOutlined,
  HddOutlined,
  PictureOutlined,
  ReloadOutlined,
  SearchOutlined,
} from '@ant-design/icons-vue'
import { api } from '@/api'
import type { ImageFilter, ImageMeta } from '@/api/types'
import { useAppStore } from '@/stores/app'
import { useGalleryStore } from '@/stores/gallery'
import { useSettingsStore } from '@/stores/settings'
import { useWallpaperStore } from '@/stores/wallpaper'
import { formatBytes } from '@/utils/format'
import { errorText } from '@/utils/error'
import EmptyState from '@/components/EmptyState.vue'
import WallpaperCard from '@/components/WallpaperCard.vue'
import { useRouter } from 'vue-router'

const { t } = useI18n()
const router = useRouter()
const app = useAppStore()
const gallery = useGalleryStore()
const settings = useSettingsStore()
const wallpaper = useWallpaperStore()

const searchInput = ref(gallery.query)
const viewMode = ref<'grid' | 'list'>('grid')
const downloadingId = ref<number | null>(null)
let searchTimer: ReturnType<typeof setTimeout> | undefined

const filterOptions = computed(() => [
  { label: t('common.all'), value: 'all' },
  { label: t('common.recent'), value: 'recent' },
  { label: t('common.favorites'), value: 'favorites' },
])

const viewOptions = computed(() => [
  { label: t('common.grid'), value: 'grid' },
  { label: t('common.list'), value: 'list' },
])

const autoText = computed(() =>
  settings.settings.autoSet ? settings.settings.autoSetTime : t('home.disabled'),
)

const scanPercent = computed(() => gallery.scanProgress?.percent ?? 0)

const showSkeleton = computed(
  () => (gallery.loading || gallery.scanning) && gallery.items.length === 0,
)

// Built from the structured payload: the backend `message` is Chinese-only.
const scanLabel = computed(() => {
  const progress = gallery.scanProgress
  if (!progress) return t('gallery.scanStarting')
  return progress.phase === 'index'
    ? t('gallery.scanIndexing', { current: progress.current, total: progress.total })
    : t('gallery.scanThumbnails', { current: progress.current, total: progress.total })
})

const scanSummary = computed(() => {
  const result = gallery.scanResult
  if (!result) return ''
  return t('gallery.scanDone', {
    imported: result.imported,
    relinked: result.relinked,
    thumbnails: result.thumbnails,
    failed: result.failed,
  })
})

const hasScanResult = computed(() => {
  const result = gallery.scanResult
  return Boolean(
    result &&
      (result.imported > 0 || result.relinked > 0 || result.thumbnails > 0 || result.failed > 0),
  )
})

watch(searchInput, (value) => {
  clearTimeout(searchTimer)
  searchTimer = setTimeout(() => void gallery.setQuery(value), 320)
})

onMounted(() => {
  void gallery.init()
  void gallery.load()
})

onBeforeUnmount(() => clearTimeout(searchTimer))

async function onRescan() {
  try {
    if (!(await gallery.rescan())) message.info(t('gallery.scanBusy'))
  } catch (e) {
    message.error(errorText(e))
  }
}

function onFilter(value: string | number) {
  void gallery.setFilter(value as ImageFilter)
}

async function openFolder() {
  try {
    await api.openImageFolder()
  } catch (e) {
    message.error(errorText(e))
  }
}

async function applyWallpaper(image: ImageMeta) {
  try {
    await api.setWallpaper(image.date, settings.settings.wallpaperStyle)
    message.success(t('home.wallpaperSet'))
  } catch (e) {
    message.error(errorText(e))
  }
}

async function favorite(image: ImageMeta) {
  try {
    const added = await gallery.toggleFavorite(image)
    message.success(added ? t('home.favoriteAdded') : t('home.favoriteRemoved'))
  } catch (e) {
    message.error(errorText(e))
  }
}

async function download(image: ImageMeta) {
  downloadingId.value = image.id
  try {
    await api.downloadImage(image.date, true)
    message.success(t('home.downloaded'))
    await gallery.load()
    void app.refreshStorage()
  } catch (e) {
    message.error(errorText(e))
  } finally {
    downloadingId.value = null
  }
}

function confirmDelete(image: ImageMeta) {
  Modal.confirm({
    title: t('gallery.deleteConfirm'),
    content: image.external ? t('gallery.deleteHintExternal') : t('gallery.deleteHint'),
    okText: t('common.delete'),
    okType: 'danger',
    cancelText: t('common.cancel'),
    async onOk() {
      try {
        await gallery.remove(image)
        message.success(t('gallery.deleted'))
        if (wallpaper.today?.id === image.id) void wallpaper.load()
      } catch (e) {
        message.error(errorText(e))
      }
    },
  })
}

function goDownloads() {
  void router.push('/downloads')
}

function goPage(page: number) {
  void gallery.setPage(page)
}
</script>

<template>
  <div class="page gallery-page">
    <header class="page-head">
      <div>
        <h1>{{ t('gallery.title') }}</h1>
        <p>{{ t('gallery.subtitle') }}</p>
      </div>
      <div class="page-actions">
        <a-button :loading="gallery.scanning" @click="onRescan">
          <ReloadOutlined v-if="!gallery.scanning" />
          {{ t('gallery.rescan') }}
        </a-button>
        <a-button @click="openFolder">
          <FolderOpenOutlined />
          {{ t('common.openFolder') }}
        </a-button>
        <a-button type="primary" @click="goDownloads">
          <CloudDownloadOutlined />
          {{ t('nav.downloads') }}
        </a-button>
      </div>
    </header>

    <section class="stats">
      <div class="stat glass">
        <span class="stat-icon"><PictureOutlined /></span>
        <div>
          <span class="stat-label">{{ t('gallery.totalImages') }}</span>
          <strong>{{ app.storage.count }}</strong>
        </div>
      </div>
      <div class="stat glass">
        <span class="stat-icon heart"><HeartOutlined /></span>
        <div>
          <span class="stat-label">{{ t('common.favorites') }}</span>
          <strong>{{ app.storage.favoriteCount }}</strong>
        </div>
      </div>
      <div class="stat glass">
        <span class="stat-icon"><HddOutlined /></span>
        <div>
          <span class="stat-label">{{ t('gallery.totalSize') }}</span>
          <strong>{{ formatBytes(app.storage.bytes) }}</strong>
        </div>
      </div>
      <div class="stat glass">
        <span class="stat-icon"><ClockCircleOutlined /></span>
        <div>
          <span class="stat-label">{{ t('gallery.autoTime') }}</span>
          <strong>{{ autoText }}</strong>
        </div>
      </div>
    </section>

    <section class="toolbar">
      <a-segmented :value="gallery.filter" :options="filterOptions" @change="onFilter" />
      <div class="toolbar-right">
        <a-input
          v-model:value="searchInput"
          class="search"
          allow-clear
          :placeholder="t('common.search')"
        >
          <template #prefix><SearchOutlined /></template>
        </a-input>
        <a-segmented v-model:value="viewMode" :options="viewOptions" class="view-toggle" />
      </div>
    </section>

    <div v-if="gallery.scanning" class="scan-panel glass">
      <div class="scan-head">
        <ReloadOutlined spin />
        <span class="scan-label">{{ scanLabel }}</span>
        <span class="scan-percent">{{ scanPercent }}%</span>
      </div>
      <a-progress :percent="scanPercent" size="small" :show-info="false" />
    </div>

    <a-alert
      v-else-if="hasScanResult"
      type="success"
      show-icon
      closable
      class="gallery-alert"
      :message="scanSummary"
      @close="gallery.scanResult = null"
    />

    <a-alert
      v-if="gallery.error"
      type="error"
      show-icon
      class="gallery-alert"
      :message="gallery.error.message"
      :description="t('error.networkHint')"
    >
      <template #action>
        <a-button size="small" @click="gallery.load()">{{ t('common.retry') }}</a-button>
      </template>
    </a-alert>

    <div v-if="showSkeleton" class="skeleton-grid">
      <div v-for="n in 8" :key="n" class="skeleton-card glass">
        <a-skeleton active :paragraph="{ rows: 2 }" />
      </div>
    </div>

    <EmptyState
      v-else-if="gallery.isEmpty && !gallery.scanning"
      :icon="PictureOutlined"
      :title="gallery.query ? t('gallery.searchEmpty') : t('gallery.emptyTitle')"
      :hint="gallery.query ? t('gallery.searchEmptyHint') : t('gallery.emptyHint')"
    >
      <template #action>
        <div class="empty-actions">
          <a-button @click="onRescan">
            <ReloadOutlined />
            {{ t('gallery.rescan') }}
          </a-button>
          <a-button type="primary" @click="goDownloads">
            {{ t('gallery.goDownload') }}
          </a-button>
        </div>
      </template>
    </EmptyState>

    <a-image-preview-group v-else>
      <div class="gallery-grid" :class="{ 'list-mode': viewMode === 'list' }">
        <WallpaperCard
          v-for="image in gallery.items"
          :key="image.id"
          :image="image"
          :class="{ downloading: downloadingId === image.id }"
          @apply="applyWallpaper"
          @favorite="favorite"
          @delete="confirmDelete"
          @download="download"
        />
      </div>
    </a-image-preview-group>

    <footer v-if="gallery.total > 0" class="gallery-footer">
      <span class="footer-text">
        {{
          t('gallery.showing', {
            from: (gallery.page - 1) * gallery.pageSize + 1,
            to: Math.min(gallery.page * gallery.pageSize, gallery.total),
            total: gallery.total,
          })
        }}
      </span>
      <a-pagination
        :current="gallery.page"
        :page-size="gallery.pageSize"
        :total="gallery.total"
        :show-size-changer="false"
        @change="goPage"
      />
    </footer>
  </div>
</template>

<style scoped>
.gallery-page {
  display: flex;
  flex-direction: column;
  gap: 18px;
}

.stats {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 12px;
}

.stat {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 14px 16px;
  border-radius: var(--bw-radius);
}

.stat-icon {
  display: grid;
  place-items: center;
  width: 38px;
  height: 38px;
  flex: none;
  border-radius: 11px;
  background: var(--bw-primary-soft);
  color: var(--bw-primary);
  font-size: 17px;
}

.stat-icon.heart {
  background: rgba(240, 68, 56, 0.12);
  color: #f04438;
}

.stat div {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.stat-label {
  font-size: 11px;
  color: var(--bw-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.stat strong {
  font-size: 17px;
  font-weight: 700;
  letter-spacing: -0.01em;
  font-variant-numeric: tabular-nums;
}

.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
  flex-wrap: wrap;
}

.toolbar-right {
  display: flex;
  align-items: center;
  gap: 10px;
  flex: 1;
  justify-content: flex-end;
}

.search {
  width: 240px;
}

.view-toggle :deep(.ant-segmented-item-label) {
  display: flex;
  align-items: center;
}

.gallery-alert {
  border-radius: 12px;
}

.scan-panel {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 12px 16px 14px;
  border-radius: var(--bw-radius);
}

.scan-head {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12.5px;
  color: var(--bw-ink);
}

.scan-label {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}

.scan-percent {
  flex: none;
  font-variant-numeric: tabular-nums;
  color: var(--bw-muted);
}

.scan-panel :deep(.ant-progress) {
  margin: 0;
}

.empty-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.gallery-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 14px;
}

.gallery-grid.list-mode {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.gallery-grid.list-mode :deep(.wallpaper-card) {
  flex-direction: row;
  align-items: stretch;
}

.gallery-grid.list-mode :deep(.thumb) {
  width: 230px;
  flex: none;
  aspect-ratio: 16 / 9;
}

.gallery-grid.list-mode :deep(.meta) {
  display: flex;
  flex-direction: column;
  justify-content: center;
  padding: 14px 18px;
}

.gallery-grid.list-mode :deep(.hover-actions) {
  opacity: 1;
  transform: none;
}

.skeleton-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 14px;
}

.skeleton-card {
  padding: 12px;
  border-radius: var(--bw-radius);
}

.gallery-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding-top: 4px;
}

.footer-text {
  font-size: 12px;
  color: var(--bw-muted);
}

@media (max-width: 1280px) {
  .gallery-grid,
  .skeleton-grid {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }
}

@media (max-width: 1000px) {
  .stats {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .gallery-grid,
  .skeleton-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 680px) {
  .gallery-grid,
  .skeleton-grid {
    grid-template-columns: minmax(0, 1fr);
  }

  .search {
    width: 100%;
  }

  .toolbar-right {
    width: 100%;
  }

  .gallery-grid.list-mode :deep(.thumb) {
    width: 130px;
  }
}
</style>
