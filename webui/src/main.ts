import { createApp } from 'vue'
import ElementPlus from 'element-plus'
import 'element-plus/dist/index.css'
import 'element-plus/theme-chalk/dark/css-vars.css'
import * as ElementPlusIconsVue from '@element-plus/icons-vue'
// 霞鹜文楷屏幕版：unicode-range 分片按需加载，首屏只拉用到的几十 KB
import 'lxgw-wenkai-screen-webfont/lxgwwenkaiscreen.css'
import App from './App.vue'
import router from './router'
import { applyPrefs, loadPrefs } from './ext/prefs'
import './styles/theme.css'

// 用户偏好先于首帧生效，避免主题色闪变
applyPrefs(loadPrefs())

const app = createApp(App)
for (const [key, component] of Object.entries(ElementPlusIconsVue)) {
  app.component(key, component)
}
app.use(ElementPlus)
app.use(router)
app.mount('#app')
