import { createI18n } from 'vue-i18n'
import zhCN from './locales/zh-CN.json'
import enUS from './locales/en-US.json'

export type AppLocale = 'zh-CN' | 'en-US'

export const SUPPORTED_LOCALES = [
  { value: 'zh-CN', label: '简体中文' },
  { value: 'en-US', label: 'English' },
] as const

export function detectLocale(): AppLocale {
  const saved = localStorage.getItem('bingwallpaper.locale')
  if (saved && SUPPORTED_LOCALES.some((l) => l.value === saved)) return saved as AppLocale
  return navigator.language.toLowerCase().startsWith('zh') ? 'zh-CN' : 'en-US'
}

const i18n = createI18n({
  legacy: false,
  locale: detectLocale(),
  fallbackLocale: 'en-US',
  messages: {
    'zh-CN': zhCN,
    'en-US': enUS,
  },
  missingWarn: false,
  fallbackWarn: false,
})

export function setI18nLocale(locale: string) {
  const value: AppLocale = SUPPORTED_LOCALES.some((l) => l.value === locale)
    ? (locale as AppLocale)
    : 'en-US'
  i18n.global.locale.value = value
  document.documentElement.lang = value
  localStorage.setItem('bingwallpaper.locale', value)
}

export default i18n
