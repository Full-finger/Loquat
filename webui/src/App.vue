<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import type { UiTab } from './api'
import { refreshExtTabs, useExtTabs } from './composables/ext-tabs'
import { mergeTabs } from './tabs'

const route = useRoute()
const collapsed = ref(false)

// 窄屏外壳：侧栏变成抽屉（浮在内容上），不再挤压主区。
// 用 matchMedia 而不是纯 CSS，因为折叠/展开是两套完全不同的交互
const NARROW = '(max-width: 820px)'
const isNarrow = ref(false)
const drawerOpen = ref(false)
let mq: MediaQueryList | undefined

function syncNarrow(e: MediaQueryList | MediaQueryListEvent) {
  isNarrow.value = e.matches
  if (!e.matches) drawerOpen.value = false // 回到宽屏收起抽屉，避免看不见的遮罩残留
}

function toggleSide() {
  if (isNarrow.value) drawerOpen.value = !drawerOpen.value
  else collapsed.value = !collapsed.value
}

// 手机端跳转后收起抽屉，否则新页面被抽屉压住
watch(
  () => route.path,
  () => {
    if (isNarrow.value) drawerOpen.value = false
  },
)

// 导航 = 内置（tabs.ts，单一声明点）+ 插件贡献（宿主聚合 TABS 声明，
// 见 docs/ext-ui.md）。共享状态：启停/安装后即时刷新，轮询兜底外部变更。
const extTabs = useExtTabs()
const tabs = computed<UiTab[]>(() => mergeTabs(extTabs.value))
// WinUI 式分区：置底组（pin=bottom，如设置）独立渲染，钉在侧栏视口底部
const mainTabs = computed(() => tabs.value.filter((t) => t.pin !== 'bottom'))
const bottomTabs = computed(() => tabs.value.filter((t) => t.pin === 'bottom'))

// 图标按 element-plus 图标组件名解析（main.ts 已全局注册），缺省通用 Link
function iconOf(tab: UiTab): string {
  return tab.icon ?? 'Link'
}

// 侧栏激活与面包屑共用的匹配：/ext/<插件>/<tab>[/子路径] 都归到插件的
// 入口 tab。子路径（detail/<名>）没有自己的 TABS 声明，先按 base 拿，
// 拿不到就取该插件第一个 tab——插件内部导航不该灭掉侧栏高亮
function matchTab(p: string): UiTab | undefined {
  const m = p.match(/^\/ext\/([^/]+)(?:\/([^/]+))?/)
  if (!m) return tabs.value.find((t) => t.path === p)
  const [, plugin, base] = m
  return (
    tabs.value.find((t) => t.path === `/ext/${plugin}/${base}`) ??
    tabs.value.find((t) => t.path.startsWith(`/ext/${plugin}/`))
  )
}

const activeIndex = computed(() => matchTab(route.path)?.path ?? route.path)

// 面包屑：顶层页一级（即侧栏当前项）；子页两级「父页 > 实体名」，
// 父级可点回列表，当前级只是位置指示不可点
const crumbs = computed(() => {
  const name = route.params.name
  if (name && route.path.startsWith('/plugins/')) {
    return [
      { title: '插件', path: '/plugins' },
      { title: String(name) },
    ]
  }
  const tab = matchTab(route.path)
  return tab ? [{ title: tab.title }] : []
})

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

onMounted(() => {
  moveIndicator()
  refreshExtTabs().finally(() => moveIndicator())
  // tab 清单 10s 一拍轮询，外部变更源最迟一个周期内出现
  timer = window.setInterval(refreshExtTabs, 10_000)
  window.addEventListener('keydown', onKeydown)

  mq = window.matchMedia(NARROW)
  syncNarrow(mq)
  mq.addEventListener('change', syncNarrow)
})
onUnmounted(() => {
  clearInterval(timer)
  window.removeEventListener('keydown', onKeydown)
  mq?.removeEventListener('change', syncNarrow)
})
</script>

