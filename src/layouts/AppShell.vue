<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { RouterLink, useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { message } from 'ant-design-vue'
import {
  BulbOutlined,
  CloudDownloadOutlined,
  FolderOpenOutlined,
  HomeOutlined,
  InfoCircleOutlined,
  PictureOutlined,
  SettingOutlined,
} from '@ant-design/icons-vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { api } from '@/api'
import { isTauri, onEvent } from '@/api/events'
import type { Settings } from '@/api/types'
import { useAppStore } from '@/stores/app'
import { useDownloadStore } from '@/stores/download'
import { useSettingsStore } from '@/stores/settings'
import { useTheme } from '@/composables/useTheme'
import { notify } from '@/composables/useNotification'
import { errorText } from '@/utils/error'
import BrandMark from '@/components/BrandMark.vue'
import SideNav from '@/components/SideNav.vue'
import UpdateDialog from '@/components/UpdateDialog.vue'
import WindowControls from '@/components/WindowControls.vue'

const route = useRoute()
const router = useRouter()
const { t } = useI18n()
const app = useAppStore()
const settings = useSettingsStore()
const downloads = useDownloadStore()
const { isDark } = useTheme()

const immersive = computed(() => route.meta.immersive === true)
const pageTitle = computed(() =>
  route.meta.title ? t(String(route.meta.title)) : t('app.name'),
)

const navItems = computed(() => [
  { name: 'home', path: '/', icon: HomeOutlined, label: t('nav.home') },
  { name: 'gallery', path: '/gallery', icon: PictureOutlined, label: t('nav.gallery') },
  { name: 'downloads', path: '/downloads', icon: CloudDownloadOutlined, label: t('nav.downloads') },
  { name: 'settings', path: '/settings', icon: SettingOutlined, label: t('nav.settings') },
  { name: 'about', path: '/about', icon: InfoCircleOutlined, label: t('nav.about') },
])

const showUpdate = ref(false)
let unlistenClose: (() => void) | undefined
let unlistenNav: (() => void) | undefined
let unlistenSettings: (() => void) | undefined
let unlistenWallpaper: (() => void) | undefined

onMounted(async () => {
  await downloads.init()

  if (isTauri) {
    const win = getCurrentWindow()
    unlistenClose = await win.onCloseRequested(async (event) => {
      if (settings.settings.closeToTray) {
        event.preventDefault()
        await win.hide()
      }
    })
  }

  unlistenNav = await onEvent<string>('app://navigate', (path) => {
    void router.push(path)
  })
  unlistenSettings = await onEvent<Settings>('settings://changed', (next) => {
    settings.apply(next)
  })
  unlistenWallpaper = await onEvent<{ date: string; path: string }>(
    'wallpaper://changed',
    () => {
      if (settings.settings.notifications) {
        void notify(t('app.name'), t('home.wallpaperSet'))
      }
      void app.refreshStorage()
    },
  )
})

watch(
  () => app.ready,
  async (ready) => {
    if (!ready || !settings.settings.autoCheckUpdate) return
    const info = await app.checkUpdate(true)
    if (info) showUpdate.value = true
  },
  { immediate: true },
)

onBeforeUnmount(() => {
  unlistenClose?.()
  unlistenNav?.()
  unlistenSettings?.()
  unlistenWallpaper?.()
})

async function openFolder() {
  try {
    await api.openImageFolder()
  } catch (e) {
    message.error(errorText(e))
  }
}

async function toggleTheme() {
  await settings.save({ theme: isDark.value ? 'light' : 'dark' })
}

async function retryBoot() {
  try {
    await app.bootstrap()
  } catch (e) {
    message.error(errorText(e))
  }
}
</script>

<template>
  <div class="shell" :class="{ immersive }">
    <header class="titlebar">
      <div v-if="immersive" class="brand drag-region">
        <BrandMark />
        <div class="brand-text">
          <strong>{{ t('app.name') }}</strong>
          <span>{{ t('app.tagline') }}</span>
        </div>
      </div>
      <div v-else class="crumbs drag-region">
        <BrandMark class="crumb-mark" />
        <span class="crumb-app">{{ t('app.name') }}</span>
        <span class="crumb-slash">/</span>
        <strong>{{ pageTitle }}</strong>
      </div>

      <nav v-if="immersive" class="top-nav no-drag" aria-label="Primary">
        <RouterLink
          v-for="item in navItems"
          :key="item.name"
          :to="item.path"
          class="top-link"
          :class="{ active: route.name === item.name }"
        >
          <component :is="item.icon" />
          <span>{{ item.label }}</span>
        </RouterLink>
      </nav>

      <div class="drag-fill" data-tauri-drag-region></div>

      <div class="toolbar no-drag">
        <a-tooltip :title="t('common.openFolder')" placement="bottom">
          <button
            type="button"
            class="icon-btn"
            :aria-label="t('common.openFolder')"
            @click="openFolder"
          >
            <FolderOpenOutlined />
          </button>
        </a-tooltip>
        <a-tooltip :title="t('settings.theme')" placement="bottom">
          <button
            type="button"
            class="icon-btn"
            :aria-label="t('settings.theme')"
            @click="toggleTheme"
          >
            <BulbOutlined />
          </button>
        </a-tooltip>
      </div>

      <WindowControls />
    </header>

    <div class="body">
      <SideNav v-if="!immersive" />
      <main class="content" :class="{ 'content-scroll': !immersive }">
        <router-view v-slot="{ Component }">
          <transition name="fade" mode="out-in">
            <component :is="Component" />
          </transition>
        </router-view>
      </main>
    </div>

    <div v-if="!app.ready" class="boot">
      <div class="boot-card glass-strong">
        <BrandMark />
        <a-spin size="large" />
        <p v-if="!app.bootError">{{ t('common.loading') }}</p>
        <template v-else>
          <p class="boot-error">{{ app.bootError }}</p>
          <a-button type="primary" @click="retryBoot">{{ t('common.retry') }}</a-button>
        </template>
      </div>
    </div>

    <UpdateDialog v-if="app.update" v-model:open="showUpdate" />
  </div>
</template>

<style scoped>
.shell {
  position: relative;
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100%;
  overflow: hidden;
  background: var(--bw-bg);
}

.titlebar {
  position: relative;
  z-index: 30;
  display: flex;
  align-items: center;
  gap: 10px;
  height: var(--bw-titlebar-height);
  padding: 0 0 0 16px;
  flex: none;
  background: var(--bw-surface-strong);
  border-bottom: 1px solid var(--bw-line);
  backdrop-filter: blur(var(--bw-blur)) saturate(140%);
  -webkit-backdrop-filter: blur(var(--bw-blur)) saturate(140%);
  color: var(--bw-ink);
  user-select: none;
}

.immersive .titlebar {
  position: absolute;
  inset: 0 0 auto 0;
  background: linear-gradient(
    to bottom,
    rgba(4, 10, 20, 0.56) 0%,
    rgba(4, 10, 20, 0.18) 62%,
    rgba(4, 10, 20, 0) 100%
  );
  border-bottom: 0;
  color: #fff;
  backdrop-filter: none;
  -webkit-backdrop-filter: none;
}

.brand {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
  padding-right: 6px;
}

.brand-text {
  display: flex;
  flex-direction: column;
  line-height: 1.2;
  min-width: 0;
}

.brand-text strong {
  font-size: 13.5px;
  font-weight: 700;
  letter-spacing: 0.01em;
}

.brand-text span {
  font-size: 10.5px;
  opacity: 0.68;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.crumbs {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  padding-right: 8px;
  font-size: 12.5px;
}

.crumb-mark {
  width: 24px !important;
  height: 24px !important;
}

.crumb-app {
  color: var(--bw-muted);
}

.crumb-slash {
  color: var(--bw-line-strong);
}

.crumbs strong {
  font-size: 13px;
  font-weight: 650;
}

.top-nav {
  display: flex;
  align-items: center;
  gap: 2px;
  margin-left: 14px;
}

.top-link {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 7px 11px;
  border-radius: 999px;
  color: rgba(255, 255, 255, 0.76);
  font-size: 12.5px;
  font-weight: 500;
  text-decoration: none;
  transition:
    background var(--bw-transition),
    color var(--bw-transition);
}

.top-link:hover {
  background: rgba(255, 255, 255, 0.14);
  color: #fff;
  text-decoration: none;
}

.top-link.active {
  background: rgba(255, 255, 255, 0.2);
  color: #fff;
  font-weight: 650;
}

.drag-fill {
  flex: 1;
  align-self: stretch;
  min-width: 20px;
}

.toolbar {
  display: flex;
  align-items: center;
  gap: 4px;
  padding-right: 4px;
}

.icon-btn {
  display: grid;
  place-items: center;
  width: 34px;
  height: 34px;
  border: 0;
  border-radius: 9px;
  background: transparent;
  color: inherit;
  font-size: 15px;
  cursor: pointer;
  opacity: 0.78;
  transition:
    background var(--bw-transition),
    opacity var(--bw-transition);
}

.icon-btn:hover {
  opacity: 1;
  background: rgba(127, 127, 127, 0.16);
}

.immersive .icon-btn:hover {
  background: rgba(255, 255, 255, 0.16);
}

.icon-btn:focus-visible {
  outline: 2px solid var(--bw-primary);
  outline-offset: -2px;
}

.body {
  display: flex;
  flex: 1;
  min-height: 0;
}

.immersive .body {
  height: 100%;
}

.content {
  flex: 1;
  min-width: 0;
  min-height: 0;
  position: relative;
}

.content-scroll {
  overflow-y: auto;
  overflow-x: hidden;
  background:
    radial-gradient(circle at 82% -10%, rgba(22, 119, 255, 0.08), transparent 46%),
    var(--bw-bg);
}

.boot {
  position: absolute;
  inset: 0;
  z-index: 60;
  display: grid;
  place-items: center;
  background: color-mix(in srgb, var(--bw-bg) 76%, transparent);
  backdrop-filter: blur(10px);
}

.boot-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 14px;
  min-width: 260px;
  padding: 30px 34px;
  border-radius: var(--bw-radius-lg);
  text-align: center;
}

.boot-card p {
  margin: 0;
  font-size: 12.5px;
  color: var(--bw-muted);
}

.boot-error {
  max-width: 320px;
  color: var(--bw-danger) !important;
  line-height: 1.6;
}

@media (max-width: 900px) {
  .top-nav {
    display: none;
  }
}
</style>
