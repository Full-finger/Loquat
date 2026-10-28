<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { getStatus, type UiTab } from './api'
import { refreshExtTabs, useExtTabs } from './composables/ext-tabs'
import { BUILTIN_TABS } from './tabs'

const route = useRoute()
const router = useRouter()
const coreOk = ref<boolean | null>(null)
const collapsed = ref(false)

// 子页（路由 meta.sub）顶栏显示返回按钮；顶层选项卡不显示
const showBack = computed(() => route.meta.sub === true)

function goBack() {
  // SPA 内来路优先 back（保留列表页滚动位置）；直链进入时回父页
  if (window.history.state?.back) router.back()
  else router.push(String(route.meta.parent ?? '/dashboard'))
}

// 导航 = 内置（tabs.ts，单一声明点）+ 插件贡献（宿主聚合 TABS 声明，
// 见 docs/ext-ui.md）。共享状态：启停/安装后即时刷新，轮询兜底外部变更。
const extTabs = useExtTabs()
const tabs = computed<UiTab[]>(() => [...BUILTIN_TABS, ...extTabs.value])

// 图标按 element-plus 图标组件名解析（main.ts 已全局注册），缺省通用 Link
function iconOf(tab: UiTab): string {
  return tab.icon ?? 'Link'
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
// 侧栏长度变化会挪动激活项位置，指示条跟着重算；项增删带 250ms
// 过渡动画，收拢/展开结束后再校一次才是终位
watch(tabs, () => {
  moveIndicator()
  window.setTimeout(moveIndicator, 300)
})

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
  refreshExtTabs().finally(() => moveIndicator())
  // 状态 10s 一拍；tab 清单搭同一拍轮询，外部变更源最迟一个周期内出现
  timer = window.setInterval(() => {
    poll()
    refreshExtTabs()
  }, 10_000)
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
        <transition name="pop">
          <button v-if="showBack" class="fold-btn" title="返回" @click="goBack">
            <el-icon :size="16"><Back /></el-icon>
          </button>
        </transition>
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
            <!-- 列表过渡：tab 增删时淡出收拢/生长展开，下方项随布局自然上移 -->
            <transition-group name="tab">
              <el-menu-item v-for="t in tabs" :key="t.id" :index="t.path">
                <el-icon><component :is="iconOf(t)" /></el-icon>
                <span>{{ t.title }}</span>
              </el-menu-item>
            </transition-group>
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
  position: relative; /* top-center 的 absolute 锚点 */
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

/* 返回按钮进出场：宽度 + 间距展开，brand 随之平滑让位 */
.pop-enter-active,
.pop-leave-active {
  transition:
    width 0.2s ease,
    margin-left 0.2s ease,
    opacity 0.15s ease,
    transform 0.2s ease;
  overflow: hidden;
}

.pop-enter-from,
.pop-leave-to {
  width: 0;
  margin-left: -8px; /* 抵消 flex gap，总位移连续不跳变 */
  opacity: 0;
  transform: translateX(-6px);
}

.top-center {
  /* 钉在顶栏几何中心：左侧按钮增减、侧栏折叠都不带动搜索框位移；
     宽度随窗口收缩，左右各留约 240px 给品牌与状态，不重叠 */
  position: absolute;
  left: 50%;
  top: 50%;
  transform: translate(-50%, -50%);
  width: min(560px, calc(100% - 480px));
  pointer-events: none; /* 空隙区域放行给下层，搜索框自己恢复 */
}

.top-center .search-box {
  pointer-events: auto;
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
  margin-left: auto; /* top-center 脱离文档流后，靠它顶到最右 */
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

/* ---- tab 增删的列表过渡（transition-group name="tab"）----
   消失：淡出 + 高度 40→0 收拢，下方项随布局自然上移（下位上移）；
   出现：反向生长，透明度延迟 50ms——空间先长出来，内容再浮现。
   缓动与全站侧栏动画共用同一条曲线 */
.menu :deep(.el-menu-item.tab-leave-active) {
  overflow: hidden;
  transition:
    opacity 0.2s ease,
    height 0.25s cubic-bezier(0.4, 0, 0.2, 1),
    margin-bottom 0.25s cubic-bezier(0.4, 0, 0.2, 1),
    transform 0.2s ease;
}

.menu :deep(.el-menu-item.tab-leave-to) {
  opacity: 0;
  transform: translateX(-10px);
  height: 0;
  margin-bottom: 0;
}

.menu :deep(.el-menu-item.tab-enter-active) {
  overflow: hidden;
  transition-property: transform, height, margin-bottom, opacity;
  transition-duration: 0.25s, 0.25s, 0.25s, 0.2s;
  transition-timing-function:
    cubic-bezier(0.4, 0, 0.2, 1),
    cubic-bezier(0.4, 0, 0.2, 1),
    cubic-bezier(0.4, 0, 0.2, 1),
    ease;
  transition-delay: 0s, 0s, 0s, 0.05s;
}

.menu :deep(.el-menu-item.tab-enter-from) {
  opacity: 0;
  transform: translateY(-6px);
  height: 0;
  margin-bottom: 0;
}

/* 列表重排（中间项被移除）时其余项平滑滑动 */
.menu :deep(.el-menu-item.tab-move) {
  transition: transform 0.25s cubic-bezier(0.4, 0, 0.2, 1);
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
