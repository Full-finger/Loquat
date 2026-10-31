<script setup lang="ts">
// 「仪表盘」页（原内置 Dashboard.vue 迁移为插件）。数据走 ctx.api 轮询。
import { computed, onMounted, onUnmounted, reactive, ref, watch } from 'vue'
import type { Status } from '@webui/api'
import { useResource } from '@webui/composables/use-resource'
import StateDot from '@webui/components/StateDot.vue'
import Donut from '@webui/components/Donut.vue'
import Sparkline from '@webui/components/Sparkline.vue'
import { useCtx } from '@webui/ext/mount'

const ctx = useCtx()

// worker 队列容量，对齐 src/stats.rs 的 WORKER_QUEUE_CAP
const QUEUE_CAP = 64
// 趋势采样上限：10s 一拍，60 点 ≈ 近 10 分钟
const SERIES_CAP = 60

// 错误内联展示（概览条下 alert），不 toast
const { data: status, error, refresh } = useResource<Status>(() => ctx.api.get<Status>('/api/status'), {
  toast: false,
})
let timer: number | undefined

// 大数字滚动动画：新值到达后从旧值 tween 过去；卸载时取消在途帧
const display = reactive({ events: 0, replies: 0 })
let rafId: number | undefined

function tween(key: 'events' | 'replies', to: number) {
  const from = display[key]
  if (from === to) return
  const start = performance.now()
  const duration = 400
  if (rafId) cancelAnimationFrame(rafId)
  function step(now: number) {
    const p = Math.min((now - start) / duration, 1)
    const eased = 1 - Math.pow(1 - p, 3) // easeOutCubic
    display[key] = Math.round(from + (to - from) * eased)
    if (p < 1) rafId = requestAnimationFrame(step)
  }
  rafId = requestAnimationFrame(step)
}

// 速率采样：每拍增量即该时间窗的吞吐，画近 N 拍趋势
const eventSeries = ref<number[]>([])
const replySeries = ref<number[]>([])

function push(series: number[], v: number) {
  series.push(v)
  if (series.length > SERIES_CAP) series.shift()
}

watch(status, (s, old) => {
  if (!s) return
  tween('events', s.events)
  tween('replies', s.replies)
  if (old) {
    push(eventSeries.value, Math.max(0, s.events - old.events))
    push(replySeries.value, Math.max(0, s.replies - old.replies))
  }
})

const adapters = computed(() => status.value?.core_detail?.adapters ?? [])
const aliveCount = computed(() => adapters.value.filter((a) => a.alive).length)
const dropped = computed(() => status.value?.core_detail?.events_dropped ?? 0)
const sys = computed(() => status.value?.sys)

function fmtUptime(sec: number): string {
  const d = Math.floor(sec / 86400)
  const h = Math.floor((sec % 86400) / 3600)
  const m = Math.floor((sec % 3600) / 60)
  if (d) return `${d} 天 ${h} 小时`
  if (h) return `${h} 小时 ${m} 分`
  return `${m} 分钟`
}

function fmtMem(bytes?: number): string {
  if (!bytes) return '—'
  const mb = bytes / 1024 ** 2
  return mb >= 1024 ? `${(mb / 1024).toFixed(1)} GB` : `${Math.round(mb)} MB`
}

// 延迟毫秒格式化：短耗时保小数，长耗时折算秒
function fmtMs(v?: number): string {
  if (v == null) return '—'
  if (v >= 1000) return `${(v / 1000).toFixed(1)}s`
  if (v >= 10) return `${Math.round(v)}ms`
  return v >= 1 ? `${v.toFixed(1)}ms` : `${v.toFixed(2)}ms`
}

// 分阶段延迟展示顺序（对齐 stats.rs 的三个记录点）
const LATENCY_STAGES: { key: 'parse' | 'chain' | 'send'; label: string }[] = [
  { key: 'parse', label: '解析' },
  { key: 'chain', label: '插件链' },
  { key: 'send', label: '发送' },
]

