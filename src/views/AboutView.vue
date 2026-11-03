<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { message } from 'ant-design-vue'
import {
  BugOutlined,
  CodeOutlined,
  GlobalOutlined,
  LinkOutlined,
  LockOutlined,
  SafetyCertificateOutlined,
  WindowsOutlined,
} from '@ant-design/icons-vue'
import { api } from '@/api'
import { useAppStore } from '@/stores/app'
import { errorText } from '@/utils/error'
import BrandMark from '@/components/BrandMark.vue'

const { t } = useI18n()
const app = useAppStore()

const GITHUB_URL = 'https://github.com/lzdbh/BingWallpaper2'

const tech = [
  { name: 'Rust', detail: '1.99' },
  { name: 'Tauri', detail: '2.12' },
  { name: 'Vue', detail: '3.5' },
  { name: 'Ant Design Vue', detail: '4.2' },
  { name: 'Pinia', detail: '4.1' },
  { name: 'SQLite', detail: 'rusqlite' },
  { name: 'reqwest', detail: 'rustls' },
  { name: 'image', detail: '0.25' },
]

const platforms = [
  { name: 'Windows', detail: '10 / 11 · x64 · ARM64' },
  { name: 'macOS', detail: '10.15+ · Intel · Apple Silicon' },
  { name: 'Linux', detail: 'x64 · ARM64 · GNOME / KDE / XFCE / Sway' },
]

const links = [
  { label: t('about.github'), url: GITHUB_URL, icon: GlobalOutlined },
  { label: t('about.issues'), url: `${GITHUB_URL}/issues`, icon: BugOutlined },
  { label: 'AGPL-3.0', url: 'https://www.gnu.org/licenses/agpl-3.0.html', icon: LinkOutlined },
]

async function openLink(url: string) {
  try {
    await api.openExternal(url)
  } catch (e) {
    message.error(errorText(e))
  }
}
</script>

<template>
  <div class="page about-page">
    <header class="page-head">
      <div>
        <h1>{{ t('about.title') }}</h1>
        <p>{{ t('about.subtitle') }}</p>
      </div>
    </header>

    <section class="about-hero glass">
      <BrandMark class="hero-logo" style="width: 76px; height: 76px" />
      <div class="hero-copy">
        <h2>{{ t('app.name') }}</h2>
        <p class="hero-meta">v{{ app.version }} · {{ app.platformLabel }}</p>
        <p class="hero-intro">{{ t('about.intro') }}</p>
        <div class="hero-tags">
          <span>Rust</span>
          <span>Tauri</span>
          <span>Vue 3</span>
          <span>Ant Design Vue</span>
          <span>Pinia</span>
        </div>
      </div>
    </section>

    <div class="about-grid">
      <section class="about-card glass">
        <h3><CodeOutlined /> {{ t('about.tech') }}</h3>
        <div class="tech-grid">
          <div v-for="item in tech" :key="item.name" class="tech-item">
            <strong>{{ item.name }}</strong>
            <span>{{ item.detail }}</span>
          </div>
        </div>
      </section>

      <section class="about-card glass">
        <h3><WindowsOutlined /> {{ t('about.platforms') }}</h3>
        <div class="platform-list">
          <div v-for="item in platforms" :key="item.name" class="platform-item">
            <strong>{{ item.name }}</strong>
            <span>{{ item.detail }}</span>
          </div>
        </div>
      </section>

      <section class="about-card glass">
        <h3><SafetyCertificateOutlined /> {{ t('about.license') }}</h3>
        <p>{{ t('about.licenseText') }}</p>
        <a-button size="small" @click="openLink('https://www.gnu.org/licenses/agpl-3.0.html')">
          <LinkOutlined />
          AGPL-3.0
        </a-button>
      </section>

      <section class="about-card glass">
        <h3><LockOutlined /> {{ t('about.privacy') }}</h3>
        <p>{{ t('about.privacyText') }}</p>
      </section>

      <section class="about-card glass links-card">
        <h3><LinkOutlined /> {{ t('about.links') }}</h3>
        <div class="link-list">
          <button
            v-for="item in links"
            :key="item.url"
            type="button"
            class="link-item"
            @click="openLink(item.url)"
          >
            <component :is="item.icon" />
            <span>{{ item.label }}</span>
            <LinkOutlined class="arrow" />
          </button>
        </div>
      </section>
    </div>

    <footer class="about-footer">
      <span>{{ t('about.builtWith') }}</span>
      <span class="dot">·</span>
      <span>{{ t('about.copyright') }}</span>
    </footer>
  </div>
