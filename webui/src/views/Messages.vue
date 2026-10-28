<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { USE_MOCK, wsUrl } from '../api'

interface MsgItem {
  time: string
  dir: 'in' | 'out'
  chat: string
  text: string
}

const items = ref<MsgItem[]>([])
const connected = ref(false)
const filter = ref<'all' | 'in' | 'out'>('all')
const query = ref('')
const stick = ref(true)
const listEl = ref<HTMLElement>()

let ws: WebSocket | undefined
let mockTimer: number | undefined
let disposed = false
let reconnectTimer: number | undefined

const filtered = computed(() =>
  items.value.filter(
    (m) =>
      (filter.value === 'all' || m.dir === filter.value) &&
      (query.value === '' ||
        m.chat.includes(query.value) ||
        m.text.toLowerCase().includes(query.value.toLowerCase())),
  ),
)

// 列表最新在顶（unshift）：「跟随」= 贴住顶部；用户下滚即暂停，滚回顶部恢复
function onScroll() {
  const el = listEl.value
  if (!el) return
  stick.value = el.scrollTop <= 48
}

watch(
  () => filtered.value.length,
  async () => {
    if (!stick.value) return
    await nextTick()
    listEl.value?.scrollTo({ top: 0 })
  },
)

function push(m: MsgItem) {
  items.value.unshift(m)
  if (items.value.length > 500) items.value.pop()
}

function fmt(d: Date): string {
  return d.toTimeString().slice(0, 8)
}

const MOCK_TEXTS = ['/ping', 'pong', '今天天气怎么样', '查到了，晴，26 度', '/status', '核心运行中']

onMounted(() => {
  if (USE_MOCK) {
    mockTimer = window.setInterval(() => {
      const dir = Math.random() > 0.5 ? 'in' : 'out'
      push({
        time: fmt(new Date()),
        dir,
        chat: Math.random() > 0.5 ? '群 12345' : '好友 67890',
        text: MOCK_TEXTS[Math.floor(Math.random() * MOCK_TEXTS.length)],
      })
    }, 2000)
  } else {
    connect()
  }
})

// 断线重连（与 Sandbox 同策略）：监控页常开，2s 间隔
function connect() {
  if (disposed) return
  ws = new WebSocket(wsUrl())
  ws.onopen = () => {
    connected.value = true
  }
  ws.onclose = () => {
    connected.value = false
    if (!disposed) reconnectTimer = window.setTimeout(connect, 2000)
  }
  ws.onmessage = (ev) => {
    try {
      const d = JSON.parse(ev.data as string) as Partial<MsgItem>
      push({
        time: d.time ?? fmt(new Date()),
        dir: d.dir === 'out' ? 'out' : 'in',
        chat: d.chat ?? '-',
        text: d.text ?? '',
      })
    } catch {
      // 忽略非 JSON 帧
    }
  }
}

onUnmounted(() => {
  disposed = true
  ws?.close()
  if (reconnectTimer) clearTimeout(reconnectTimer)
  if (mockTimer) clearInterval(mockTimer)
})
</script>

<template>
  <div class="wrap">
    <div class="page-head">
      <div>
        <h2>消息流</h2>
        <div class="desc">实时收发监控，最多保留 500 条</div>
      </div>
      <el-tag v-if="USE_MOCK || connected" type="success" effect="plain" size="small">
        已连接
      </el-tag>
      <el-tag v-else type="danger" effect="plain" size="small">连接断开</el-tag>
    </div>

    <div class="toolbar">
      <el-radio-group v-model="filter" size="small">
        <el-radio-button value="all">全部</el-radio-button>
        <el-radio-button value="in">收</el-radio-button>
        <el-radio-button value="out">发</el-radio-button>
      </el-radio-group>
      <el-input
        v-model="query"
        placeholder="搜会话或内容"
        size="small"
        clearable
        style="width: 220px"
      />
    </div>

    <div ref="listEl" class="msg-list panel" @scroll="onScroll">
      <el-empty v-if="filtered.length === 0" description="暂无消息" :image-size="72" />
      <div v-for="(m, i) in filtered" :key="i" class="msg-row">
        <span class="msg-time mono">{{ m.time }}</span>
        <span class="msg-dir" :class="m.dir">{{ m.dir === 'in' ? '收' : '发' }}</span>
        <span class="msg-chat mono">{{ m.chat }}</span>
        <span class="msg-text">{{ m.text }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.wrap {
  display: flex;
  flex-direction: column;
  /* 视口 - 顶栏48 - 页头/工具条/主区内边距 */
  height: calc(100vh - 208px);
}

.page-head {
  align-items: center;
}

.toolbar {
  display: flex;
  gap: 12px;
  align-items: center;
  margin-bottom: 12px;
}

.msg-list {
  flex: 1;
  overflow-y: auto;
  padding: 8px 0;
}

.msg-row {
  display: flex;
  align-items: baseline;
  gap: 12px;
  padding: 5px 16px;
  font-size: 13px;
  line-height: 1.5;
}

.msg-row:hover {
  background: var(--bg-raised);
}

.msg-time {
  color: var(--text-3);
  font-size: 12px;
  flex-shrink: 0;
}

.msg-dir {
  flex-shrink: 0;
  width: 18px;
  height: 18px;
  line-height: 18px;
  border-radius: 4px;
  text-align: center;
  font-size: 11px;
  align-self: center;
}

.msg-dir.in {
  color: var(--accent);
  background: rgba(83, 155, 245, 0.15);
}

.msg-dir.out {
  color: var(--pink);
  background: rgba(152, 110, 226, 0.15);
}

.msg-chat {
  color: var(--text-2);
  font-size: 12px;
  flex-shrink: 0;
  min-width: 90px;
}

.msg-text {
  color: var(--text-1);
  word-break: break-all;
}
</style>
