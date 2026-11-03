<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import {
  CloudDownloadOutlined,
  HddOutlined,
  HomeOutlined,
  InfoCircleOutlined,
  PictureOutlined,
  SettingOutlined,
} from '@ant-design/icons-vue'
import BrandMark from './BrandMark.vue'
import { useAppStore } from '@/stores/app'
import { useGalleryStore } from '@/stores/gallery'
import { formatBytes } from '@/utils/format'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const app = useAppStore()
const gallery = useGalleryStore()

const items = computed(() => [
  { name: 'home', path: '/', icon: HomeOutlined, label: t('nav.home') },
  {
    name: 'gallery',
    path: '/gallery',
    icon: PictureOutlined,
    label: t('nav.gallery'),
    count: app.storage.count || gallery.total,
  },
  { name: 'downloads', path: '/downloads', icon: CloudDownloadOutlined, label: t('nav.downloads') },
  { name: 'settings', path: '/settings', icon: SettingOutlined, label: t('nav.settings') },
  { name: 'about', path: '/about', icon: InfoCircleOutlined, label: t('nav.about') },
])

const storagePercent = computed(() => {
  const cap = 2 * 1024 ** 3
  return Math.min(100, Math.round((app.storage.bytes / cap) * 100))
})

function go(path: string) {
  void router.push(path)
}
</script>

<template>
  <aside class="sidebar">
    <div class="brand">
      <BrandMark />
      <div class="brand-text">
        <strong>{{ t('app.name') }}</strong>
        <span>{{ t('app.tagline') }}</span>
      </div>
    </div>

    <nav class="nav" aria-label="Main navigation">
      <button
        v-for="item in items"
        :key="item.name"
        type="button"
        class="nav-item"
        :class="{ active: route.name === item.name }"
        @click="go(item.path)"
      >
        <component :is="item.icon" class="nav-icon" />
        <span class="nav-label">{{ item.label }}</span>
        <span v-if="item.count" class="nav-count">{{ item.count }}</span>
      </button>
    </nav>

    <div class="footer">
      <div class="storage-card">
        <div class="storage-head">
          <HddOutlined />
          <span>{{ t('common.storage') }}</span>
          <strong>{{ formatBytes(app.storage.bytes) }}</strong>
        </div>
        <a-progress
          :percent="storagePercent"
          :show-info="false"
          size="small"
          :stroke-color="{ '0%': '#1677ff', '100%': '#69b1ff' }"
        />
        <span class="storage-meta">{{ app.storage.count }} {{ t('common.images') }} · {{ app.storage.favoriteCount }} {{ t('common.favorites') }}</span>
      </div>
      <div class="version">
        <span>v{{ app.version }}</span>
        <span v-if="!app.capabilities.os.startsWith('unknown')" class="dot">·</span>
        <span v-if="!app.capabilities.os.startsWith('unknown')">{{ app.capabilities.os }}</span>
      </div>
    </div>
  </aside>
</template>

<style scoped>
.sidebar {
  width: var(--bw-sidebar-width);
  flex: none;
  display: flex;
  flex-direction: column;
  gap: 18px;
  padding: 18px 14px 16px;
  border-right: 1px solid var(--bw-line);
  background: var(--bw-surface-muted);
  backdrop-filter: blur(var(--bw-blur));
  -webkit-backdrop-filter: blur(var(--bw-blur));
  overflow: hidden;
}

.brand {
  display: flex;
  align-items: center;
  gap: 11px;
  padding: 4px 8px 10px;
}

.brand-text {
  display: flex;
  flex-direction: column;
  min-width: 0;
  line-height: 1.25;
}

.brand-text strong {
  font-size: 14.5px;
  font-weight: 700;
  letter-spacing: 0.01em;
}

.brand-text span {
  font-size: 11px;
  color: var(--bw-muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.nav {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 11px;
  width: 100%;
  padding: 10px 12px;
  border: 0;
  border-radius: 12px;
  background: transparent;
  color: var(--bw-ink-soft);
  font-size: 13.5px;
  font-weight: 500;
  text-align: left;
  cursor: pointer;
  transition:
    background var(--bw-transition),
    color var(--bw-transition),
    transform var(--bw-transition);
}

.nav-item:hover {
  background: rgba(127, 127, 127, 0.1);
  color: var(--bw-ink);
}

.nav-item.active {
  background: var(--bw-primary-soft);
  color: var(--bw-primary);
  font-weight: 650;
}

.nav-item:active {
  transform: scale(0.985);
}

.nav-icon {
  font-size: 17px;
  flex: none;
}

.nav-label {
  flex: 1;
}

.nav-count {
  min-width: 22px;
  padding: 1px 7px;
  border-radius: 999px;
  background: rgba(22, 119, 255, 0.14);
  color: var(--bw-primary);
  font-size: 11px;
  font-weight: 700;
  text-align: center;
}

.footer {
  margin-top: auto;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.storage-card {
  padding: 12px;
  border: 1px solid var(--bw-line);
  border-radius: 14px;
  background: var(--bw-surface);
  box-shadow: var(--bw-shadow-soft);
}

.storage-head {
  display: flex;
  align-items: center;
  gap: 7px;
  font-size: 12px;
  color: var(--bw-muted);
  margin-bottom: 6px;
}

.storage-head strong {
  margin-left: auto;
  color: var(--bw-ink);
  font-size: 12.5px;
}

.storage-meta {
  display: block;
  margin-top: 4px;
  font-size: 11px;
  color: var(--bw-muted);
}

.version {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 0 8px;
  font-size: 11px;
  color: var(--bw-muted);
}

.dot {
  opacity: 0.5;
}
</style>
