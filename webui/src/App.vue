<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { ChatDotRound, Coin, Grid, Link, Monitor, Odometer, Setting } from '@element-plus/icons-vue'
import { fetchUiTabs, getStatus, type UiTab } from './api'

const route = useRoute()
const coreOk = ref<boolean | null>(null)
const collapsed = ref(false)

// 导航 tab 由宿主注册（通用扩展机制）：内置页与扩展页同一路径声明。
// 组件映射在前端路由表；拉取失败回退到内置三项。
const tabs = ref<UiTab[]>([
  { id: 'dashboard', title: '仪表盘', path: '/dashboard' },
  { id: 'plugins', title: '插件', path: '/plugins' },
  { id: 'messages', title: '消息流', path: '/messages' },
])

const TAB_ICONS: Record<string, unknown> = {
  dashboard: Odometer,
  plugins: Grid,
  messages: ChatDotRound,
  providers: Coin,
  sandbox: Monitor,
  adapters: Setting,
}

function iconOf(id: string): unknown {
  return TAB_ICONS[id] ?? Link
}

// VSCode 式搜索栏：占位，暂无搜索功能；Ctrl+K 聚焦
const searchEl = ref<HTMLInputElement>()
const searchText = ref('')

function onKeydown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
    e.preventDefault()
    searchEl.value?.focus()
  }
}

// WinUI 式连续指示条：胶囊短条，切菜单时滑到激活项位置
const menuEl = ref<HTMLElement>()
const indY = ref(0)
const indH = ref(18)
const indVisible = ref(false)

function moveIndicator() {
  nextTick(() => {
    const active = menuEl.value?.querySelector('.el-menu-item.is-active') as HTMLElement | null
    const wrap = menuEl.value
    if (!active || !wrap) return
    // 用视口坐标差值定位，不依赖 offsetParent 参考系
    const wrapTop = wrap.getBoundingClientRect().top
    const itemTop = active.getBoundingClientRect().top
    indY.value = itemTop - wrapTop + (active.offsetHeight - indH.value) / 2
    indVisible.value = true
  })
}

watch(() => route.path, moveIndicator)

let timer: number | undefined

async function poll() {
  try {
    const s = await getStatus()
    coreOk.value = s.core === '运行中'
  } catch {
    coreOk.value = false
  }
}

onMounted(() => {
  moveIndicator()
  poll()
  fetchUiTabs()
    .then((t) => {
      if (t.length > 0) tabs.value = t
    })
    .catch(() => {}) // 拉不到就用内置三项
    .finally(() => moveIndicator())
  timer = window.setInterval(poll, 10_000)
  window.addEventListener('keydown', onKeydown)
})
onUnmounted(() => {
  clearInterval(timer)
  window.removeEventListener('keydown', onKeydown)
})
</script>

<template>
  <el-container class="layout">
    <el-header height="48px" class="topbar">
      <div class="top-left">
        <button
          class="fold-btn"
          :title="collapsed ? '展开侧栏' : '折叠侧栏'"
          @click="collapsed = !collapsed"
        >
          <el-icon :size="16">
            <Expand v-if="collapsed" />
            <Fold v-else />
          </el-icon>
        </button>
        <span class="brand-dot" />
        <span class="brand-name">Loquat</span>
        <span class="brand-ver mono">v0.1</span>
      </div>

      <div class="top-center">
        <div class="search-box">
          <el-icon :size="14"><Search /></el-icon>
          <input
            ref="searchEl"
            v-model="searchText"
            placeholder="搜索命令、插件、会话…"
            @keydown.escape="searchEl?.blur()"
          />
          <span class="kbd mono">Ctrl K</span>
        </div>
      </div>

      <div class="top-right">
        <span class="pulse" :class="{ off: coreOk === false }" />
        <span>{{ coreOk === false ? '核心未就绪' : '核心运行中' }}</span>
      </div>
    </el-header>

    <el-container class="body">
      <el-aside :width="collapsed ? '64px' : '200px'" class="aside" :class="{ collapsed }">
        <div ref="menuEl" class="menu-wrap">
          <span
            v-show="indVisible"
            class="indicator"
            :style="{ transform: `translateY(${indY}px)`, height: `${indH}px` }"
          />
          <el-menu router :default-active="route.path" class="menu">
            <el-menu-item v-for="t in tabs" :key="t.id" :index="t.path">
              <el-icon><component :is="iconOf(t.id)" /></el-icon>
              <span>{{ t.title }}</span>
            </el-menu-item>
          </el-menu>
        </div>
      </el-aside>

      <el-main class="main">
        <router-view v-slot="{ Component }">
          <transition name="fade" mode="out-in">
            <component :is="Component" />
          </transition>
        </router-view>
      </el-main>
    </el-container>
  </el-container>
</template>

<style scoped>
.layout {
  height: 100vh;
}

/* ---- 顶部标题栏：左品牌 / 中搜索 / 右状态 ---- */
.topbar {
  background: var(--bg-frame);
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 0 12px;
}

.top-left {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.brand-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: linear-gradient(135deg, var(--accent), var(--pink));
  flex-shrink: 0;
}

