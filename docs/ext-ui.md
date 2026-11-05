# 插件页面扩展（WebUI ext-ui）

设计参考 Home Assistant `panel_custom` 与 Grafana `module.js`：插件自带页面
脚本，宿主负责导航聚合与脚本分发，前端运行时动态挂载。不做 iframe 沙箱
（VSCode webview 式）——Python 插件本就在宿主进程内跑任意代码，插件 JS
与其同信任域，隔离是安全表演。

设计基线：**标签页尽量由插件注册，宿主壳只留插件管理页**（它是启停插件
的入口，插件化会自锁）。五个功能页已迁移为第一方插件
`extensions/<名>_ui/`；「/」落地到排序后的第一个 tab（views/Home.vue），
全部插件页禁用时兜底到插件管理页。

## UI 偏好（壳与设置页的分工）

设置页（settings_ui）只是编辑界面；偏好本体与生效逻辑在壳的
`webui/src/ext/prefs.ts`：`{ theme, accent, home }` 存 localStorage 单键，
`applyPrefs` 切 html.dark class（theme）并写内联变量（accent），壳启动时
应用一次，设置页改动即时应用；index.html 里有同款首帧脚本防闪变。
新增偏好 = prefs.ts 加字段 + 设置页加控件，两处。

主题与主题色：theme.css 三段结构——:root 放结构与 --el-* 映射，
html.dark / html:not(.dark) 各一套 Primer 色号（浅色 = Primer Light）。
所有 accent/pink/danger 派生色（可读文本、徽标、气泡、渐变）一律
color-mix 自对应 token 或用 --accent-text/--pink-text/--danger-text，
不写硬编码色值；element-plus 经 --el-color-primary: var(--accent) 自动跟随。

## 契约

### 插件侧（extensions/<名>/）

`plugin.py` 声明模块属性（与 SCHEMA / PRIORITY 同风格）：

```python
TABS = [
    # order 是侧栏排序权重（小在前），缺省 100；内置插件管理页固定 20
    {"id": "main", "title": "统计", "icon": "TrendCharts", "order": 40},
    # pin="bottom" 把入口固定在侧栏最底（分隔线下方），像 WinUI 的设置：
    # 不参与 order 竞争，其它 tab 再多也挤不掉（样例：settings_ui）
    {"id": "main", "title": "设置", "icon": "Setting", "order": 90, "pin": "bottom"},
]
```

tab id 支持子路径导航：路由 `/ext/<插件>/<id>/<子路径>` 会挂载同一个
元素，完整子路径经 `ctx.tab` 传入（如 `detail/napcat`），插件自行分段
解析。适合「列表 + 详情」式页面（样例：adapters_ui）。

页面脚本两种形态，宿主不区分（只认 `ui.js` 这个文件）：

- **免构建手写**：原生 ES module + 自定义元素（样例：demo_ui）。
- **构建产物**：源码在 `ui-src/`（TS + Vue SFC），由 webui 的构建脚本
  打成单文件 `ui.js`（第一方插件用这种）。构建复用 webui/node_modules，
  插件目录不需要自己的 package.json：

  ```sh
  cd webui
  npm run plugins          # 全量构建一次
  npm run plugins:watch    # 开发时挂着，改 ui-src 即时重build
  ```

  element-plus 与主题 CSS 由宿主壳全局加载，插件只打包自己的 scoped
  样式（构建脚本以 `<style id="lq-ui-<名>">` 幂等注入）。共享组件、
  工具与 API 类型经 `@webui/` 别名直接引用宿主壳源码（单一实现点），
  类型引用一律 `import type`（构建期擦除）。

两种形态都要为每个 tab 注册自定义元素，元素名约定
`loquat-page-<插件名>-<tab id>`：

```js
const TAG = 'loquat-page-demo_ui-demo'
if (!customElements.get(TAG)) {           // 必须守卫，见下
  customElements.define(TAG, class extends HTMLElement {
    connectedCallback() { /* 用 this.ctx 渲染 */ }
  })
}
```

构建型插件用共享挂载器（等价于上面的手写）：

```ts
import { definePage } from '@webui/ext/mount'
import StatsPage from './StatsPage.vue'

definePage('loquat-page-stats_ui-main', StatsPage)
```

