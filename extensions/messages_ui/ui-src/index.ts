// 「消息流」插件页入口（契约见 docs/ext-ui.md）
import { definePage } from '@webui/ext/mount'
import MessagesPage from './MessagesPage.vue'

definePage('loquat-page-messages_ui-main', MessagesPage)
