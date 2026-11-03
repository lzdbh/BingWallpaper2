<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import {
  BlockOutlined,
  BorderOutlined,
  CloseOutlined,
  MinusOutlined,
} from '@ant-design/icons-vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { isTauri } from '@/api/events'
import { useSettingsStore } from '@/stores/settings'

const { t } = useI18n()
const settings = useSettingsStore()
const win = isTauri ? getCurrentWindow() : null
const maximized = ref(false)
let unlisten: (() => void) | undefined

async function syncMaximized() {
  if (win) maximized.value = await win.isMaximized()
}

onMounted(async () => {
  if (!win) return
  await syncMaximized()
  unlisten = await win.onResized(() => void syncMaximized())
})

onBeforeUnmount(() => unlisten?.())

async function minimize() {
  await win?.minimize()
}

async function toggleMaximize() {
  await win?.toggleMaximize()
  await syncMaximized()
}

async function close() {
  if (!win) return
  if (settings.settings.closeToTray) await win.hide()
  else await win.close()
}
</script>

<template>
  <div v-if="isTauri" class="window-controls no-drag">
    <button
      class="control"
      type="button"
      :aria-label="t('window.minimize')"
      :title="t('window.minimize')"
      @click="minimize"
    >
      <MinusOutlined />
    </button>
    <button
      class="control"
      type="button"
      :aria-label="maximized ? t('window.restore') : t('window.maximize')"
      :title="maximized ? t('window.restore') : t('window.maximize')"
      @click="toggleMaximize"
    >
      <BlockOutlined v-if="maximized" />
      <BorderOutlined v-else />
    </button>
    <button
      class="control close"
      type="button"
      :aria-label="t('window.close')"
      :title="t('window.close')"
      @click="close"
    >
      <CloseOutlined />
    </button>
  </div>
</template>

<style scoped>
.window-controls {
  display: flex;
  align-items: stretch;
  height: 100%;
  margin-left: 6px;
}

.control {
  width: 46px;
  height: 100%;
  display: grid;
  place-items: center;
  border: 0;
  background: transparent;
  color: inherit;
  cursor: pointer;
  opacity: 0.72;
  transition:
    background var(--bw-transition),
    opacity var(--bw-transition),
    color var(--bw-transition);
  font-size: 13px;
}

.control:hover {
  opacity: 1;
  background: rgba(127, 127, 127, 0.16);
}

.control.close:hover {
  background: #e5484d;
  color: #fff;
}

.control:focus-visible {
  outline: 2px solid var(--bw-primary);
  outline-offset: -3px;
}
</style>
