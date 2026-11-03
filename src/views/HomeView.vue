<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { message } from 'ant-design-vue'
import {
  ClockCircleOutlined,
  DownloadOutlined,
  HeartFilled,
  HeartOutlined,
  ReloadOutlined,
  ThunderboltOutlined,
} from '@ant-design/icons-vue'
import type { ImageSize, WallpaperStyle } from '@/api/types'
import { useAppStore } from '@/stores/app'
import { useSettingsStore } from '@/stores/settings'
import { useWallpaperStore } from '@/stores/wallpaper'
import {
  IMAGE_SIZE_OPTIONS,
  assetUrl,
  buildStyleOptions,
  formatDate,
  formatWeekday,
  sizeLabel,
  styleLabelKey,
} from '@/utils/format'
import { errorText } from '@/utils/error'

const { t, locale } = useI18n()
const app = useAppStore()
const settings = useSettingsStore()
const wallpaper = useWallpaperStore()

const today = computed(() => wallpaper.today)
const hero = computed(() => assetUrl(today.value?.filePath))
const dateLine = computed(() => {
  if (!today.value) return t('home.eyebrow')
  return `${formatDate(today.value.date, locale.value)} · ${formatWeekday(today.value.date, locale.value)} · ${t('home.eyebrow')}`
})
const autoText = computed(() =>
  settings.settings.autoSet
    ? t('home.autoSetOn', { time: settings.settings.autoSetTime })
    : t('home.autoSetOff'),
)
const storyText = computed(
  () => today.value?.copyright || t('home.storyFallback'),
)

const sizeOptions = IMAGE_SIZE_OPTIONS.map((option) => ({ ...option }))
const styleOptions = computed(() =>
  buildStyleOptions(app.capabilities.styles, (style) => t(styleLabelKey(style))),
)

const imageSize = computed({
  get: () => settings.settings.imageSize,
  set: (value: ImageSize) => void changeSize(value),
})

const wallpaperStyle = computed({
  get: () => settings.settings.wallpaperStyle,
  set: (value: WallpaperStyle) => void updateStyle(value),
})

onMounted(() => {
  if (!today.value && !wallpaper.loading) void refresh()
})

async function refresh() {
  const image = await wallpaper.load(true)
  if (image) message.success(t('home.refreshed'))
  else if (wallpaper.error) message.error(wallpaper.error.message)
}

async function changeSize(value: ImageSize) {
  if (value === settings.settings.imageSize) return
  await settings.save({ imageSize: value })
  await refresh()
}

async function updateStyle(value: WallpaperStyle) {
  await settings.save({ wallpaperStyle: value })
}

async function applyWallpaper() {
  try {
    await wallpaper.setAsWallpaper(settings.settings.wallpaperStyle)
    message.success(t('home.wallpaperSet'))
  } catch (e) {
    message.error(errorText(e))
  }
}

async function toggleFavorite() {
  try {
    const added = await wallpaper.toggleFavorite()
    if (added) message.success(t('home.favoriteAdded'))
    else message.success(t('home.favoriteRemoved'))
  } catch (e) {
    message.error(errorText(e))
  }
}

async function downloadToday() {
  const image = await wallpaper.downloadToday(true)
  if (image) {
    message.success(t('home.downloaded'))
    void app.refreshStorage()
  } else if (wallpaper.error) {
    message.error(wallpaper.error.message)
  }
}
</script>

