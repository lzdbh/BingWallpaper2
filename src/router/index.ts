import { createRouter, createWebHashHistory } from 'vue-router'

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    {
      path: '/',
      name: 'home',
      component: () => import('@/views/HomeView.vue'),
      meta: { immersive: true, title: 'nav.home' },
    },
    {
      path: '/gallery',
      name: 'gallery',
      component: () => import('@/views/GalleryView.vue'),
      meta: { title: 'nav.gallery' },
    },
    {
      path: '/downloads',
      name: 'downloads',
      component: () => import('@/views/DownloadsView.vue'),
      meta: { title: 'nav.downloads' },
    },
    {
      path: '/settings',
      name: 'settings',
      component: () => import('@/views/SettingsView.vue'),
      meta: { title: 'nav.settings' },
    },
    {
      path: '/about',
      name: 'about',
      component: () => import('@/views/AboutView.vue'),
      meta: { title: 'nav.about' },
    },
    { path: '/:pathMatch(.*)*', redirect: '/' },
  ],
  scrollBehavior: () => ({ top: 0 }),
})

export default router
