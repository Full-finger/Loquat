// 演示插件页面（契约见 docs/ext-ui.md）。三个要点：
// 1) 为每个 TABS 条目注册自定义元素 loquat-page-<插件名>-<tab id>
// 2) 宿主注入 ctx（挂载前设置，connectedCallback 可直接取）：
//    ctx.api.get/post 走宿主 http 封装（token 自动携带），ctx.toast 是
//    Element Plus 消息条薄封装
// 3) 必须防重复 define：热更后 v 变化会让宿主重新 import 本文件
const TAG = 'loquat-page-demo_ui-demo'

if (!customElements.get(TAG)) {
  customElements.define(
    TAG,
    class extends HTMLElement {
      async connectedCallback() {
        if (this.rendered) return
        this.rendered = true
        const { ctx } = this
        // 挂在宿主 DOM 里，直接用主题 CSS 变量（自带回退值便于独立调试）
        this.innerHTML = `
          <style>
            .demo-ui-card {
              max-width: 720px;
              background: var(--bg-panel, #1b1d23);
              border: 1px solid var(--border, #2a2d35);
              border-radius: 12px;
              padding: 20px 22px;
              color: var(--text-1, #e7e9ee);
              font-size: 13px;
              line-height: 1.8;
            }
            .demo-ui-card h2 { margin: 0 0 4px; font-size: 16px; }
            .demo-ui-card .hint { color: var(--text-3, #8a8f9c); margin: 0 0 14px; }
            .demo-ui-card pre {
              margin: 12px 0 0;
              background: var(--bg-raised, #23262e);
              border-radius: 8px;
              padding: 12px;
              overflow: auto;
              font-size: 12px;
              color: var(--text-2, #b8bdc9);
            }
            .demo-ui-card button {
              cursor: pointer;
              border: none;
              border-radius: 6px;
              padding: 6px 14px;
              background: var(--accent, #539bf5);
              color: #fff;
              font-size: 13px;
            }
          </style>
          <div class="demo-ui-card">
            <h2>插件演示页</h2>
            <p class="hint">
              这个页面完全由 extensions/demo_ui/ui.js 渲染——宿主只负责
              导航聚合（TABS）与脚本分发（/ext/demo_ui/ui.js）。
            </p>
            <button>ctx.toast 提示</button>
            <pre class="demo-ui-cfg">正在经 ctx.api.get 读取本插件配置…</pre>
          </div>`
        this.querySelector('button').addEventListener('click', () => {
          ctx.toast('来自插件页面的提示', 'success')
        })
        const cfgEl = this.querySelector('.demo-ui-cfg')
        try {
          const cfg = await ctx.api.get(`/api/plugins/${ctx.plugin}/config`)
          const values = cfg.values && Object.keys(cfg.values).length
            ? cfg.values
            : { 说明: '本插件未声明 SCHEMA，无配置文件' }
          cfgEl.textContent = JSON.stringify(values, null, 2)
        } catch (e) {
          cfgEl.textContent = `读取失败：${e}`
        }
      }
    },
  )
}
