<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { message, Modal } from 'ant-design-vue'
import dayjs, { type Dayjs } from 'dayjs'
import { open } from '@tauri-apps/plugin-dialog'
import {
  BellOutlined,
  CloudSyncOutlined,
  CloseOutlined,
  FolderOpenOutlined,
  HddOutlined,
  PictureOutlined,
  ReloadOutlined,
  SettingOutlined,
  ThunderboltOutlined,
} from '@ant-design/icons-vue'
import { api, isTauri } from '@/api'
import type {
  ImageSize,
  SettingsPatch,
  ThemeMode,
  UpdateChannel,
  WallpaperStyle,
} from '@/api/types'
import { SUPPORTED_LOCALES } from '@/i18n'
import { useAppStore } from '@/stores/app'
import { useSettingsStore } from '@/stores/settings'
import { useWallpaperStore } from '@/stores/wallpaper'
import {
  IMAGE_SIZE_OPTIONS,
  assetUrl,
  buildStyleOptions,
  formatBytes,
  styleLabelKey,
} from '@/utils/format'
import { errorText } from '@/utils/error'
import SettingRow from '@/components/SettingRow.vue'
import SettingSection from '@/components/SettingSection.vue'

const { t } = useI18n()
const app = useAppStore()
const settings = useSettingsStore()
const wallpaper = useWallpaperStore()

const clearing = ref(false)
const proxyInput = ref(settings.settings.proxy ?? '')

watch(
  () => settings.settings.proxy,
  (value) => {
    proxyInput.value = value ?? ''
  },
)

const imageSize = computed({
  get: () => settings.settings.imageSize,
  set: (value: ImageSize) => void update({ imageSize: value }),
})

const wallpaperStyle = computed({
  get: () => settings.settings.wallpaperStyle,
  set: (value: WallpaperStyle) => void update({ wallpaperStyle: value }),
})

const autoStart = computed({
  get: () => settings.settings.autoStart,
  set: (value: boolean) => void update({ autoStart: value }),
})

const autoSet = computed({
  get: () => settings.settings.autoSet,
  set: (value: boolean) => void update({ autoSet: value }),
})

const catchUp = computed({
  get: () => settings.settings.catchUp,
  set: (value: boolean) => void update({ catchUp: value }),
})

const notifications = computed({
  get: () => settings.settings.notifications,
  set: (value: boolean) => void update({ notifications: value }),
})

const closeToTray = computed({
  get: () => settings.settings.closeToTray,
  set: (value: boolean) => void update({ closeToTray: value }),
})

const autoCheckUpdate = computed({
  get: () => settings.settings.autoCheckUpdate,
  set: (value: boolean) => void update({ autoCheckUpdate: value }),
})

const theme = computed({
  get: () => settings.settings.theme,
  set: (value: ThemeMode) => void update({ theme: value }),
})

const localeValue = computed({
  get: () => settings.settings.locale,
  set: (value: string) => void update({ locale: value }),
})

const market = computed({
  get: () => settings.settings.market,
  set: (value: string) => void update({ market: value }),
})

const updateChannel = computed({
  get: () => settings.settings.updateChannel,
  set: (value: UpdateChannel) => void update({ updateChannel: value }),
})

const autoTime = computed({
  get: () => dayjs(settings.settings.autoSetTime || '00:05', 'HH:mm'),
  set: (value: Dayjs | null) => {
    if (value) void update({ autoSetTime: value.format('HH:mm') })
  },
})

const retention = computed({
  get: () => settings.settings.retentionDays,
  set: (value: number | null) => {
    if (typeof value === 'number') void update({ retentionDays: value })
  },
})

const imageDir = computed(() => settings.settings.imageDir ?? '')
const previewImage = computed(() => assetUrl(wallpaper.today?.filePath))
const todayTitle = computed(() => wallpaper.today?.title ?? '')
const previewLoading = computed(() => wallpaper.setting)
const appVersion = computed(() => app.version)
const appPlatform = computed(() => app.platformLabel)
const checkingUpdate = computed(() => app.checkingUpdate)
const installing = computed(() => app.installing)
const updateInfo = computed(() => app.update)
const storageBytes = computed(() => app.storage.bytes)
const storageCount = computed(() => app.storage.count)
const storageFavorites = computed(() => app.storage.favoriteCount)
const previewStyle = computed(() =>
  previewImage.value ? { backgroundImage: `url('${previewImage.value}')` } : {},
)
const autoText = computed(() =>
  settings.settings.autoSet ? settings.settings.autoSetTime : t('home.disabled'),
)

