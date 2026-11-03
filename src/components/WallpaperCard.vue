<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  DeleteOutlined,
  DownloadOutlined,
  HeartFilled,
  HeartOutlined,
  ThunderboltOutlined,
} from '@ant-design/icons-vue'
import type { ImageMeta } from '@/api/types'
import { assetUrl, formatBytes, formatDateShort } from '@/utils/format'

const props = defineProps<{
  image: ImageMeta
}>()

const emit = defineEmits<{
  (e: 'apply', image: ImageMeta): void
  (e: 'favorite', image: ImageMeta): void
  (e: 'delete', image: ImageMeta): void
  (e: 'download', image: ImageMeta): void
}>()

const { t, locale } = useI18n()

const thumb = computed(() => assetUrl(props.image.thumbPath || props.image.filePath))
const preview = computed(() => ({ src: assetUrl(props.image.filePath) }))
const dateLabel = computed(() => formatDateShort(props.image.date, locale.value))
// Scanned files carry no Bing metadata, so the date stands in for the title.
const titleText = computed(() => props.image.title || dateLabel.value)
</script>

<template>
  <article class="wallpaper-card" :class="{ favorite: image.favorite }">
    <div class="thumb">
      <a-image
        :src="thumb"
        :preview="preview"
        :alt="titleText"
        decoding="async"
        :root-class-name="'card-image'"
      />
      <span class="date-badge">{{ dateLabel }}</span>
      <button
        type="button"
        class="fav-btn"
        :class="{ active: image.favorite }"
        :aria-label="image.favorite ? t('common.unfavorite') : t('common.favorite')"
        :title="image.favorite ? t('common.unfavorite') : t('common.favorite')"
        @click.stop="emit('favorite', image)"
      >
        <HeartFilled v-if="image.favorite" />
        <HeartOutlined v-else />
      </button>
      <div class="hover-actions">
        <a-button type="primary" size="small" @click.stop="emit('apply', image)">
          <ThunderboltOutlined />
          {{ t('gallery.setFromCard') }}
        </a-button>
        <a-button
          size="small"
          danger
          :title="t('common.delete')"
          @click.stop="emit('delete', image)"
        >
          <DeleteOutlined />
        </a-button>
      </div>

      <!-- Every gallery entry is a local file, so the download action only earns a
          permanent place once the file has gone missing outside the app. -->
      <a-button
        v-if="!image.downloaded"
        class="download-missing"
        type="primary"
        size="small"
        @click.stop="emit('download', image)"
      >
        <DownloadOutlined />
        {{ t('common.download') }}
      </a-button>
    </div>

    <div class="meta">
      <h3 class="card-title ellipsis" :title="titleText">{{ titleText }}</h3>
      <p v-if="image.copyright" class="card-copy ellipsis" :title="image.copyright">
        {{ image.copyright }}
      </p>
      <div class="card-tags">
        <span>{{ image.date }}</span>
        <span class="sep">·</span>
        <span>{{ formatBytes(image.bytes) }}</span>
        <span v-if="image.external" class="tag-external">{{ t('gallery.externalTag') }}</span>
      </div>
    </div>
  </article>
</template>

<style scoped>
.wallpaper-card {
  position: relative;
  display: flex;
  flex-direction: column;
  min-width: 0;
  overflow: hidden;
  border: 1px solid var(--bw-line);
  border-radius: var(--bw-radius);
  background: var(--bw-surface);
  box-shadow: var(--bw-shadow-soft);
  transition:
    transform var(--bw-transition),
    box-shadow var(--bw-transition),
    border-color var(--bw-transition);
}

.wallpaper-card:hover {
  transform: translateY(-3px);
  box-shadow: var(--bw-shadow);
  border-color: rgba(22, 119, 255, 0.32);
}

.thumb {
  position: relative;
  aspect-ratio: 16 / 10;
  overflow: hidden;
  background: linear-gradient(135deg, #dfe7f5, #c7d4ea);
}

.thumb :deep(.ant-image) {
  width: 100%;
  height: 100%;
  display: block;
}

.thumb :deep(img) {
  width: 100%;
  height: 100%;
  object-fit: cover;
  transition: transform 420ms cubic-bezier(0.2, 0, 0, 1);
}

.wallpaper-card:hover .thumb :deep(img) {
  transform: scale(1.035);
}

.date-badge {
  position: absolute;
  left: 10px;
  top: 10px;
  padding: 3px 9px;
  border-radius: 999px;
  background: rgba(8, 15, 26, 0.5);
  color: #fff;
  font-size: 11px;
  font-weight: 600;
  backdrop-filter: blur(8px);
  letter-spacing: 0.02em;
}

.fav-btn {
  position: absolute;
  right: 10px;
  top: 10px;
  display: grid;
  place-items: center;
  width: 30px;
  height: 30px;
  border: 0;
  border-radius: 999px;
  background: rgba(8, 15, 26, 0.42);
  color: #fff;
  font-size: 14px;
  cursor: pointer;
  backdrop-filter: blur(8px);
  transition:
    background var(--bw-transition),
    color var(--bw-transition),
    transform var(--bw-transition);
}

.fav-btn:hover {
  transform: scale(1.08);
  background: rgba(8, 15, 26, 0.62);
}

.fav-btn.active {
  color: #ff5d73;
}

.hover-actions {
  position: absolute;
  inset: auto 10px 10px 10px;
  display: flex;
  justify-content: flex-end;
  gap: 6px;
  opacity: 0;
  transform: translateY(6px);
  transition:
    opacity var(--bw-transition),
    transform var(--bw-transition);
}

.wallpaper-card:hover .hover-actions,
.wallpaper-card:focus-within .hover-actions {
  opacity: 1;
  transform: translateY(0);
}

.download-missing {
  position: absolute;
  left: 10px;
  bottom: 10px;
  z-index: 2;
  box-shadow: var(--bw-shadow-soft);
}

.meta {
  padding: 11px 13px 12px;
  min-width: 0;
}

.card-title {
  margin: 0;
  font-size: 13.5px;
  font-weight: 650;
  color: var(--bw-ink);
}

.card-copy {
  margin: 3px 0 0;
  font-size: 11.5px;
  color: var(--bw-muted);
}

.card-tags {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 8px;
  font-size: 10.5px;
  color: var(--bw-muted);
  font-variant-numeric: tabular-nums;
}

.sep {
  opacity: 0.5;
}

.tag-external {
  padding: 1px 7px;
  border-radius: 999px;
  background: var(--bw-primary-soft);
  color: var(--bw-primary);
  font-weight: 600;
}
</style>
