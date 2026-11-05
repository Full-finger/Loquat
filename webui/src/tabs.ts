// 内置导航 tab 的唯一声明点。页面组件在 router/index.ts（path → 组件）。
// 设计基线：标签页尽量由插件注册（TABS + ui.js 契约，见 docs/ext-ui.md），
// 这里只保留插件管理页本身——它是启停插件的入口，插件化会自锁。
// 插件贡献的 tab 由宿主经 /api/extensions/ui 聚合，App.vue 用 mergeTabs
// 排序合并；「/」落地到排序后的第一个 tab（views/Home.vue）。
import type { UiTab } from './api'

export interface BuiltinTab {
  id: string
  title: string
  path: string
  /** element-plus 图标组件名（main.ts 已全局注册，字符串即可解析） */
  icon: string
  /** 排序权重：插件管理页固定 20，其余内置位从 10 起 */
  order: number
  /** WinUI 式置底（分隔线下方），目前只有 "bottom" */
  pin?: 'bottom'
}

export const BUILTIN_TABS: BuiltinTab[] = [
  { id: 'plugins', title: '插件', path: '/plugins', icon: 'Grid', order: 20 },
]

/** 侧栏最终清单：内置在前、插件在后，按 order 升序稳定排序；
 *  pin="bottom" 的项（如设置）殿后——WinUI NavigationView 的 Settings 就是
 *  这个待遇：分隔线下方固定底部，其它 tab 再多也挤不掉它。 */
export function mergeTabs(ext: UiTab[]): UiTab[] {
  const merged = [...BUILTIN_TABS, ...ext].sort((a, b) => (a.order ?? 100) - (b.order ?? 100))
  const pinned = merged.filter((t) => t.pin === 'bottom')
  return [...merged.filter((t) => t.pin !== 'bottom'), ...pinned]
}
