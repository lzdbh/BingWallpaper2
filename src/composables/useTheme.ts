import { computed, ref, watchEffect } from 'vue'
import { useSettingsStore } from '@/stores/settings'

const media = window.matchMedia('(prefers-color-scheme: dark)')
const systemDark = ref(media.matches)

media.addEventListener('change', (event) => {
  systemDark.value = event.matches
})

export function useTheme() {
  const settings = useSettingsStore()
  const isDark = computed(() => {
    const mode = settings.settings.theme
    return mode === 'dark' || (mode === 'system' && systemDark.value)
  })

  watchEffect(() => {
    document.documentElement.dataset.theme = isDark.value ? 'dark' : 'light'
    document.documentElement.style.colorScheme = isDark.value ? 'dark' : 'light'
  })

  return { isDark, systemDark }
}
