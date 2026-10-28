<script setup lang="ts">
import { onMounted, onUnmounted, reactive, ref, watch } from 'vue'
import { getStatus, type Status } from '../api'

// worker 队列容量，对齐 src/stats.rs 的 WORKER_QUEUE_CAP
const QUEUE_CAP = 64

const status = ref<Status | null>(null)
const error = ref('')
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

watch(status, (s) => {
  if (!s) return
  tween('events', s.events)
  tween('replies', s.replies)
})

async function refresh() {
  try {
    status.value = await getStatus()
    error.value = ''
  } catch (e) {
    error.value = String(e)
  }
}

function coreSub(): string {
  const d = status.value?.core_detail as { adapter_pid?: number } | undefined
  return d?.adapter_pid ? `适配器 PID ${d.adapter_pid}` : '等待适配器上线'
}

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
  <div>
    <div class="page-head">
      <div>
        <h2>仪表盘</h2>
        <div class="desc">核心、业务与管线的实时状态，10 秒自动刷新</div>
      </div>
      <el-button size="small" @click="refresh">刷新</el-button>
    </div>

    <el-alert
      v-if="error"
      :title="`请求失败：${error}`"
      type="error"
      show-icon
      :closable="false"
      style="margin-bottom: 16px"
    />

    <template v-if="status">
      <div class="stat-grid">
        <div class="stat-card">
          <div class="stat-label">核心</div>
          <div class="stat-value">{{ status.core }}</div>
          <div class="stat-sub">{{ coreSub() }}</div>
        </div>
        <div class="stat-card">
          <div class="stat-label">事件</div>
          <div class="stat-value">{{ display.events }}</div>
          <div class="stat-sub">运行期间累计接收</div>
        </div>
        <div class="stat-card">
          <div class="stat-label">回复</div>
          <div class="stat-value">{{ display.replies }}</div>
          <div class="stat-sub">成功送达的消息</div>
        </div>
        <div class="stat-card">
          <div class="stat-label">业务</div>
          <div class="stat-value">{{ (status.workers?.length ?? 0) + ' worker' }}</div>
          <div class="stat-sub">责任链按优先级顺序执行</div>
        </div>
      </div>

      <div class="panel">
        <div class="panel-title">worker 管线</div>
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
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
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
  width: 48px;
  text-align: right;
}
</style>