元素上会被注入 `ctx` 属性（挂载前设置，`connectedCallback` 里可直接取）：

```ts
ctx = {
  plugin: string,
  tab: string,                            // 含子路径，如 "detail/napcat"
  api: {                                  // 走前端 http 封装，token 自动携带
    get(path), post(path, body?), patch(path, body?), del(path),
  },
  toast(msg, type?),                      // Element Plus 消息条
  nav(path),                              // 跳前端路由（其他 tab / 内置页 / 自己的子路径）
  wsUrl(),                                // 事件流 WS 地址（token 查询参数兜底）
  guardDirty(isDirty),                    // 未保存守卫：路由离开/刷新关页时拦截确认
}
```

### 宿主侧（src/）

- `GET /api/extensions/ui`（`Business::ui_tabs`）：聚合**已启用**插件的
  TABS 声明，条目不合规（缺 id/title）或 ui.js 缺失时丢弃。返回
  `{ id: "<插件>-<tab>", title, path: "/ext/<插件>/<tab>", entry, icon,
  order, pin, v }`，`v` 是 ui.js 的 mtime 毫秒；`pin` 仅在声明为
  `"bottom"` 时出现。
- `GET /ext/<插件>/ui.js`（`Business::plugin_ui_js`）：脚本分发。动态
  `import()` 带不了 Authorization 头，token 走 `?token=` 查询参数兜底
  （与 `/api/ws` 同款）；name 只允许字母数字`-_` 且目录必须含 plugin.py，
  挡路径穿越。

### 前端侧（webui/src/）

- 内置导航只剩插件管理页（`tabs.ts` 单一声明）；插件 tab 按 order 升序
  合并（`mergeTabs`），激活高亮与面包屑把 `/ext/<插件>/<tab>[/子路径]`
  归到插件的入口 tab（`App.vue matchTab`）。
- `/ext/:name[/:tab]` → `ExtensionPage.vue` 通用挂载器：import（URL 带
  `token` 与 `v` 查询参数）→ `customElements.whenDefined`（10s 超时）→
  挂载注入 ctx。tab 子路径重挂同一 ui.js，元数据按插件回退匹配。
- `/` → `Home.vue`：等第一份 tab 清单，跳到排序后的第一个 tab。

## 必踩点（都在挂载器/构建脚本里处理过，写插件时留意）

- **防重复 define**：ui.js 改动后 `v` 变化，宿主会让前端重新 import 同
  一文件，模块再次执行——不守卫 `customElements.get` 会抛
  "already defined"（`definePage` 已内置）。
- **token**：`import()` 与 WebSocket 都带不了自定义头，只有查询参数一条
  路（会进访问日志；服务只绑回环）。
- **热更粒度**：改 ui.js 不重启宿主，`v` 指纹自动击穿浏览器缓存；
  watch 构建下改 ui-src 保存即生效。
- **侧栏实时性**：启停/安装在页面上操作后即时增删 tab；控制台命令、
  手改插件目录等外部变更最迟 10s（轮询周期）出现，都无需刷新页面。
- **ctx 生命周期**：每次路由进入重新创建元素并注入新 ctx，离开时元素整
  个移除——插件页面的内部状态不跨访问保留。
- **process.env.NODE_ENV**：vue/element-plus 的 bundler 构建引用它，
  库模式不做自动替换；构建脚本已 define 成 production，自己写构建的
  插件记得同样处理。

## 边界（现状，按需演进）

- 构建型插件各自内联一份 vue + element-plus（约 1.5MB/个，本机工具不
  计较）；宿主壳与插件页各自持有 Vue 实例，互不通信，协作只经 DOM 与 ctx。
- ctx 暂无细粒度 WS 订阅通道：wsUrl 是裸地址，帧过滤插件自己做
  （sandbox_ui 只取 platform=sandbox 的 out 帧，同款做法可复制）。
- 插件页面跑在主文档信任域，等价于它 Python 部分的权限——恶意插件页面
  不是威胁模型（它本来就能在进程里干任何事）。

## 回归

`webui/scripts/smoke-ext-tabs.mjs`：起宿主后跑，覆盖首页落地、侧栏顺序、
五个插件页挂载、适配器详情子路径与返回。
