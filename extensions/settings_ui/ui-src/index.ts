// 「设置」插件页入口（契约见 docs/ext-ui.md）
import { definePage } from '@webui/ext/mount'
import SettingsPage from './SettingsPage.vue'

definePage('loquat-page-settings_ui-main', SettingsPage)
