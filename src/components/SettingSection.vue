<script setup lang="ts">
import type { Component } from 'vue'

defineProps<{
  title: string
  description?: string
  icon?: Component
}>()
</script>

<template>
  <section class="setting-section glass">
    <header class="section-head">
      <span v-if="icon" class="section-icon">
        <component :is="icon" />
      </span>
      <div class="section-title">
        <h3>{{ title }}</h3>
        <p v-if="description">{{ description }}</p>
      </div>
      <div class="section-actions">
        <slot name="actions" />
      </div>
    </header>
    <div class="section-rows">
      <slot />
    </div>
  </section>
</template>

<style scoped>
.setting-section {
  border-radius: var(--bw-radius);
  padding: 6px 18px 10px;
}

.section-head {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 14px 0 10px;
  border-bottom: 1px solid var(--bw-line);
}

.section-icon {
  display: grid;
  place-items: center;
  width: 30px;
  height: 30px;
  border-radius: 9px;
  background: var(--bw-primary-soft);
  color: var(--bw-primary);
  font-size: 15px;
  flex: none;
}

.section-title {
  min-width: 0;
}

.section-title h3 {
  margin: 0;
  font-size: 14.5px;
  font-weight: 650;
  letter-spacing: 0.01em;
}

.section-title p {
  margin: 2px 0 0;
  font-size: 12px;
  color: var(--bw-muted);
}

.section-actions {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 8px;
}

.section-rows {
  display: flex;
  flex-direction: column;
}
</style>
