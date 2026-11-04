<script setup lang="ts">
import type { Component } from 'vue'
import { InboxOutlined } from '@ant-design/icons-vue'

defineProps<{
  title: string
  hint?: string
  icon?: Component
}>()
</script>

<template>
  <div class="empty-state">
    <div class="empty-icon">
      <component :is="icon ?? InboxOutlined" />
    </div>
    <h3>{{ title }}</h3>
    <p v-if="hint">{{ hint }}</p>
    <div class="empty-action">
      <slot name="action" />
    </div>
  </div>
</template>

<style scoped>
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 54px 24px;
  text-align: center;
  border: 1px dashed var(--bw-line-strong);
  border-radius: var(--bw-radius);
  background: var(--bw-surface-muted);
}

.empty-icon {
  display: grid;
  place-items: center;
  width: 54px;
  height: 54px;
  margin-bottom: 4px;
  border-radius: 16px;
  background: var(--bw-primary-soft);
  color: var(--bw-primary);
  font-size: 25px;
}

.empty-state h3 {
  margin: 0;
  font-size: 15px;
  font-weight: 650;
}

.empty-state p {
  margin: 0;
  max-width: 420px;
  font-size: 12.5px;
  line-height: 1.6;
  color: var(--bw-muted);
}

.empty-action {
  margin-top: 10px;
}
</style>
