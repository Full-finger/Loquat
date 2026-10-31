import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vite'

// 开发时转发到 Rust 宿主的 axum。宿主换端口时用 VITE_API_TARGET 覆盖：
//   VITE_API_TARGET=http://127.0.0.1:7901 npm run dev
// 未启动宿主也页面不炸——前端在 dev 下默认走 mock（VITE_USE_MOCK=false 可关）。
const target = process.env.VITE_API_TARGET ?? 'http://127.0.0.1:7900'

// https://vite.dev/config/
export default defineConfig({
  plugins: [vue()],
  server: {
    proxy: {
      '/api': {
        target,
        ws: true,
      },
      // 插件页面脚本（WebUI 扩展机制）：动态 import 的 ui.js 同源转发宿主
      '/ext': {
        target,
      },
    },
  },
})
