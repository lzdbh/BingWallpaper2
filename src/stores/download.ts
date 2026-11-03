import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import { api } from '@/api'
import { onEvent } from '@/api/events'
import type { DownloadFinished, DownloadProgress } from '@/api/types'
import { formatDateTime } from '@/utils/format'
import { errorText } from '@/utils/error'
import i18n from '@/i18n'
import { useAppStore } from './app'

export interface LogEntry {
  id: number
  time: string
  level: 'info' | 'success' | 'warn' | 'error'
  text: string
}

export const useDownloadStore = defineStore('download', () => {
  const taskId = ref<string | null>(null)
  const running = ref(false)
  const progress = ref<DownloadProgress | null>(null)
  const result = ref<DownloadFinished | null>(null)
  const logs = ref<LogEntry[]>([])
  const error = ref<string | null>(null)
  let logId = 0
  let initialized = false

  const percent = computed(() => progress.value?.percent ?? 0)
  const isDone = computed(() => !running.value && result.value !== null)

  function pushLog(text: string, level: LogEntry['level'] = 'info') {
    logs.value.push({ id: ++logId, time: formatDateTime(), level, text })
    if (logs.value.length > 300) logs.value.splice(0, logs.value.length - 300)
  }

  async function init() {
    if (initialized) return
    initialized = true
    await onEvent<DownloadProgress>('download://progress', (payload) => {
      if (taskId.value && payload.taskId !== taskId.value) return
      progress.value = payload
      pushLog(
        i18n.global.t('downloads.logProgress', {
          current: payload.current,
          total: payload.total,
          message: payload.message,
        }),
        'info',
      )
    })
    await onEvent<DownloadFinished>('download://finished', (payload) => {
      if (taskId.value && payload.taskId !== taskId.value) return
      running.value = false
      result.value = payload
      pushLog(
        i18n.global.t('downloads.logFinished', {
          success: payload.success,
          failed: payload.failed,
          skipped: payload.skipped,
        }),
        payload.failed > 0 ? 'warn' : 'success',
      )
      void useAppStore().refreshStorage()
    })
  }

  async function start(startDate: string, endDate: string) {
    if (running.value) return
    error.value = null
    result.value = null
    progress.value = null
    logs.value = []
    running.value = true
    try {
      const id = await api.downloadRange(startDate, endDate)
      taskId.value = id
      pushLog(i18n.global.t('downloads.logStarted', { id }), 'info')
    } catch (e) {
      running.value = false
      error.value = errorText(e)
      pushLog(error.value, 'error')
    }
  }

  async function cancel() {
    if (!taskId.value || !running.value) return
    try {
      const cancelled = await api.cancelDownload(taskId.value)
      if (cancelled) {
        running.value = false
        pushLog(i18n.global.t('downloads.cancelled'), 'warn')
      }
    } catch (e) {
      error.value = errorText(e)
    }
  }

  function clear() {
    if (running.value) return
    logs.value = []
    progress.value = null
    result.value = null
    taskId.value = null
    error.value = null
  }

  return {
    taskId,
    running,
    progress,
    result,
    logs,
    error,
    percent,
    isDone,
    init,
    start,
    cancel,
    clear,
  }
})
