// 「仪表盘」插件页入口（契约见 docs/ext-ui.md）
import { definePage } from '@webui/ext/mount'
import DashboardPage from './DashboardPage.vue'

definePage('loquat-page-dashboard_ui-main', DashboardPage)