<template>
  <div class="home">
    <div
      v-if="today"
      class="hero"
      :style="{ backgroundImage: `url('${hero}')` }"
    >
      <div class="scrim"></div>

      <div class="hero-content">
        <div class="hero-main">
          <div class="eyebrow">
            <span class="pulse"></span>
            <span>{{ dateLine }}</span>
          </div>
          <h1 class="hero-title">{{ today.title }}</h1>
          <p class="copyright">{{ today.copyright }}</p>

          <div class="chips">
            <span class="chip">{{ sizeLabel(settings.settings.imageSize) }}</span>
            <button type="button" class="chip clickable" @click="toggleFavorite">
              <HeartFilled v-if="today.favorite" class="heart-on" />
              <HeartOutlined v-else />
              {{ today.favorite ? t('common.favorites') : t('common.favorite') }}
            </button>
            <button type="button" class="chip clickable" @click="downloadToday">
              <DownloadOutlined />
              {{ t('common.download') }}
            </button>
            <span class="chip">{{ t('home.fromBing') }}</span>
          </div>
        </div>

        <div class="dock">
          <div class="story glass">
            <span class="story-label">{{ t('home.story') }}</span>
            <p>{{ storyText }}</p>
            <div class="story-meta">
              <ClockCircleOutlined />
              <span>{{ autoText }}</span>
            </div>
          </div>

          <div class="control glass-strong">
            <label class="field">
              <span>{{ t('home.imageSize') }}</span>
              <a-select
                v-model:value="imageSize"
                :options="sizeOptions"
                class="field-select"
                :disabled="wallpaper.loading"
              />
            </label>

            <label class="field">
              <span>{{ t('home.style') }}</span>
              <a-select
                v-model:value="wallpaperStyle"
                :options="styleOptions"
                class="field-select"
              />
            </label>

            <a-button
              class="ghost-btn"
              :loading="wallpaper.loading"
              @click="refresh"
            >
              <ReloadOutlined />
              {{ t('common.refresh') }}
            </a-button>

            <a-button
              type="primary"
              size="large"
              class="apply-btn"
              :loading="wallpaper.setting"
              @click="applyWallpaper"
            >
              <ThunderboltOutlined />
              {{ t('common.setWallpaper') }}
            </a-button>
          </div>
        </div>
      </div>
    </div>

    <div v-else class="home-state">
      <a-spin v-if="wallpaper.loading" size="large" />
      <template v-else>
        <p>{{ wallpaper.error?.message || t('common.noData') }}</p>
        <a-button type="primary" @click="refresh">{{ t('common.retry') }}</a-button>
      </template>
    </div>
  </div>
</template>

<style scoped>
.home {
  position: relative;
  width: 100%;
  height: 100%;
  min-height: 0;
  background: #0a1220;
}

.hero {
  position: relative;
  width: 100%;
  height: 100%;
  background-size: cover;
  background-position: center;
  background-repeat: no-repeat;
  overflow: hidden;
}

.hero::before {
  content: '';
  position: absolute;
  inset: 0;
  background: inherit;
  filter: saturate(1.04);
  transform: scale(1.01);
}

.scrim {
  position: absolute;
  inset: 0;
  background:
    linear-gradient(
      100deg,
      rgba(3, 8, 16, 0.76) 0%,
      rgba(3, 8, 16, 0.38) 38%,
      rgba(3, 8, 16, 0.04) 68%,
      rgba(3, 8, 16, 0.22) 100%
    ),
    linear-gradient(to top, rgba(3, 8, 16, 0.78) 0%, rgba(3, 8, 16, 0) 46%);
}

.hero-content {
  position: relative;
  z-index: 2;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  gap: 24px;
  width: 100%;
  height: 100%;
  padding: calc(var(--bw-titlebar-height) + 26px) 46px 34px;
  color: #fff;
}

.hero-main {
  max-width: 720px;
  padding-top: 6vh;
}

.eyebrow {
  display: inline-flex;
  align-items: center;
  gap: 9px;
  padding: 6px 13px;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.13);
  border: 1px solid rgba(255, 255, 255, 0.18);
  backdrop-filter: blur(10px);
  font-size: 11.5px;
  font-weight: 500;
  letter-spacing: 0.03em;
  color: rgba(255, 255, 255, 0.9);
}

.pulse {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: #52e5a3;
  box-shadow: 0 0 0 0 rgba(82, 229, 163, 0.55);
  animation: pulse 2.2s infinite;
}

@keyframes pulse {
  0% {
    box-shadow: 0 0 0 0 rgba(82, 229, 163, 0.55);
  }
  70% {
    box-shadow: 0 0 0 9px rgba(82, 229, 163, 0);
  }
  100% {
    box-shadow: 0 0 0 0 rgba(82, 229, 163, 0);
  }
}

.hero-title {
  margin: 18px 0 10px;
  max-width: 16ch;
  font-size: clamp(38px, 5.6vw, 74px);
  line-height: 1.02;
  font-weight: 760;
  letter-spacing: -0.035em;
  text-wrap: balance;
  text-shadow: 0 8px 40px rgba(0, 0, 0, 0.38);
}

