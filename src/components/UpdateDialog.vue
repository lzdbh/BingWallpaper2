<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useAppStore } from '@/stores/app'
import { formatDate } from '@/utils/format'

const props = defineProps<{ open: boolean }>()
const emit = defineEmits<{ (e: 'update:open', value: boolean): void }>()
const { t, locale } = useI18n()
const app = useAppStore()

const visible = computed({
  get: () => props.open,
  set: (value: boolean) => emit('update:open', value),
})

async function install() {
  await app.installUpdate()
  visible.value = false
}
</script>

<template>
  <a-modal
    v-model:open="visible"
    :title="t('common.install')"
    :ok-text="t('common.install')"
    :cancel-text="t('common.later')"
    :confirm-loading="app.installing"
    @ok="install"
  >
    <div v-if="app.update" class="update-body">
      <div class="version-line">
        <span class="badge">v{{ app.update.version }}</span>
        <span class="current">{{ t('common.version') }} v{{ app.update.currentVersion }}</span>
      </div>
      <p v-if="app.update.publishedAt" class="published">
        {{ t('common.lastUpdated') }}：{{ formatDate(app.update.publishedAt, locale) }}
      </p>
      <pre class="notes">{{ app.update.notes || t('common.noData') }}</pre>
    </div>
    <a-empty v-else />
  </a-modal>
</template>

<style scoped>
.update-body {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.version-line {
  display: flex;
  align-items: center;
  gap: 10px;
}

.badge {
  padding: 3px 10px;
  border-radius: 999px;
  background: var(--bw-primary-soft);
  color: var(--bw-primary);
  font-weight: 700;
  font-size: 12px;
}

.current,
.published {
  font-size: 12px;
  color: var(--bw-muted);
}

.notes {
  margin: 6px 0 0;
  max-height: 240px;
  overflow: auto;
  padding: 12px;
  border-radius: 10px;
  background: rgba(127, 127, 127, 0.08);
  font-family: inherit;
  font-size: 12px;
  line-height: 1.65;
  white-space: pre-wrap;
}
</style>
