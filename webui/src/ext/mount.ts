// 插件页面的 Vue 挂载器（被构建型插件的 ui-src 引用，见 docs/ext-ui.md）。
// 根组件挂到契约自定义元素上（ExtensionPage 挂载前注入 ctx）。
// 用轻 DOM（无 shadow root）：主题变量与 element-plus 全局样式直接生效，
// element-plus JS 由插件自带，CSS 由宿主壳提供——不要在插件里再引全局样式。
import { createApp, inject, type Component, type InjectionKey } from 'vue'
import ElementPlus from 'element-plus'
import * as Icons from '@element-plus/icons-vue'
import type { ExtCtx } from '../api'

const CTX_KEY: InjectionKey<ExtCtx> = Symbol('extCtx')

/** 注册契约自定义元素：loquat-page-<插件>-<tab> → Vue 应用。 */
export function definePage(tag: string, Root: Component): void {
  // 必须守卫：ui.js 热更后 v 变化，宿主重新 import 本文件，
  // 不守卫 customElements.define 会抛 "already defined"
  if (customElements.get(tag)) return
  customElements.define(
    tag,
    class extends HTMLElement {
      private app?: ReturnType<typeof createApp>

      connectedCallback() {
        if (this.app) return
        const host = document.createElement('div')
        this.appendChild(host)
        this.app = createApp(Root)
        for (const [name, comp] of Object.entries(Icons)) this.app.component(name, comp)
        this.app.use(ElementPlus)
        this.app.provide(CTX_KEY, (this as { ctx?: ExtCtx }).ctx as ExtCtx)
        this.app.mount(host)
      }

      disconnectedCallback() {
        this.app?.unmount()
        this.app = undefined
      }
    },
  )
}

/** 页面组件里取宿主注入的 ctx（definePage 挂载时 provide）。 */
export function useCtx(): ExtCtx {
  return inject(CTX_KEY) as ExtCtx
}