// 型号里的注册商标符号去掉，对齐 fastfetch 的干净排版
function cleanBrand(s: string): string {
  return s.replace(/\((?:R|TM|C)\)|®|™/gi, '').replace(/\s+/g, ' ').trim()
}

// fastfetch 风格的系统信息行：OS / CPU / GPU
const sysLines = computed<[string, string][]>(() => {
  const s = sys.value
  if (!s) return []
  const lines: [string, string][] = []
  const os = [s.os_long, s.arch].filter(Boolean).join(' ')
  if (os) lines.push(['OS', os])
  if (s.cpu_brand) {
    const freq = s.cpu_freq ? ` @ ${(s.cpu_freq / 1000).toFixed(3)}GHz` : ''
    lines.push(['CPU', `${cleanBrand(s.cpu_brand)} (${s.cpu_cores})${freq}`])
  }
  for (const g of s.gpus ?? []) lines.push(['GPU', g])
  return lines
})

onMounted(() => {
  refresh()
  timer = window.setInterval(refresh, 10_000)
})
onUnmounted(() => {
  clearInterval(timer)
  if (rafId) cancelAnimationFrame(rafId)
})
</script>

<template>
  <div class="dash">
    <el-alert
      v-if="error"
      :title="`请求失败：${error}`"
      type="error"
      show-icon
      :closable="false"
      style="margin-bottom: 16px"
    />

    <template v-if="status">
      <!-- 概览行：关键数字直接排版，无框 -->
      <div class="overview">
        <div class="ov-item">
          <div class="ov-label">核心</div>
          <div class="ov-value">{{ status.core }}</div>
          <div class="ov-sub">
            {{ status.uptime_sec ? `已运行 ${fmtUptime(status.uptime_sec)}` : '等待适配器上线' }}
          </div>
        </div>
        <div class="ov-item">
          <div class="ov-label">事件</div>
          <div class="ov-value" :class="{ danger: dropped > 0 }">{{ display.events }}</div>
          <div class="ov-sub" :class="{ danger: dropped > 0 }">
            {{ dropped > 0 ? `丢弃 ${dropped}（队列过载）` : '无丢弃' }}
          </div>
          <Sparkline :points="eventSeries" class="ov-spark" />
        </div>
        <div class="ov-item">
          <div class="ov-label">回复</div>
          <div class="ov-value">{{ display.replies }}</div>
          <div class="ov-sub">成功送达的消息</div>
          <Sparkline :points="replySeries" class="ov-spark" />
        </div>
        <div class="ov-item">
          <div class="ov-label">适配器</div>
          <div
            class="ov-value"
            :class="{ danger: adapters.length > 0 && aliveCount < adapters.length }"
          >
            {{ aliveCount }}/{{ adapters.length }} 在线
          </div>
          <div class="ov-sub">
            {{ adapters.length ? adapters.map((a) => a.platform).join(' · ') : '无运行记录' }}
          </div>
        </div>
      </div>

      <section class="sec">
        <h3 class="sec-title">适配器</h3>
        <div v-if="!adapters.length" class="muted">暂无运行记录，启用适配器后出现在这里</div>
        <div v-for="a in adapters" :key="a.platform" class="adapter-row">
          <StateDot :state="a.alive ? 'on' : 'off'" class="adapter-name">
            {{ a.platform }}
          </StateDot>
          <span class="mono adapter-meta">PID {{ a.pid }}</span>
          <span class="mono adapter-meta">CPU {{ (a.cpu ?? 0).toFixed(1) }}%</span>
          <span class="mono adapter-meta">{{ fmtMem(a.mem) }}</span>
        </div>
      </section>

      <section class="sec">
        <h3 class="sec-title">worker 管线</h3>
        <div v-for="(backlog, i) in status.workers ?? []" :key="i" class="pipe">
          <span class="mono pipe-label">W{{ i }}</span>
          <div class="pipe-track">
            <div
              class="pipe-fill"
              :class="{ hot: backlog > 0 }"
              :style="{ width: Math.max((backlog / QUEUE_CAP) * 100, 2) + '%' }"
            />
          </div>
          <span class="mono pipe-num">{{ backlog }}/{{ QUEUE_CAP }}</span>
          <span class="mono pipe-done">{{ status.workers_done?.[i] ?? 0 }} done</span>
        </div>
        <div class="pipe-meta mono">
          <span>主队列 {{ status.main_backlog ?? 0 }} 待处理</span>
          <span :class="{ 'drop-hot': (status.worker_dropped ?? 0) > 0 }">
            worker 丢弃 {{ status.worker_dropped ?? 0 }}
          </span>
          <span
            :class="{
                'drop-hot':
                  (status.outbound?.dropped ?? 0) > 0,
              }"
          >
            出站丢弃 {{ status.outbound?.dropped ?? 0 }} · 重试
            {{ status.outbound?.retries ?? 0 }}
          </span>
        </div>
        <div class="lat">
          <div v-for="s in LATENCY_STAGES" :key="s.key" class="lat-row mono">
            <span class="lat-label">{{ s.label }}</span>
            <span>p50 {{ fmtMs(status.latency?.[s.key]?.p50) }}</span>
            <span>p95 {{ fmtMs(status.latency?.[s.key]?.p95) }}</span>
            <span>p99 {{ fmtMs(status.latency?.[s.key]?.p99) }}</span>
            <span class="lat-n">{{ status.latency?.[s.key]?.n ?? 0 }} 样本</span>
          </div>
        </div>
      </section>

      <section class="sec">
        <h3 class="sec-title">资源占用</h3>
        <div class="res-row">
          <div class="rings">
            <Donut
              :percent="sys?.host_cpu ?? 0"
              :percent2="sys?.sys_cpu"
              :label="`${Math.round(sys?.host_cpu ?? 0)}%`"
              sub="宿主 CPU"
              :tip="`宿主进程 ${sys?.host_cpu?.toFixed(1) ?? '—'}% · 系统总占用 ${sys?.sys_cpu?.toFixed(1) ?? '—'}%`"
            />
            <Donut
              :percent="sys && sys.mem_total ? ((sys.host_mem ?? 0) / sys.mem_total) * 100 : 0"
              :percent2="sys && sys.mem_total ? ((sys.sys_mem_used ?? 0) / sys.mem_total) * 100 : undefined"
              :label="fmtMem(sys?.host_mem)"
              sub="宿主内存"
              :warn-at="80"
              :tip="`宿主进程 ${fmtMem(sys?.host_mem)} · 系统已用 ${fmtMem(sys?.sys_mem_used)}（共 ${fmtMem(sys?.mem_total)}）`"
            />
          </div>
          <div class="sys-info mono">
            <div v-for="[k, v] in sysLines" :key="k" class="sys-line">
              <span class="sys-key">{{ k }}:</span>
              {{ v }}
            </div>
          </div>
        </div>
      </section>

      <section class="sec">
        <h3 class="sec-title">活跃会话</h3>
        <div v-if="!status.active_chats?.length" class="muted">无积压中的会话</div>
        <div v-else class="chat-tags">
          <span v-for="c in status.active_chats" :key="c" class="mono chat-tag">{{ c }}</span>
        </div>
      </section>
    </template>
  </div>