<template>
  <el-container class="layout">
    <el-header height="48px" class="topbar">
      <div class="top-left">
        <button
          class="fold-btn"
          :title="isNarrow ? '打开导航' : collapsed ? '展开侧栏' : '折叠侧栏'"
          @click="toggleSide"
        >
          <el-icon :size="16">
            <Expand v-if="isNarrow || collapsed" />
            <Fold v-else />
          </el-icon>
        </button>
        <nav class="crumbs" aria-label="面包屑">
          <template v-for="(c, i) in crumbs" :key="c.path ?? c.title">
            <span v-if="i" class="crumb-sep">/</span>
            <router-link v-if="c.path" :to="c.path" class="crumb link">{{ c.title }}</router-link>
            <span v-else class="crumb">{{ c.title }}</span>
          </template>
        </nav>
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

    </el-header>

    <el-container class="body">
      <!-- 窄屏抽屉遮罩：点空白处收起，顺带挡住下层滚动 -->
      <transition name="scrim">
        <div v-if="isNarrow && drawerOpen" class="scrim" @click="drawerOpen = false" />
      </transition>

      <el-aside
        :width="isNarrow ? '236px' : collapsed ? '64px' : '200px'"
        class="aside"
        :class="{ collapsed: collapsed && !isNarrow, mobile: isNarrow, open: drawerOpen }"
      >
        <div ref="menuEl" class="menu-wrap">
          <span
            v-show="indVisible"
            class="indicator"
            :style="{ transform: `translateY(${indY}px)`, height: `${indH}px` }"
          />
          <el-menu router :default-active="activeIndex" class="menu menu-main">
            <!-- 列表过渡：tab 增删时淡出收拢/生长展开，下方项随布局自然上移 -->
            <transition-group name="tab">
              <el-menu-item v-for="t in mainTabs" :key="t.id" :index="t.path">
                <el-icon><component :is="iconOf(t)" /></el-icon>
                <span>{{ t.title }}</span>
              </el-menu-item>
            </transition-group>
          </el-menu>
          <!-- 置底区：不参与主列表滚动，项再多也钉在侧栏底缘（WinUI Settings 同款） -->
          <el-menu router :default-active="activeIndex" class="menu menu-bottom">
            <transition-group name="tab">
              <el-menu-item
                v-for="t in bottomTabs"
                :key="t.id"
                :index="t.path"
                class="pinned"
              >
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
  /* dvh：移动端地址栏收起/软键盘弹出时跟着视口走，vh 兜底老浏览器 */
  height: 100vh;
  height: 100dvh;
}

/* ---- 顶部标题栏：左折叠+面包屑 / 中搜索 ---- */
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

/* 面包屑：父级可点，当前级加粗表示「你在这」；超长时各级独立截断。
   上限压着居中搜索框的左缘（窗口越窄给得越少），避免盖字 */
.crumbs {
  display: flex;
  align-items: center;
  gap: 4px;
  min-width: 0;
  max-width: clamp(132px, calc(50vw - 336px), 420px);
  font-size: 13px;
}

