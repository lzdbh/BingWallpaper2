<script setup lang="ts">
import { nextTick, ref, watch } from 'vue'
import type { LogEntry } from '@/stores/download'

const props = defineProps<{
  entries: LogEntry[]
  placeholder?: string
}>()

const scroller = ref<HTMLElement | null>(null)

watch(
  () => props.entries.length,
  async () => {
    await nextTick()
    if (scroller.value) scroller.value.scrollTop = scroller.value.scrollHeight
  },
)
</script>

<template>
  <div ref="scroller" class="log-console">
    <div v-if="!entries.length" class="placeholder">{{ placeholder }}</div>
    <div v-for="entry in entries" :key="entry.id" class="line" :class="entry.level">
      <span class="time">{{ entry.time }}</span>
      <span class="text">{{ entry.text }}</span>
    </div>
  </div>
</template>

<style scoped>
.log-console {
  height: 100%;
  min-height: 180px;
  max-height: 320px;
  overflow: auto;
  padding: 14px 16px;
  border-radius: 12px;
  background: #0b1220;
  color: #d6e2f5;
  font-family: 'Cascadia Code', 'SFMono-Regular', Consolas, 'Liberation Mono', monospace;
  font-size: 11.5px;
  line-height: 1.7;
}

.placeholder {
  color: #6b7c96;
}

.line {
  display: flex;
  gap: 10px;
  white-space: pre-wrap;
  word-break: break-word;
}

.time {
  flex: none;
  color: #61748f;
  font-variant-numeric: tabular-nums;
}

.text {
  min-width: 0;
}

.line.success .text {
  color: #7ee2a8;
}

.line.warn .text {
  color: #f7c66b;
}

.line.error .text {
  color: #ff8f8f;
}
</style>