</template>

<style scoped>
/* 无框分区：标题 + 内容，区间细分隔线，整列限宽居中 */
.dash {
  max-width: 960px;
  margin: 0 auto;
}

.sec {
  padding: 20px 0;
}

.sec + .sec {
  border-top: 1px solid var(--border-subtle);
}

.sec-title {
  margin: 0 0 14px;
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.08em;
  color: var(--text-3);
}

.muted {
  color: var(--text-3);
  font-size: 12px;
}

/* ---- 概览行 ---- */
.overview {
  display: flex;
  align-items: stretch;
  flex-wrap: wrap;
  padding-bottom: 20px;
}

.ov-item {
  flex: 1;
  min-width: 170px;
  padding: 2px 24px;
}

.ov-item:first-child {
  padding-left: 0;
}

.ov-item + .ov-item {
  border-left: 1px solid var(--border-subtle);
}

.ov-label {
  color: var(--text-3);
  font-size: 11px;
  letter-spacing: 0.08em;
  margin-bottom: 6px;
}

.ov-value {
  font-family: var(--mono);
  font-size: 22px;
  font-weight: 600;
  line-height: 1.1;
  color: var(--accent);
  white-space: nowrap;
}

.ov-value.danger {
  color: var(--danger);
}

.ov-sub {
  color: var(--text-3);
  font-size: 11px;
  margin-top: 6px;
}

