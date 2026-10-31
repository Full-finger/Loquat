# Loquat WebUI

宿主壳（Vue 3 + Vite + Element Plus）。标签页尽量由插件注册
（契约见 `docs/ext-ui.md`），壳里只保留插件管理页与通用挂载器。

## 命令

```sh
npm run dev            # 开发壳（代理到宿主 7900，未起宿主走 mock）
npm run build          # 类型检查 + 构建壳 → dist/（宿主从这里托管）
npm run plugins        # 构建插件页：extensions/<名>_ui/ui-src → ui.js
npm run plugins:watch  # 同上，watch 模式（改插件页源码即时重构建）
```

## 布局

- `src/views/`：壳自有页面——插件管理（原生）、ExtensionPage（插件页
  通用挂载器）、Home（动态首页）。
- `src/ext/mount.ts`：插件页共享挂载器（`definePage` / `useCtx`）。
- `src/components/`、`src/composables/`：与插件页共享的组件与工具，
  插件源码经 `@webui/` 别名引用，不在插件里复制。
- `scripts/build-plugins.mjs`：插件页构建（单文件 ESM，scoped 样式注入）。
- `scripts/smoke-ext-tabs.mjs`：插件页冒烟（先起宿主再跑）。
