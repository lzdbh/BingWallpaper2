<script setup lang="ts">
import { computed, ref } from 'vue'
import { useI18n } from 'vue-i18n'
import { message } from 'ant-design-vue'
import dayjs, { type Dayjs } from 'dayjs'
import {
  CalendarOutlined,
  CheckCircleOutlined,
  CloudDownloadOutlined,
  CloseCircleOutlined,
  DeleteOutlined,
  DownloadOutlined,
  FieldTimeOutlined,
  StopOutlined,
} from '@ant-design/icons-vue'
import { useDownloadStore } from '@/stores/download'
import { formatDateTime } from '@/utils/format'
import LogConsole from '@/components/LogConsole.vue'

const { t } = useI18n()
const downloads = useDownloadStore()

const range = ref<[Dayjs, Dayjs] | undefined>([dayjs().subtract(6, 'day'), dayjs()])

const progressStatus = computed(() => {
  if (downloads.result) return downloads.result.failed > 0 ? 'exception' : 'success'
  if (downloads.running) return 'active'
  return 'normal'
})

const statusText = computed(() => {
  if (downloads.running) return t('downloads.running')
  if (downloads.result) return downloads.result.message || t('downloads.finished')
  return t('downloads.idle')
})

function setLast7() {
  range.value = [dayjs().subtract(6, 'day'), dayjs()]
}

function setLast30() {
  range.value = [dayjs().subtract(29, 'day'), dayjs()]
}

function setThisMonth() {
  range.value = [dayjs().startOf('month'), dayjs()]
}

async function start() {
  if (!range.value?.[0] || !range.value?.[1]) {
    message.warning(t('downloads.rangeRequired'))
    return
  }
  await downloads.start(
    range.value[0].format('YYYY-MM-DD'),
    range.value[1].format('YYYY-MM-DD'),
  )
  if (downloads.error) message.error(downloads.error)
}

async function cancel() {
  await downloads.cancel()
}
</script>

<template>
  <div class="page downloads-page">
    <header class="page-head">
      <div>
        <h1>{{ t('downloads.title') }}</h1>
        <p>{{ t('downloads.subtitle') }}</p>
      </div>
      <span class="status-pill" :class="{ active: downloads.running }">
        <span class="status-dot"></span>
        {{ statusText }}
      </span>
    </header>

    <div class="download-layout">
      <div class="download-main">
        <section class="panel glass">
          <div class="panel-head">
            <h3><CalendarOutlined /> {{ t('downloads.range') }}</h3>
          </div>

          <a-range-picker
            v-model:value="range"
            class="range-picker"
            format="YYYY-MM-DD"
            :allow-clear="false"
            :disabled="downloads.running"
          />

          <div class="quick-row">
            <span class="quick-label">{{ t('downloads.quick') }}</span>
            <a-button size="small" :disabled="downloads.running" @click="setLast7">
              {{ t('downloads.last7') }}
            </a-button>
            <a-button size="small" :disabled="downloads.running" @click="setLast30">
              {{ t('downloads.last30') }}
            </a-button>
            <a-button size="small" :disabled="downloads.running" @click="setThisMonth">
              {{ t('downloads.thisMonth') }}
            </a-button>
          </div>

          <div class="form-actions">
            <a-button
              type="primary"
              size="large"
              :loading="downloads.running"
              @click="start"
            >
              <DownloadOutlined />
              {{ t('downloads.startTask') }}
            </a-button>
            <a-button
              v-if="downloads.running"
              danger
              size="large"
              @click="cancel"
            >
              <StopOutlined />
              {{ t('downloads.cancelTask') }}
            </a-button>
          </div>
        </section>

        <section class="panel glass">
          <div class="panel-head">
            <h3><FieldTimeOutlined /> {{ t('downloads.progress') }}</h3>
            <span class="progress-message">{{ downloads.progress?.message || statusText }}</span>
          </div>

          <a-progress
            :percent="downloads.percent"
            :status="progressStatus"
            :stroke-width="10"
          />

          <div class="progress-meta">
            <span v-if="downloads.progress">
              {{ downloads.progress.current }} / {{ downloads.progress.total }}
            </span>
            <span v-else>{{ t('downloads.idle') }}</span>
            <span v-if="downloads.progress">
              {{ downloads.progress.percent }}%
            </span>
          </div>

          <div v-if="downloads.result" class="result-grid">
            <div class="result success">
              <CheckCircleOutlined />
              <span>{{ t('downloads.success') }}</span>
              <strong>{{ downloads.result.success }}</strong>
            </div>
            <div class="result failed">
              <CloseCircleOutlined />
              <span>{{ t('downloads.failed') }}</span>
              <strong>{{ downloads.result.failed }}</strong>
            </div>
            <div class="result skipped">
              <DeleteOutlined />
              <span>{{ t('downloads.skipped') }}</span>
              <strong>{{ downloads.result.skipped }}</strong>
            </div>
          </div>

          <p v-else-if="!downloads.running" class="idle-hint">
            {{ t('downloads.idleHint') }}
          </p>
        </section>
      </div>

      <section class="panel glass log-panel">
        <div class="panel-head">
          <h3><CloudDownloadOutlined /> {{ t('downloads.log') }}</h3>
          <div class="log-actions">
            <span v-if="downloads.logs.length" class="log-time">
              {{ formatDateTime() }}
            </span>
            <a-button size="small" :disabled="downloads.running" @click="downloads.clear">
              {{ t('common.clear') }}
            </a-button>
          </div>
        </div>
        <LogConsole
          :entries="downloads.logs"
          :placeholder="t('downloads.idleHint')"
        />
      </section>
    </div>
  </div>
