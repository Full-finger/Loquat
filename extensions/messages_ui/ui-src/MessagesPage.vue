<script setup lang="ts">
// 「消息流」页（原内置 Messages.vue 迁移为插件）。WS 地址走宿主注入的
// ctx.wsUrl（token 查询参数兜底），重连策略与原页相同。
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import PageHead from '@webui/components/PageHead.vue'
import StateDot from '@webui/components/StateDot.vue'
import { useCtx } from '@webui/ext/mount'

interface MsgItem {
  time: string
  dir: 'in' | 'out'
  chat: string
  text: string
}

const ctx = useCtx()

const items = ref<MsgItem[]>([])
const connected = ref(false)
const filter = ref<'all' | 'in' | 'out'>('all')
const query = ref('')
const stick = ref(true)
const listEl = ref<HTMLElement>()

let ws: WebSocket | undefined
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

// 断线重连（与沙箱同策略）：监控页常开，2s 间隔
function connect() {
  if (disposed) return
  ws = new WebSocket(ctx.wsUrl())
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

onMounted(connect)

onUnmounted(() => {
  disposed = true
  ws?.close()
  if (reconnectTimer) clearTimeout(reconnectTimer)
})
</script>

<template>
  <div class="wrap">
    <PageHead>
      <template #actions>
        <StateDot :state="connected ? 'on' : 'off'">
          {{ connected ? '已连接' : '连接断开' }}
        </StateDot>
      </template>
    </PageHead>

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
  background: color-mix(in srgb, var(--accent) 15%, transparent);
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