const sizeOptions = IMAGE_SIZE_OPTIONS.map((option) => ({ ...option }))
const styleOptions = computed(() =>
  buildStyleOptions(app.capabilities.styles, (style) => t(styleLabelKey(style))),
)
const themeOptions = computed(() => [
  { value: 'system', label: t('settings.themeSystem') },
  { value: 'light', label: t('settings.themeLight') },
  { value: 'dark', label: t('settings.themeDark') },
])
const localeOptions = SUPPORTED_LOCALES.map((item) => ({ ...item }))
const marketOptions = [
  { value: 'zh-CN', label: '中文（中国）' },
  { value: 'en-US', label: 'English (US)' },
  { value: 'en-GB', label: 'English (UK)' },
  { value: 'ja-JP', label: '日本語' },
  { value: 'de-DE', label: 'Deutsch' },
  { value: 'fr-FR', label: 'Français' },
]
const channelOptions = computed(() => [
  { value: 'stable', label: t('settings.stable') },
  { value: 'beta', label: t('settings.beta') },
])

const saveState = computed(() => {
  if (settings.saving) return t('common.saving')
  if (settings.lastSavedAt) return t('common.saved')
  return ''
})

async function update(patch: SettingsPatch) {
  const next = await settings.save(patch)
  if (!next) message.error(t('settings.saveFailed'))
  return next
}

async function pickFolder() {
  try {
    if (!isTauri) {
      await update({ imageDir: 'D:\\Pictures\\BingWallpaper' })
      message.success(t('settings.dirPicked'))
      return
    }
    const dir = await open({
      directory: true,
      multiple: false,
      title: t('settings.saveDir'),
      defaultPath: settings.settings.imageDir ?? undefined,
    })
    if (typeof dir === 'string') {
      await update({ imageDir: dir })
      message.success(t('settings.dirPicked'))
    }
  } catch (e) {
    message.error(errorText(e))
  }
}

async function clearDir() {
  await update({ imageDir: null })
}

function saveProxy() {
  const value = proxyInput.value.trim()
  void update({ proxy: value || null })
}

async function applyPreview() {
  try {
    await wallpaper.setAsWallpaper(settings.settings.wallpaperStyle)
    message.success(t('home.wallpaperSet'))
  } catch (e) {
    message.error(errorText(e))
  }
}

async function checkUpdate() {
  const info = await app.checkUpdate(false)
  if (info) message.success(`${t('common.install')} v${info.version}`)
  else message.info(t('common.upToDate'))
}

async function installUpdate() {
  try {
    await app.installUpdate()
    message.success(t('common.success'))
  } catch (e) {
    message.error(errorText(e))
  }
}

async function clearCache() {
  clearing.value = true
  try {
    const result = await api.clearCache()
    message.success(
      t('common.cleared', {
        count: result.removed,
        size: formatBytes(result.bytesFreed),
      }),
    )
    await app.refreshStorage()
    if (!wallpaper.today) await wallpaper.load()
  } catch (e) {
    message.error(errorText(e))
  } finally {
    clearing.value = false
  }
}

function resetSettings() {
  Modal.confirm({
    title: t('settings.reset'),
    content: t('settings.resetConfirm'),
    okText: t('common.confirm'),
    cancelText: t('common.cancel'),
    async onOk() {
      const next = await settings.reset()
      if (next) message.success(t('settings.resetDone'))
      else message.error(t('settings.saveFailed'))
    },
  })
}
</script>