</template>

<style scoped>
.about-page {
  display: flex;
  flex-direction: column;
  gap: 18px;
}

.about-hero {
  display: flex;
  align-items: center;
  gap: 22px;
  padding: 26px 28px;
  border-radius: var(--bw-radius-lg);
  background:
    radial-gradient(circle at 88% 12%, rgba(22, 119, 255, 0.16), transparent 42%),
    var(--bw-surface);
}

.hero-logo {
  flex: none;
  filter: drop-shadow(0 14px 28px rgba(22, 119, 255, 0.34));
}

.hero-copy {
  min-width: 0;
}

.hero-copy h2 {
  margin: 0;
  font-size: 24px;
  font-weight: 750;
  letter-spacing: -0.02em;
}

.hero-meta {
  margin: 4px 0 0;
  font-size: 12px;
  color: var(--bw-muted);
  font-variant-numeric: tabular-nums;
}

.hero-intro {
  margin: 10px 0 0;
  max-width: 720px;
  font-size: 12.5px;
  line-height: 1.7;
  color: var(--bw-ink-soft);
}

.hero-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 7px;
  margin-top: 12px;
}

.hero-tags span {
  padding: 4px 10px;
  border-radius: 999px;
  border: 1px solid var(--bw-line);
  background: var(--bw-primary-soft);
  color: var(--bw-primary);
  font-size: 10.5px;
  font-weight: 600;
}

.about-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 16px;
}

.about-card {
  padding: 18px 20px;
  border-radius: var(--bw-radius);
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.about-card h3 {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0;
  font-size: 13.5px;
  font-weight: 650;
}

.about-card h3 :deep(.anticon) {
  color: var(--bw-primary);
}

.about-card p {
  margin: 0;
  font-size: 12px;
  line-height: 1.7;
  color: var(--bw-muted);
}

.tech-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
}

.tech-item,
.platform-item {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 10px;
  background: rgba(127, 127, 127, 0.07);
  font-size: 11.5px;
}

.tech-item strong,
.platform-item strong {
  font-weight: 650;
  color: var(--bw-ink);
}

.tech-item span,
.platform-item span {
  color: var(--bw-muted);
  font-size: 10.5px;
  text-align: right;
}

.platform-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.links-card {
  grid-column: 1 / -1;
}

.link-list {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 8px;
}

.link-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border: 1px solid var(--bw-line);
  border-radius: 11px;
  background: transparent;
  color: var(--bw-ink-soft);
  font-size: 12px;
  cursor: pointer;
  transition:
    background var(--bw-transition),
    border-color var(--bw-transition),
    color var(--bw-transition);
}

.link-item:hover {
  background: var(--bw-primary-soft);
  border-color: rgba(22, 119, 255, 0.3);
  color: var(--bw-primary);
}

.link-item span {
  flex: 1;
  text-align: left;
}

.link-item .arrow {
  opacity: 0.5;
  font-size: 11px;
}

.about-footer {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  padding: 4px 2px 10px;
  font-size: 11px;
  color: var(--bw-muted);
}

.dot {
  opacity: 0.5;
}

@media (max-width: 900px) {
  .about-grid {
    grid-template-columns: minmax(0, 1fr);
  }

  .link-list {
    grid-template-columns: minmax(0, 1fr);
  }
}

@media (max-width: 640px) {
  .about-hero {
    flex-direction: column;
    align-items: flex-start;
  }
}
</style>
