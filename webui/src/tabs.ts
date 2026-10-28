// 内置导航 tab 的唯一声明点。页面组件在 router/index.ts（path → 组件），
// 这里只声明入口：新增内置页 = 两处各加一条，不碰宿主。
// 插件贡献的 tab（TABS + ui.js 契约，见 docs/ext-ui.md）由宿主经
// /api/extensions/ui 聚合，App.vue 在内置列表后动态合并。
export interface BuiltinTab {
  id: string
  title: string
  path: string
  /** element-plus 图标组件名（main.ts 已全局注册，字符串即可解析） */
  icon: string
}

export const BUILTIN_TABS: BuiltinTab[] = [
  { id: 'dashboard', title: '仪表盘', path: '/dashboard', icon: 'Odometer' },
  { id: 'plugins', title: '插件', path: '/plugins', icon: 'Grid' },
  { id: 'messages', title: '消息流', path: '/messages', icon: 'ChatDotRound' },
  { id: 'providers', title: '服务', path: '/providers', icon: 'Coin' },
  { id: 'adapters', title: '适配器', path: '/adapters', icon: 'Setting' },
  { id: 'sandbox', title: '沙箱', path: '/sandbox', icon: 'Monitor' },
]