.crumb {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.crumb.link {
  color: var(--text-2);
  text-decoration: none;
  padding: 3px 6px;
  border-radius: 6px;
  transition: background 0.12s ease, color 0.12s ease;
}

.crumb.link:hover {
  background: var(--bg-raised);
  color: var(--text-1);
}

.crumb:last-child {
  color: var(--text-1);
  font-weight: 600;
}

.crumb-sep {
  color: var(--text-3);
  flex-shrink: 0;
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
  /* 钉在顶栏几何中心：左侧按钮增减、侧栏折叠都不带动搜索框位移；
     宽度随窗口收缩，右侧已无状态区，只预留面包屑的呼吸空间 */
  position: absolute;
  left: 50%;
  top: 50%;
  transform: translate(-50%, -50%);
  width: min(560px, calc(100% - 360px));
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
  /* border-box：height 含上下 padding，否则总高 100%+16px 溢出，
     底部留白被 aside 的 overflow:hidden 裁掉，置底项直接碰底 */
  box-sizing: border-box;
  height: 100%;
  display: flex;
  flex-direction: column;
  position: relative;
  padding: 8px 0;
}

/* 主列表吃掉剩余空间并自滚，置底区因此恒贴侧栏底缘；
   项少时设置也钉在底部，不跟主列表内容走 */
.menu-main {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
}

.menu-bottom {
  flex-shrink: 0;
}

/* 共享指示条：胶囊短条，贴菜单项左缘（项缘 x=4，缩进 6px），切换时滑动。
   回弹曲线：冲过目标再弹回，尾端有「扣上」的手感 */
.indicator {
  position: absolute;
  left: 10px;
  top: 0;
  width: 3px;
  border-radius: 2px;
  background: var(--accent);
  transition:
    transform 0.3s cubic-bezier(0.34, 1.56, 0.64, 1),
    height 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
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

/* 折叠态：只让文字让位，图标原地不动。
   图标水平起点 = 菜单内边距 4 + 项内边距 16 = 20（中心 32，与顶栏汉堡按钮同轴）。
   这个值和侧栏宽度无关，所以折叠态**必须保留** 16px 内边距并保持 flex-start 左对齐：
   若改成 padding:0 + justify-content:center，虽然端点仍回到 20，但居中在过渡第一帧
   就按尚未收缩的 200px 项宽重算一次位置（图标右移到 ~94），随后随宽度收缩被拉回 20，
   于是多出一段「先右移再左移」的抖动。文字靠 span 宽度归零让位即可。 */
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

/* 置底组（pin=bottom，如设置）：WinUI 式分隔线，与主列表隔开 */
.menu :deep(.el-menu-item.pinned:first-child) {
  position: relative;
}

.menu :deep(.el-menu-item.pinned:first-child)::before {
  content: '';
  position: absolute;
  top: -6px;
  left: 4px;
  right: 4px;
  height: 1px;
  background: var(--border-subtle);
}

/* 选中态：WinUI 式可见圆角背景块，无描边。用 text-1 派生：暗色是
   白纱、亮色是黑纱，同一公式两主题都成立 */
.menu :deep(.el-menu-item.is-active) {
  background: color-mix(in srgb, var(--text-1) 7%, transparent);
  color: var(--text-1);
}

/* ---- tab 增删的列表过渡（transition-group name="tab"）----
   只在两个量上动：高度、透明度。锚点固定在自身 layout 顶边，于是
   「出现 = 从顶边往下长开」「消失 = 底边往上收拢」，同轴、同锚、互为镜像。
   两个方向的 transform 都不要加，原因有二：
   1) 方向语义打架。原来是「离场左移 10px + 向上收」「入场从上往下落 6px」，
      同一个元素生与死走了两条不同的路；且 -6px 让新项前半程压在上一项
      （top 302 落在上一行 266–306 里）身上，读作「从上一行里挤出来」。
   2) 侧栏有效宽只有 192px（折叠态 56px），几像素的 X 偏移不足以读成位移动画，
      只会让图标文字错位，折叠态还会被 .aside 的 overflow:hidden 切掉半边。
   节奏是对称的「空间 / 内容」错拍：入场先长空间再浮现（opacity 延后 50ms），
   离场则内容先走完（0–200ms）空间再收拢（0–250ms），互为时间逆过程。
   缓动与全站侧栏动画共用同一条曲线。 */
.menu :deep(.el-menu-item.tab-leave-active) {
  overflow: hidden;
  transition:
    opacity 0.2s ease,
    height 0.25s cubic-bezier(0.4, 0, 0.2, 1),
    margin-bottom 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}

.menu :deep(.el-menu-item.tab-leave-to) {
  opacity: 0;
  height: 0;
  margin-bottom: 0;
}

.menu :deep(.el-menu-item.tab-enter-active) {
  overflow: hidden;
  transition:
    height 0.25s cubic-bezier(0.4, 0, 0.2, 1),
    margin-bottom 0.25s cubic-bezier(0.4, 0, 0.2, 1),
    opacity 0.2s ease 0.05s;
}

.menu :deep(.el-menu-item.tab-enter-from) {
  opacity: 0;
  height: 0;
  margin-bottom: 0;
}

/* 列表重排（中间项被移除）时其余项平滑滑动 */
.menu :deep(.el-menu-item.tab-move) {
  transition: transform 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}

.main {
  background:
    radial-gradient(900px 600px at 75% -10%, color-mix(in srgb, var(--accent) 5%, transparent), transparent),
    var(--bg-page);
  padding: 24px;
  overflow-y: auto;
  /* WinUI 式内容面板：与顶栏、侧栏连接处圆角过渡，无描边 */
  border-radius: 12px 12px 0 0;
}

/* ---- 窄屏（≤820px）：侧栏浮成抽屉，主区拿回整幅宽度 ----
   桌面端行为完全不变：折叠/展开仍是 200↔64 的宽度过渡 */
.scrim {
  position: fixed;
  inset: 48px 0 0;
  z-index: 20;
  background: rgba(0, 0, 0, 0.5);
  backdrop-filter: blur(1.5px);
}

.scrim-enter-active,
.scrim-leave-active {
  transition: opacity var(--dur) var(--ease-out);
}

.scrim-enter-from,
.scrim-leave-to {
  opacity: 0;
}

@media (max-width: 820px) {
  .aside.mobile {
    position: fixed;
    top: 48px;
    bottom: 0;
    left: 0;
    z-index: 30;
    transform: translateX(-100%);
    transition: transform var(--dur-slow) var(--ease-out);
    box-shadow: var(--shadow-3);
    background: var(--bg-frame);
  }

  .aside.mobile.open {
    transform: none;
  }

  /* 触屏目标放大到 44px，符合可点区域下限 */
  .aside.mobile .menu :deep(.el-menu-item) {
    height: 44px;
    font-size: 14px;
  }

  /* 顶栏三段式在窄屏放不下：搜索框让位，状态只留呼吸点 */
  .top-center {
    display: none;
  }

  .topbar {
    padding: 0 8px;
  }

  /* 窄屏搜索框已隐藏，面包屑放开占满剩余空间 */
  .crumbs {
    max-width: none;
  }

  .main {
    padding: 14px;
    border-radius: 12px;
  }
}

@media (max-width: 480px) {
  .main {
    padding: 10px;
  }
}
</style>
