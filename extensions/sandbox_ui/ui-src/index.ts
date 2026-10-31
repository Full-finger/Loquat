// 「沙箱」插件页入口（契约见 docs/ext-ui.md）
import { definePage } from '@webui/ext/mount'
import SandboxPage from './SandboxPage.vue'

definePage('loquat-page-sandbox_ui-main', SandboxPage)