</template>

<style scoped>
.downloads-page {
  display: flex;
  flex-direction: column;
  gap: 18px;
}

.status-pill {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  padding: 7px 12px;
  border-radius: 999px;
  border: 1px solid var(--bw-line);
  background: var(--bw-surface);
  font-size: 11.5px;
  color: var(--bw-muted);
  white-space: nowrap;
}

.status-pill.active {
  color: var(--bw-primary);
  border-color: rgba(22, 119, 255, 0.3);
  background: var(--bw-primary-soft);
}

.status-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: currentColor;
  opacity: 0.7;
}

.status-pill.active .status-dot {
  animation: blink 1.2s infinite;
}

@keyframes blink {
  50% {
    opacity: 0.2;
  }
}

.download-layout {
  display: grid;
  grid-template-columns: minmax(0, 1.15fr) minmax(320px, 0.85fr);
  gap: 16px;
  align-items: start;
}

.download-main {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.panel {
  padding: 16px 18px;
  border-radius: var(--bw-radius);
}

.panel-head {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 14px;
}

.panel-head h3 {
  display: flex;
  align-items: center;
  gap: 7px;
  margin: 0;
  font-size: 13.5px;
  font-weight: 650;
}

.panel-head h3 :deep(.anticon) {
  color: var(--bw-primary);
}

.progress-message {
  margin-left: auto;
  font-size: 11.5px;
  color: var(--bw-muted);
  max-width: 55%;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}

.range-picker {
  width: 100%;
}

.quick-row {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  margin-top: 12px;
}

.quick-label {
  font-size: 11.5px;
  color: var(--bw-muted);
  margin-right: 2px;
}

.form-actions {
  display: flex;
  gap: 10px;
  margin-top: 18px;
}

.form-actions :deep(.ant-btn) {
  min-width: 130px;
}

.progress-meta {
  display: flex;
  justify-content: space-between;
  margin-top: 4px;
  font-size: 11.5px;
  color: var(--bw-muted);
  font-variant-numeric: tabular-nums;
}

.idle-hint {
  margin: 12px 0 0;
  font-size: 12px;
  line-height: 1.6;
  color: var(--bw-muted);
}

.result-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 10px;
  margin-top: 14px;
}

.result {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 10px 12px;
  border-radius: 11px;
  background: rgba(127, 127, 127, 0.08);
  font-size: 11.5px;
  color: var(--bw-muted);
}

.result strong {
  margin-left: auto;
  font-size: 15px;
  color: var(--bw-ink);
}

.result.success :deep(.anticon) {
  color: var(--bw-success);
}

.result.failed :deep(.anticon) {
  color: var(--bw-danger);
}

.result.skipped :deep(.anticon) {
  color: var(--bw-warning);
}

.log-panel {
  display: flex;
  flex-direction: column;
  min-height: 380px;
}

.log-panel :deep(.log-console) {
  flex: 1;
  max-height: none;
}

.log-actions {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 10px;
}

.log-time {
  font-size: 10.5px;
  color: var(--bw-muted);
  font-variant-numeric: tabular-nums;
}

@media (max-width: 1080px) {
  .download-layout {
    grid-template-columns: minmax(0, 1fr);
  }

  .log-panel {
    min-height: 280px;
  }
}

@media (max-width: 620px) {
  .result-grid {
    grid-template-columns: minmax(0, 1fr);
  }
}
</style>
