import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [vue()],
  server: {
    proxy: {
      // 开发时转发到 Rust 宿主的 axum（未启动时页面走 mock 数据）
      '/api': {
        target: 'http://127.0.0.1:7900',
        ws: true,
      },
      // 插件页面脚本（WebUI 扩展机制）：动态 import 的 ui.js 同源转发宿主
      '/ext': {
        target: 'http://127.0.0.1:7900',
      },
    },
  },
})
