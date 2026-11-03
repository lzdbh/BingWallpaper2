<script setup lang="ts">
import { computed, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { message, theme as antTheme } from 'ant-design-vue'
import zhCN from 'ant-design-vue/es/locale/zh_CN'
import enUS from 'ant-design-vue/es/locale/en_US'
import dayjs from 'dayjs'
import 'dayjs/locale/zh-cn'
import AppShell from '@/layouts/AppShell.vue'
import { useAppStore } from '@/stores/app'
import { useTheme } from '@/composables/useTheme'
import { errorText } from '@/utils/error'

const { locale } = useI18n()
const app = useAppStore()
const { isDark } = useTheme()

const antLocale = computed(() => (locale.value === 'zh-CN' ? zhCN : enUS))

const themeConfig = computed(() => ({
  algorithm: isDark.value ? antTheme.darkAlgorithm : antTheme.defaultAlgorithm,
  token: {
    colorPrimary: '#1677ff',
    colorInfo: '#1677ff',
    borderRadius: 10,
    fontFamily:
      "'Segoe UI Variable', 'Segoe UI', 'Microsoft YaHei UI', 'PingFang SC', 'Noto Sans CJK SC', 'Noto Sans', system-ui, sans-serif",
  },
  components: {
    Button: { controlHeight: 38 },
    Card: { borderRadiusLG: 16 },
    Modal: { borderRadiusLG: 20 },
  },
}))

watch(
  locale,
  (value) => {
    dayjs.locale(value === 'zh-CN' ? 'zh-cn' : 'en')
  },
  { immediate: true },
)

onMounted(async () => {
  try {
    await app.bootstrap()
  } catch (e) {
    message.error(errorText(e))
  }
})
</script>

<template>
  <a-config-provider :theme="themeConfig" :locale="antLocale">
    <AppShell />
  </a-config-provider>
</template>