<template>
  <div class="page settings-page">
    <header class="page-head">
      <div>
        <h1>{{ t('settings.title') }}</h1>
        <p>{{ t('settings.subtitle') }}</p>
      </div>
      <span v-if="saveState" class="save-state">
        <span class="save-dot" :class="{ saving: settings.saving }"></span>
        {{ saveState }}
      </span>
    </header>

    <div class="settings-layout">
      <div class="settings-main">
        <SettingSection
          :title="t('settings.wallpaper')"
          :description="t('settings.subtitle')"
          :icon="PictureOutlined"
        >
          <SettingRow :label="t('settings.imageSize')">
            <a-select v-model:value="imageSize" :options="sizeOptions" class="w-180" />
          </SettingRow>
          <SettingRow :label="t('settings.style')" :hint="t('settings.styleHint')">
            <a-select v-model:value="wallpaperStyle" :options="styleOptions" class="w-180" />
          </SettingRow>
          <SettingRow :label="t('settings.saveDir')" :hint="t('settings.saveDirHint')">
            <a-input :value="imageDir" readonly class="dir-input" />
            <a-button :title="t('common.browse')" @click="pickFolder">
              <FolderOpenOutlined />
            </a-button>
            <a-button v-if="imageDir" :title="t('common.cancel')" @click="clearDir">
              <CloseOutlined />
            </a-button>
          </SettingRow>
          <SettingRow :label="t('settings.market')" :hint="t('settings.marketHint')">
            <a-select v-model:value="market" :options="marketOptions" class="w-180" />
          </SettingRow>
        </SettingSection>

        <SettingSection
          :title="t('settings.automation')"
          :icon="ThunderboltOutlined"
        >
          <SettingRow :label="t('settings.autoStart')">
            <a-switch v-model:checked="autoStart" />
          </SettingRow>
          <SettingRow :label="t('settings.autoSet')">
            <a-switch v-model:checked="autoSet" />
          </SettingRow>
          <SettingRow :label="t('settings.autoSetTime')">
            <a-time-picker
              v-model:value="autoTime"
              format="HH:mm"
              :minute-step="5"
              :allow-clear="false"
              class="w-140"
            />
          </SettingRow>
          <SettingRow :label="t('settings.catchUp')">
            <a-switch v-model:checked="catchUp" />
          </SettingRow>
          <SettingRow :label="t('settings.notifications')">
            <a-switch v-model:checked="notifications" />
          </SettingRow>
        </SettingSection>

        <SettingSection :title="t('settings.general')" :icon="SettingOutlined">
          <SettingRow :label="t('settings.language')">
            <a-select v-model:value="localeValue" :options="localeOptions" class="w-180" />
          </SettingRow>
          <SettingRow :label="t('settings.theme')">
            <a-select v-model:value="theme" :options="themeOptions" class="w-180" />
          </SettingRow>
          <SettingRow :label="t('settings.closeToTray')">
            <a-switch v-model:checked="closeToTray" />
          </SettingRow>
          <SettingRow :label="t('settings.autoCheckUpdate')">
            <a-switch v-model:checked="autoCheckUpdate" />
          </SettingRow>
          <SettingRow :label="t('settings.updateChannel')">
            <a-select v-model:value="updateChannel" :options="channelOptions" class="w-180" />
          </SettingRow>
          <SettingRow :label="t('settings.proxy')" :hint="t('settings.proxyHint')">
            <a-input
              v-model:value="proxyInput"
              class="proxy-input"
              placeholder="http://127.0.0.1:7890"
              @blur="saveProxy"
              @press-enter="saveProxy"
            />
          </SettingRow>
          <SettingRow :label="t('settings.reset')">
            <a-button danger @click="resetSettings">
              <ReloadOutlined />
              {{ t('settings.reset') }}
            </a-button>
          </SettingRow>
        </SettingSection>
      </div>

      <aside class="settings-side">
        <section class="side-card glass preview-card">
          <h3><PictureOutlined /> {{ t('settings.preview') }}</h3>
          <div class="preview-image" :style="previewStyle">
            <span v-if="!previewImage" class="preview-empty">{{ t('common.noData') }}</span>
            <div v-else class="preview-shade">
              <span>{{ todayTitle }}</span>
            </div>
          </div>
          <a-button type="primary" block :loading="previewLoading" @click="applyPreview">
            {{ t('common.setWallpaper') }}
          </a-button>
        </section>

        <section class="side-card glass">
          <h3><CloudSyncOutlined /> {{ t('common.version') }}</h3>
          <div class="info-row">
            <span>{{ t('common.version') }}</span>
            <strong>v{{ appVersion }}</strong>
          </div>
          <div class="info-row">
            <span>{{ t('common.platform') }}</span>
            <strong>{{ appPlatform }}</strong>
          </div>
          <div class="info-row">
            <span>{{ t('gallery.autoTime') }}</span>
            <strong>{{ autoText }}</strong>
          </div>
          <a-button block :loading="checkingUpdate" @click="checkUpdate">
            <CloudSyncOutlined />
            {{ t('common.checkUpdate') }}
          </a-button>
          <div v-if="updateInfo" class="update-box">
            <div>
              <strong>v{{ updateInfo.version }}</strong>
              <span>{{ t('common.install') }}</span>
            </div>
            <a-button type="primary" size="small" :loading="installing" @click="installUpdate">
              {{ t('common.install') }}
            </a-button>
          </div>
        </section>

        <section class="side-card glass">
          <h3><HddOutlined /> {{ t('settings.storage') }}</h3>
          <div class="info-row">
            <span>{{ t('common.storage') }}</span>
            <strong>{{ formatBytes(storageBytes) }}</strong>
          </div>
          <div class="info-row">
            <span>{{ t('gallery.totalImages') }}</span>
            <strong>{{ storageCount }}</strong>
          </div>
          <div class="info-row">
            <span>{{ t('common.favorites') }}</span>
            <strong>{{ storageFavorites }}</strong>
          </div>
          <div class="retention-row">
            <div>
              <span>{{ t('settings.retention') }}</span>
              <small>{{ t('settings.retentionHint') }}</small>
            </div>
            <a-input-number
              v-model:value="retention"
              :min="1"
              :max="365"
              size="small"
              class="retention-input"
            />
          </div>
          <a-button block :loading="clearing" @click="clearCache">
            <BellOutlined />
            {{ t('common.clearCache') }}
          </a-button>
        </section>
      </aside>
    </div>
  </div>
