# 插件页面扩展（WebUI ext-ui）

设计参考 Home Assistant `panel_custom` 与 Grafana `module.js`：插件自带页面
脚本，宿主负责导航聚合与脚本分发，前端运行时动态挂载。不做 iframe 沙箱
（VSCode webview 式）——Python 插件本就在宿主进程内跑任意代码，插件 JS
与其同信任域，隔离是安全表演。

## 契约

### 插件侧（extensions/<名>/）

`plugin.py` 声明模块属性（与 SCHEMA / PRIORITY 同风格）：

```python
TABS = [
    {"id": "stats", "title": "统计", "icon": "TrendCharts"},  # icon 可省略
]
```

同目录 `ui.js`（原生 ES module，免构建、手写即可）为每个 tab 注册自定义
元素，元素名约定 `loquat-page-<插件名>-<tab id>`：

```js
const TAG = 'loquat-page-demo_ui-demo'
if (!customElements.get(TAG)) {           // 必须守卫，见下
  customElements.define(TAG, class extends HTMLElement {
    connectedCallback() { /* 用 this.ctx 渲染 */ }
  })
}
```

元素上会被注入 `ctx` 属性（挂载前设置，`connectedCallback` 里可直接取）：

```ts
ctx = {
  plugin: string, tab: string,
  api: { get(path), post(path, body) },  // 走前端 http 封装，token 自动携带
  toast(msg, type?),                      // Element Plus 消息条
}
```

完整可运行样例：`extensions/demo_ui/`。

### 宿主侧（src/）

- `GET /api/extensions/ui`（`Business::ui_tabs`）：聚合**已启用**插件的
  TABS 声明，条目不合规（缺 id/title）或 ui.js 缺失时丢弃。返回
  `{ id: "<插件>-<tab>", title, path: "/ext/<插件>/<tab>", entry, icon, v }`，
  `v` 是 ui.js 的 mtime 毫秒。
- `GET /ext/<插件>/ui.js`（`Business::plugin_ui_js`）：脚本分发。动态
  `import()` 带不了 Authorization 头，token 走 `?token=` 查询参数兜底
  （与 `/api/ws` 同款）；name 只允许字母数字`-_` 且目录必须含 plugin.py，
  挡路径穿越。

### 前端侧（webui/src/）

- 内置页面导航在 `tabs.ts` 单一声明（组件映射在 `router/index.ts`），
  新增内置页两处各加一条，不碰宿主。
- `/ext/:name/:tab?` → `ExtensionPage.vue` 通用挂载器：import（URL 带
  `token` 与 `v` 查询参数）→ `customElements.whenDefined`（10s 超时）→
  挂载注入 ctx。import 成功但元素没注册属契约违规，会给带标签名的
  错误卡片。

## 必踩点（都在挂载器里处理过，写插件时留意）

- **防重复 define**：ui.js 改动后 `v` 变化，宿主会让前端重新 import 同
  一文件，模块再次执行——不守卫 `customElements.get` 会抛
  "already defined"。
- **token**：`import()` 与 WebSocket 都带不了自定义头，只有查询参数一条
  路（会进访问日志；服务只绑回环）。
- **热更粒度**：改 ui.js 不重启宿主，`v` 指纹自动击穿浏览器缓存。
- **侧栏实时性**：启停/安装在页面上操作后即时增删 tab；控制台命令、
  手改插件目录等外部变更最迟 10s（轮询周期）出现，都无需刷新页面。
- **ctx 生命周期**：每次路由进入重新创建元素并注入新 ctx，离开时元素整
  个移除——插件页面的内部状态不跨访问保留。

## 边界（现状，按需演进）

- ctx 暂无 WS 订阅通道：需要实时消息流的页面等 v2。
- 一个插件多个 tab 共享同一个 ui.js（按 tab id 注册多个元素即可）。
- 插件页面跑在主文档信任域，等价于它 Python 部分的权限——恶意插件页面
  不是威胁模型（它本来就能在进程里干任何事）。
