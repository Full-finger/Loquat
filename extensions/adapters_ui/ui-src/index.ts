// 「适配器」插件页入口（契约见 docs/ext-ui.md）。
// 一个 ui.js 注册两个元素：main = 列表，detail/<名> = 详情（ctx.tab 子路径）。
import { definePage } from '@webui/ext/mount'
import AdaptersPage from './AdaptersPage.vue'
import AdapterDetailPage from './AdapterDetailPage.vue'

definePage('loquat-page-adapters_ui-main', AdaptersPage)
definePage('loquat-page-adapters_ui-detail', AdapterDetailPage)
