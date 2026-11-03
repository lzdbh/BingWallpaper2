import { createApp } from 'vue'
import { createPinia } from 'pinia'
import 'ant-design-vue/dist/reset.css'
import './styles/tokens.css'
import './styles/global.css'
import App from './App.vue'
import router from './router'
import i18n from './i18n'

createApp(App).use(createPinia()).use(router).use(i18n).mount('#app')