.ov-sub.danger {
  color: var(--danger);
  font-weight: 500;
}

.ov-spark {
  width: 100%;
  max-width: 150px;
  margin-top: 8px;
}

/* ---- 适配器行 ---- */
.adapter-row {
  display: grid;
  grid-template-columns: 140px 1fr 1fr 1fr;
  gap: 12px;
  align-items: center;
  padding: 7px 0;
  font-size: 13px;
}

.adapter-row + .adapter-row {
  border-top: 1px solid var(--border-subtle);
}

.adapter-meta {
  color: var(--text-3);
  font-size: 12px;
}

/* ---- worker 管线 ---- */
.pipe {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 8px;
}

.pipe:last-child {
  margin-bottom: 0;
}

.pipe-label {
  color: var(--text-3);
  width: 24px;
  font-size: 12px;
}

.pipe-track {
  flex: 1;
  height: 6px;
  border-radius: 3px;
  background: var(--bg-frame);
  overflow: hidden;
}

.pipe-fill {
  height: 100%;
  border-radius: 3px;
  background: var(--border);
  transition: width 0.3s ease;
}

.pipe-fill.hot {
  background: var(--accent);
}

.pipe-num {
  color: var(--text-3);
  font-size: 12px;
  width: 56px;
  text-align: right;
}

.pipe-done {
  color: var(--text-3);
  font-size: 12px;
  width: 72px;
  text-align: right;
}

/* 主队列占用 + worker 丢弃，与积压条同列对齐 */
.pipe-meta {
  display: flex;
  justify-content: space-between;
  margin: 10px 0 14px;
  padding-top: 10px;
  border-top: 1px dashed var(--border-subtle);
  color: var(--text-3);
  font-size: 12px;
}

.drop-hot {
  color: var(--danger);
  font-weight: 500;
}

/* 分阶段延迟：标签 + 三个分位一列，右对齐样本数 */
.lat {
  display: grid;
  gap: 6px;
}

.lat-row {
  display: flex;
  align-items: baseline;
  gap: 18px;
  font-size: 12px;
  color: var(--text-2);
}

.lat-label {
  color: var(--text-3);
  width: 44px;
}

.lat-n {
  margin-left: auto;
  color: var(--text-3);
}

/* ---- 资源占用 ---- */
.res-row {
  display: flex;
  align-items: center;
  gap: 28px;
  flex-wrap: wrap;
  /* 容器查询锚点：按本行实际宽度切换窄屏布局（换行与否由宽度决定） */
  container-type: inline-size;
}

/* 圆环组在除系统信息外的剩余空间里居中 */
.rings {
  flex: 1;
  display: flex;
  justify-content: center;
  gap: 44px;
}

/* 窄到会换行时信息块独占一行：取消推右，整块居中（文字仍左对齐）。
   阈值 ≈ 圆环组最小宽 236 + 间距 28 + 信息块最长行 ~330 */
@container (max-width: 620px) {
  .sys-info {
    margin: 0 auto;
  }
}

.sys-info {
  margin-left: auto; /* 推到行的最右，圆环靠左 */
  color: var(--text-2);
  font-size: 12px;
  line-height: 2;
  min-width: 0;
  overflow: hidden;
  text-align: left;
}

.sys-key {
  color: var(--accent);
}

/* ---- 活跃会话 ---- */
.chat-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.chat-tag {
  font-size: 12px;
  color: var(--text-2);
  background: var(--bg-raised);
  border: 1px solid var(--border-subtle);
  border-radius: 6px;
  padding: 3px 8px;
}
</style>
