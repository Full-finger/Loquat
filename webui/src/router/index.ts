import { createRouter, createWebHistory } from 'vue-router'

export default createRouter({
  history: createWebHistory(),
  routes: [
    // 动态首页：跳到排序后的第一个 tab（views/Home.vue），不再是写死的
    // 仪表盘——导航已尽量由插件注册，默认页跟着插件走
    { path: '/', component: () => import('../views/Home.vue') },
    { path: '/plugins', component: () => import('../views/Plugins.vue') },
    {
      path: '/plugins/:name/config',
      component: () => import('../views/PluginConfig.vue'),
      // 子页：顶栏显示返回按钮（App.vue 按 meta.sub 渲染），parent 是返回兜底
      meta: { sub: true, parent: '/plugins' },
    },
    {
      // 插件页面（WebUI 扩展机制，契约见 docs/ext-ui.md）：
      // 组件是通用的，内容由插件的 ui.js 在运行时挂载。
      // tab 支持子路径（如 detail/napcat）：插件内部二级导航仍可深链
      path: '/ext/:name',
      component: () => import('../views/ExtensionPage.vue'),
    },
    {
      path: '/ext/:name/:tab(.*)',
      component: () => import('../views/ExtensionPage.vue'),
    },
  ],
})
