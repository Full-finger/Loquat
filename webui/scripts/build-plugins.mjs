// 构建插件 WebUI 页面：extensions/<名>/ui-src/index.ts → extensions/<名>/ui.js
// （单文件 ESM，SFC 样式以 <style id="lq-ui-<名>"> 注入 document.head，幂等）。
//
// 用法（在 webui/ 下）：
//   node scripts/build-plugins.mjs           # 全量构建一次
//   node scripts/build-plugins.mjs sandbox_ui  # 只构建指定插件
//   node scripts/build-plugins.mjs --watch   # 全量 watch（开发时挂着）
//
// 说明：
// - root 设为 webui/，插件源码直接复用 webui/node_modules（vue、element-plus），
//   插件目录不需要自己的 package.json。
// - element-plus / 主题 CSS 由宿主壳（main.ts）全局加载，插件不打包它们，
//   避免每个插件重复注入一份全局样式；插件只打包自己的 scoped 样式。
// - 产物 ui.js 由宿主按 docs/ext-ui.md 契约分发（GET /ext/<名>/ui.js）。
import { build } from 'vite'
import vue from '@vitejs/plugin-vue'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const webui = path.dirname(path.dirname(fileURLToPath(import.meta.url)))
const extRoot = path.resolve(webui, '..')

const argv = process.argv.slice(2)
const watch = argv.includes('--watch')
const only = argv.filter((a) => a !== '--watch')

const names = fs
  .readdirSync(path.join(extRoot, 'extensions'), { withFileTypes: true })
  .filter((d) => d.isDirectory())
  .map((d) => d.name)
  .filter((n) => (!only.length || only.includes(n)) && fs.existsSync(path.join(extRoot, 'extensions', n, 'ui-src', 'index.ts')))

if (!names.length) {
  console.error('没有找到带 ui-src/index.ts 的插件目录')
  process.exit(1)
}

// 库模式会把 SFC 样式抽到 bundle.css；这里把它并回 js 顶部，
// 按 id 幂等注入——热更重新 import 时不会叠出多份样式。
function injectCssPlugin(name) {
  return {
    name: `lq-inject-css(${name})`,
    closeBundle() {
      const dir = path.join(extRoot, 'extensions', name, '.uibuild')
      const css = fs.existsSync(path.join(dir, 'bundle.css'))
        ? fs.readFileSync(path.join(dir, 'bundle.css'), 'utf8')
        : ''
      const guard = css
        ? `if(!document.getElementById('lq-ui-${name}')){` +
          `var s=document.createElement('style');s.id='lq-ui-${name}';` +
          `s.textContent=${JSON.stringify(css)};document.head.appendChild(s);}`
        : ''
      const js = fs.readFileSync(path.join(dir, 'bundle.js'), 'utf8')
      fs.writeFileSync(path.join(extRoot, 'extensions', name, 'ui.js'), guard + '\n' + js)
    },
  }
}

let failed = false
await Promise.all(
  names.map(async (name) => {
    const dir = path.join(extRoot, 'extensions', name)
    try {
      await build({
        root: webui,
        configFile: false,
        plugins: [vue(), injectCssPlugin(name)],
        resolve: {
          // 插件源码复用宿主壳的组件与工具（只读引用，单一实现点）。
          // 裸依赖显式指到 webui/node_modules：插件目录在 webui 之外，
          // 解析器从 importer 向上走不到 node_modules
          alias: {
            '@webui': path.join(webui, 'src'),
            vue: path.join(webui, 'node_modules/vue'),
            'element-plus': path.join(webui, 'node_modules/element-plus'),
            '@element-plus/icons-vue': path.join(webui, 'node_modules/@element-plus/icons-vue'),
          },
        },
        define: {
          // vue / element-plus 的 bundler 构建引用 process.env.NODE_ENV，
          // 库模式不做自动替换，浏览器里没有 process，必须在构建期抹掉
          'process.env.NODE_ENV': JSON.stringify('production'),
        },
        build: {
          outDir: path.join(dir, '.uibuild'),
          emptyOutDir: true,
          watch: watch ? {} : undefined,
          lib: {
            entry: path.join(dir, 'ui-src', 'index.ts'),
            formats: ['es'],
            fileName: () => 'bundle.js',
          },
          rollupOptions: {
            // 库模式默认把 package.json 依赖外置（浏览器里没人解析裸名），
            // 插件要的是自包含单文件，显式清空
            external: [],
            output: {
              entryFileNames: 'bundle.js',
              assetFileNames: 'bundle.[ext]',
            },
          },
          target: 'es2022',
        },
      })
      console.log(`[ui] extensions/${name}/ui.js 构建完成`)
    } catch (e) {
      failed = true
      console.error(`[ui] extensions/${name} 构建失败:`, e?.message ?? e)
    }
  }),
)

// 一次性构建后清掉中间产物；watch 模式保留（每次重构建都会重写）
if (!watch) {
  for (const name of names) {
    fs.rmSync(path.join(extRoot, 'extensions', name, '.uibuild'), { recursive: true, force: true })
  }
}
if (failed && !watch) process.exit(1)