.copyright {
  margin: 0;
  font-size: 13.5px;
  line-height: 1.6;
  color: rgba(255, 255, 255, 0.76);
  max-width: 60ch;
  text-shadow: 0 2px 14px rgba(0, 0, 0, 0.4);
}

.chips {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  margin-top: 22px;
}

.chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  border: 1px solid rgba(255, 255, 255, 0.2);
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.12);
  backdrop-filter: blur(10px);
  color: rgba(255, 255, 255, 0.92);
  font-size: 11.5px;
  font-weight: 500;
  letter-spacing: 0.01em;
}

.chip.clickable {
  cursor: pointer;
  transition:
    background var(--bw-transition),
    border-color var(--bw-transition),
    transform var(--bw-transition);
}

.chip.clickable:hover {
  background: rgba(255, 255, 255, 0.22);
  border-color: rgba(255, 255, 255, 0.34);
}

.chip.clickable:active {
  transform: scale(0.97);
}

.heart-on {
  color: #ff7d92;
}

.dock {
  display: grid;
  grid-template-columns: minmax(0, 420px) auto;
  align-items: end;
  justify-content: space-between;
  gap: 18px;
}

.story {
  padding: 16px 18px;
  border-radius: var(--bw-radius);
  background: rgba(8, 15, 26, 0.46);
  border-color: rgba(255, 255, 255, 0.16);
  box-shadow: 0 18px 40px rgba(0, 0, 0, 0.24);
  color: #fff;
}

.story-label {
  display: block;
  margin-bottom: 6px;
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.16em;
  text-transform: uppercase;
  color: rgba(255, 255, 255, 0.58);
}

.story p {
  margin: 0;
  font-size: 12.5px;
  line-height: 1.65;
  color: rgba(255, 255, 255, 0.86);
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.story-meta {
  display: flex;
  align-items: center;
  gap: 7px;
  margin-top: 10px;
  font-size: 11px;
  color: rgba(255, 255, 255, 0.62);
}

.control {
  display: flex;
  align-items: flex-end;
  gap: 12px;
  padding: 14px 16px;
  border-radius: var(--bw-radius);
  background: rgba(10, 18, 30, 0.6);
  border-color: rgba(255, 255, 255, 0.18);
  box-shadow: 0 20px 46px rgba(0, 0, 0, 0.3);
  color: #fff;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 5px;
}

.field > span {
  font-size: 10px;
  font-weight: 600;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: rgba(255, 255, 255, 0.56);
}

.field-select {
  width: 132px;
}

.control :deep(.ant-select-selector) {
  background: rgba(255, 255, 255, 0.12) !important;
  border-color: rgba(255, 255, 255, 0.18) !important;
  color: #fff !important;
  backdrop-filter: blur(8px);
}

.control :deep(.ant-select-selection-item) {
  color: #fff !important;
}

.control :deep(.ant-select-arrow) {
  color: rgba(255, 255, 255, 0.72);
}

.ghost-btn {
  background: rgba(255, 255, 255, 0.1);
  border-color: rgba(255, 255, 255, 0.22);
  color: #fff;
}

.ghost-btn:hover {
  background: rgba(255, 255, 255, 0.2) !important;
  border-color: rgba(255, 255, 255, 0.34) !important;
  color: #fff !important;
}

.apply-btn {
  min-width: 132px;
}

.home-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 14px;
  height: 100%;
  color: rgba(255, 255, 255, 0.8);
  font-size: 13px;
}

.home-state p {
  margin: 0;
}

@media (max-width: 1100px) {
  .dock {
    grid-template-columns: minmax(0, 1fr);
  }

  .story {
    display: none;
  }

  .control {
    justify-self: stretch;
    flex-wrap: wrap;
  }
}

@media (max-width: 760px) {
  .hero-content {
    padding: calc(var(--bw-titlebar-height) + 16px) 22px 22px;
  }

  .hero-main {
    padding-top: 2vh;
  }

  .field-select {
    width: 116px;
  }

  .apply-btn {
    flex: 1;
  }
}
</style>
