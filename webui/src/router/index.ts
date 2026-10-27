import { createRouter, createWebHistory } from 'vue-router'

export default createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', redirect: '/dashboard' },
    { path: '/dashboard', component: () => import('../views/Dashboard.vue') },
    { path: '/plugins', component: () => import('../views/Plugins.vue') },
    { path: '/messages', component: () => import('../views/Messages.vue') },
    { path: '/sandbox', component: () => import('../views/Sandbox.vue') },
    { path: '/adapters', component: () => import('../views/Adapters.vue') },
  ],
})
