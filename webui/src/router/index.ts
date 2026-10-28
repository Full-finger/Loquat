import { createRouter, createWebHistory } from 'vue-router'

export default createRouter({
  history: createWebHistory(),
  routes: [
    { path: '/', redirect: '/dashboard' },
    { path: '/dashboard', component: () => import('../views/Dashboard.vue') },
    { path: '/plugins', component: () => import('../views/Plugins.vue') },
    {
      path: '/plugins/:name/config',
      component: () => import('../views/PluginConfig.vue'),
      // 子页：顶栏显示返回按钮（App.vue 按 meta.sub 渲染），parent 是返回兜底
      meta: { sub: true, parent: '/plugins' },
    },
    { path: '/messages', component: () => import('../views/Messages.vue') },
    { path: '/providers', component: () => import('../views/Providers.vue') },
    { path: '/sandbox', component: () => import('../views/Sandbox.vue') },
    { path: '/adapters', component: () => import('../views/Adapters.vue') },
    {
      path: '/adapters/:name',
      component: () => import('../views/AdapterConfig.vue'),
      meta: { sub: true, parent: '/adapters' },
    },
    {
      // 插件页面（WebUI 扩展机制，契约见 docs/ext-ui.md）：
      // 组件是通用的，内容由插件的 ui.js 在运行时挂载
      path: '/ext/:name/:tab?',
      component: () => import('../views/ExtensionPage.vue'),
    },
  ],
})
