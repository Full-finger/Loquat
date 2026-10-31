// 「服务」插件页入口（契约见 docs/ext-ui.md）：注册契约自定义元素，
// 页面实现见 ProvidersPage.vue。防重复 define 在 definePage 内处理。
import { definePage } from '@webui/ext/mount'
import ProvidersPage from './ProvidersPage.vue'

definePage('loquat-page-providers_ui-main', ProvidersPage)
