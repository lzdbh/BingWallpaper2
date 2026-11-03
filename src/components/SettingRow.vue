<script setup lang="ts">
defineProps<{
  label: string
  hint?: string
  stacked?: boolean
}>()
</script>

<template>
  <div class="setting-row" :class="{ stacked }">
    <div class="row-text">
      <span class="row-label">{{ label }}</span>
      <small v-if="hint" class="row-hint">{{ hint }}</small>
    </div>
    <div class="row-control">
      <slot />
    </div>
  </div>
</template>

<style scoped>
.setting-row {
  display: flex;
  align-items: center;
  gap: 18px;
  padding: 12px 0;
  border-bottom: 1px solid var(--bw-line);
}

.setting-row:last-child {
  border-bottom: 0;
}

.setting-row.stacked {
  flex-direction: column;
  align-items: stretch;
  gap: 8px;
}

.row-text {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
  flex: 1;
}

.row-label {
  font-size: 13.5px;
  font-weight: 500;
  color: var(--bw-ink);
}

.row-hint {
  font-size: 11.5px;
  color: var(--bw-muted);
  line-height: 1.45;
}

.row-control {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
  flex: none;
  max-width: 62%;
}

.stacked .row-control {
  max-width: 100%;
  justify-content: flex-start;
}
</style>