.brand-name {
  font-size: 15px;
  font-weight: 700;
  letter-spacing: 0.02em;
  white-space: nowrap;
}

.brand-ver {
  color: var(--text-3);
  font-size: 11px;
  white-space: nowrap;
}

.fold-btn {
  border: none;
  background: transparent;
  color: var(--text-2);
  width: 40px; /* 中心对齐图标线 x=32：顶栏 padding 12 + 40/2 */
  height: 32px;
  border-radius: 6px;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  transition: background 0.12s ease, color 0.12s ease;
}

.fold-btn:hover {
  background: var(--bg-raised);
  color: var(--text-1);
}

.top-center {
  flex: 1;
  display: flex;
  justify-content: center;
  min-width: 0;
  /* 搜索框悬在主内容区上方：在品牌与状态之间的区间居中 */
}

.search-box {
  width: 100%;
  max-width: 560px;
  display: flex;
  align-items: center;
  gap: 8px;
  height: 32px;
  padding: 0 10px;
  background: var(--bg-raised);
  border-radius: 6px;
  color: var(--text-3);
  transition: background 0.15s ease;
}

.search-box:focus-within {
  background: var(--bg-panel);
}

.search-box input {
  flex: 1;
  min-width: 0;
  border: none;
  outline: none;
  background: transparent;
  color: var(--text-1);
  font-size: 13px;
}

.search-box input::placeholder {
  color: var(--text-3);
}

.kbd {
  font-size: 10px;
  color: var(--text-3);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 1px 5px;
  background: var(--bg-panel);
  white-space: nowrap;
}

.top-right {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
  color: var(--text-2);
  font-size: 12px;
  white-space: nowrap;
  flex-shrink: 0;
}

.pulse {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--ok);
  animation: breathe 2.4s ease-in-out infinite;
  flex-shrink: 0;
}

.pulse.off {
  background: var(--danger);
  animation: none;
}

@keyframes breathe {
  0%,
  100% {
    box-shadow: 0 0 0 0 rgba(87, 171, 90, 0.4);
  }
  50% {
    box-shadow: 0 0 0 5px rgba(87, 171, 90, 0);
  }
}

/* ---- 下半区：图标侧栏 + 主区 ---- */
.body {
  flex: 1;
  overflow: hidden;
  /* 框架色：主区圆角外露的部分透出它，形成面板嵌入感 */
  background: var(--bg-frame);
}

.aside {
  background: var(--bg-frame);
  transition: width 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  overflow: hidden;
}

.menu-wrap {
  height: 100%;
  position: relative;
  padding: 8px 0;
}

/* 共享指示条：胶囊短条，贴菜单项左缘（项缘 x=4，缩进 6px），切换时滑动 */
.indicator {
  position: absolute;
  left: 10px;
  top: 0;
  width: 3px;
  border-radius: 2px;
  background: var(--accent);
  transition:
    transform 0.25s cubic-bezier(0.4, 0, 0.2, 1),
    height 0.25s cubic-bezier(0.4, 0, 0.2, 1);
  pointer-events: none;
}

.menu {
  border-right: none;
  background: transparent;
  padding: 0 4px;
}

.menu :deep(.el-menu-item) {
  height: 40px;
  border-radius: 6px;
  margin-bottom: 2px;
  color: var(--text-2);
  display: flex;
  align-items: center;
  /* !important：Element 对菜单项的 padding 特异性更高，普通写法压不住 */
  padding: 0 16px !important;
  white-space: nowrap;
  transition:
    padding 0.25s cubic-bezier(0.4, 0, 0.2, 1),
    color 0.12s ease,
    background 0.12s ease;
}

.menu :deep(.el-menu-item .el-icon) {
  width: 24px;
  height: 24px;
  justify-content: center;
  margin-right: 12px;
  flex-shrink: 0;
  transition: margin-right 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}

.menu :deep(.el-menu-item span) {
  display: inline-block; /* 必须：inline 的 width 无效，折叠时文字会撑住空间挤偏图标 */
  transition: opacity 0.2s ease;
  overflow: hidden;
}

/* 折叠态：图标居中、文字淡出让位，全部连续过渡 */
.aside.collapsed .menu :deep(.el-menu-item) {
  padding: 0 !important;
  justify-content: center;
}

.aside.collapsed .menu :deep(.el-menu-item .el-icon) {
  margin-right: 0;
}

.aside.collapsed .menu :deep(.el-menu-item span) {
  opacity: 0;
  width: 0;
}

.menu :deep(.el-menu-item:hover) {
  background: var(--bg-raised);
}

/* 选中态：WinUI 式可见圆角背景块，无描边 */
.menu :deep(.el-menu-item.is-active) {
  background: rgba(255, 255, 255, 0.07);
  color: var(--text-1);
}

.main {
  background:
    radial-gradient(900px 600px at 75% -10%, rgba(83, 155, 245, 0.05), transparent),
    var(--bg-page);
  padding: 24px;
  overflow-y: auto;
  /* WinUI 式内容面板：与顶栏、侧栏连接处圆角过渡，无描边 */
  border-radius: 12px 12px 0 0;
}
</style>
