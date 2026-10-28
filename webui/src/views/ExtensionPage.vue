<script setup lang="ts">
// 插件页面通用挂载器（WebUI 扩展机制，契约见 docs/ext-ui.md）。
// 设计对齐 Home Assistant panel_custom：动态 import 插件的 ui.js →
// 等它注册自定义元素 loquat-page-<插件>-<tab> → 挂载并注入 ctx。
// Python 插件本就在进程内跑任意代码，插件 JS 与其同信任域，不做沙箱。
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { ElMessage } from 'element-plus'
import { extCtxApi, fetchUiTabs, USE_MOCK, type ExtCtx, type UiTab } from '../api'

const route = useRoute()
const name = computed(() => String(route.params.name ?? ''))
const tab = computed(() => String(route.params.tab ?? ''))
// 元素标签契约：loquat-page-<插件名>-<tab id>（ui.js 按 it define）
const tag = computed(() => `loquat-page-${name.value}-${tab.value}`)

const host = ref<HTMLElement>()
const state = ref<'loading' | 'ok' | 'error'>('loading')
const errMsg = ref('')

function unmountPage() {
  host.value?.replaceChildren()
}

function makeCtx(): ExtCtx {
  return {
    plugin: name.value,
    tab: tab.value,
    api: extCtxApi(),
    toast: (msg, type) => ElMessage({ message: msg, type: type ?? 'info' }),
  }
}

async function load() {
  unmountPage()
  state.value = 'loading'
  errMsg.value = ''
  if (USE_MOCK) {
    state.value = 'error'
    errMsg.value = 'mock 模式没有插件页面：启动宿主后经真实后端访问'
    return
  }
  try {
    // tab 清单拿入口与缓存指纹；取不到说明插件被禁用或未声明 TABS
    let tabs: UiTab[]
    try {
      tabs = await fetchUiTabs()
    } catch (e) {
      throw new Error(`宿主不可达（/api/extensions/ui）：${e}`)
    }
    const meta = tabs.find((t) => t.id === `${name.value}-${tab.value}`)
    if (!meta?.entry) {
      throw new Error(`插件 ${name.value} 未注册页面「${tab.value}」（可能已被禁用或未声明 TABS）`)
    }

    // 动态 import() 带不了 Authorization 头：token 走查询参数（宿主同步
    // 支持）；v 是 ui.js mtime，热更后变化击穿 module 缓存
    const q = new URLSearchParams()
    const token = localStorage.getItem('loquat_token')
    if (token) q.set('token', token)
    if (meta.v) q.set('v', String(meta.v))
    await import(/* @vite-ignore */ `${meta.entry}?${q}`)

    // import 成功 ≠ 元素已注册：契约违规时用超时兜底
    //（Home Assistant「Custom element doesn't exist」同款坑）
    const timeout = new Promise<never>((_, reject) =>
      setTimeout(() => reject(new Error('等待元素注册超时（10s）')), 10_000),
    )
    await Promise.race([customElements.whenDefined(tag.value), timeout])
    if (!customElements.get(tag.value)) {
      throw new Error(`ui.js 未注册元素 <${tag.value}>（契约：为每个 tab define 该标签）`)
    }

    const el = document.createElement(tag.value) as HTMLElement & { ctx?: ExtCtx }
    el.ctx = makeCtx() // 先设置再插入，connectedCallback 里即可取到
    host.value?.appendChild(el)
    state.value = 'ok'
  } catch (e) {
    errMsg.value = String(e)
    state.value = 'error'
  }
}

onMounted(load)
// /ext/a/x ↔ /ext/b/y 复用同一组件实例，参数变化时重挂
watch([name, tab], load)
onUnmounted(unmountPage)
</script>

<template>
  <div class="ext-page">
    <el-skeleton v-if="state === 'loading'" :rows="6" animated />
    <el-result
      v-else-if="state === 'error'"
      icon="warning"
      title="插件页面加载失败"
      :sub-title="errMsg"
    />
    <!-- 挂载点常驻：卸载/重挂由脚本控制 -->
    <div v-show="state === 'ok'" ref="host" />
  </div>
</template>

<style scoped>
.ext-page {
  min-height: 200px;
}
</style>