</template>

<style scoped>
.settings-page {
  display: flex;
  flex-direction: column;
  gap: 18px;
}

.save-state {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  font-size: 11.5px;
  color: var(--bw-muted);
  white-space: nowrap;
}

.save-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--bw-success);
}

.save-dot.saving {
  background: var(--bw-warning);
  animation: blink 1s infinite;
}

@keyframes blink {
  50% {
    opacity: 0.3;
  }
}

.settings-layout {
  display: grid;
  grid-template-columns: minmax(0, 1.45fr) minmax(300px, 0.75fr);
  gap: 16px;
  align-items: start;
}

.settings-main,
.settings-side {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.w-180 {
  width: 180px;
}

.w-140 {
  width: 140px;
}

.dir-input {
  width: 240px;
}

.proxy-input {
  width: 240px;
}

.side-card {
  padding: 16px;
  border-radius: var(--bw-radius);
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.side-card h3 {
  display: flex;
  align-items: center;
  gap: 7px;
  margin: 0 0 2px;
  font-size: 13.5px;
  font-weight: 650;
}

.side-card h3 :deep(.anticon) {
  color: var(--bw-primary);
}

.preview-image {
  position: relative;
  aspect-ratio: 16 / 9;
  border-radius: 12px;
  background:
    linear-gradient(135deg, rgba(22, 119, 255, 0.14), rgba(9, 88, 217, 0.05)),
    var(--bw-bg-deep);
  background-size: cover;
  background-position: center;
  overflow: hidden;
  display: grid;
  place-items: center;
}

.preview-empty {
  font-size: 11.5px;
  color: var(--bw-muted);
}

.preview-shade {
  position: absolute;
  inset: auto 0 0 0;
  padding: 18px 12px 10px;
  background: linear-gradient(to top, rgba(3, 8, 16, 0.72), transparent);
  color: #fff;
  font-size: 11.5px;
  font-weight: 600;
}

.info-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  font-size: 12px;
  color: var(--bw-muted);
}

.info-row strong {
  font-size: 12.5px;
  color: var(--bw-ink);
  font-weight: 650;
  text-align: right;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.update-box {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 10px 12px;
  border-radius: 11px;
  background: var(--bw-primary-soft);
}

.update-box div {
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.update-box strong {
  font-size: 13px;
  color: var(--bw-primary);
}

.update-box span {
  font-size: 10.5px;
  color: var(--bw-muted);
}

.retention-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 4px 0;
}

.retention-row div {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.retention-row span {
  font-size: 12px;
  color: var(--bw-muted);
}

.retention-row small {
  font-size: 10.5px;
  color: var(--bw-muted);
  opacity: 0.8;
}

.retention-input {
  width: 84px;
}

@media (max-width: 1100px) {
  .settings-layout {
    grid-template-columns: minmax(0, 1fr);
  }

  .settings-side {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
  }
}

@media (max-width: 720px) {
  .dir-input,
  .proxy-input {
    width: 100%;
  }
}
</style>
