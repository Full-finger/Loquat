<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { sandboxSend, USE_MOCK, wsUrl } from '../api'

interface Bubble {
  dir: 'in' | 'out'
  text: string
  time: string
  /** 群聊消息的会话标签（显示用） */
  chat: string
}

const bubbles = ref<Bubble[]>([])
const connected = ref(false)
const mode = ref<'private' | 'group'>('private')
const atBot = ref(true)
const input = ref('')
const sending = ref(false)
const listEl = ref<HTMLElement>()

let ws: WebSocket | undefined
// 卸载后置位：close 会触发 onclose，没有它就会僵尸重连（路由切换累积连接）
let disposed = false

function fmt(d: Date): string {
  return d.toTimeString().slice(0, 8)
}

function push(b: Bubble) {
  bubbles.value.push(b)
  if (bubbles.value.length > 200) bubbles.value.shift()
  nextTick(() => listEl.value?.scrollTo({ top: listEl.value.scrollHeight }))
}

// 消息流 WS 与「消息流」页共用：只取沙箱平台的帧。
// in = 用户注入的消息回显，out = 机器人的回复（宿主 publish 推送）。
function connect() {
  if (disposed) return
  ws = new WebSocket(wsUrl())
  ws.onopen = () => {
    connected.value = true
  }
  ws.onclose = () => {
    connected.value = false
    if (!disposed) setTimeout(connect, 2000) // 断线重连，沙箱常开
  }
  ws.onmessage = (ev) => {
    try {
      const d = JSON.parse(ev.data as string) as Record<string, unknown>
      // in 帧在 submit 里本地回显过了（注释同），这里只收机器人的 out 帧
      if (d.platform !== 'sandbox' || d.dir !== 'out') return
      push({
        dir: 'out',
        text: String(d.text ?? ''),
        time: String(d.time ?? fmt(new Date())),
        chat: String(d.chat ?? ''),
      })
    } catch {
      // 忽略非 JSON 帧
    }
  }
}

async function submit() {
  const text = input.value.trim()
  if (!text || sending.value) return
  sending.value = true
  try {
    await sandboxSend(text, mode.value, mode.value === 'group' && atBot.value)
    // 用户消息本地回显（WS 只推 out 帧，注入的 in 帧同样会到，
    // 去重麻烦，本地即时回显体验更好；WS 的 in 帧直接忽略）
    push({
      dir: 'in',
      text: (mode.value === 'group' && atBot.value ? '@sandbox-bot ' : '') + text,
      time: fmt(new Date()),
      chat: mode.value === 'group' ? '群 dev-group' : '好友 10000',
    })
    input.value = ''
  } catch (e) {
    push({ dir: 'out', text: `发送失败：${e}`, time: fmt(new Date()), chat: '' })
  } finally {
    sending.value = false
  }
}

// mock：开发模式下造两条对话
onMounted(() => {
  if (USE_MOCK) {
    connected.value = true
    push({ dir: 'in', text: '在吗', time: fmt(new Date()), chat: '好友 10000' })
    push({ dir: 'out', text: '在的，沙箱正常', time: fmt(new Date()), chat: '好友 10000' })
    return
  }
  connect()
})

onUnmounted(() => {
  disposed = true
  ws?.close()
})

watch(mode, (m) => {
  if (m === 'private') atBot.value = true
})
</script>

<template>
  <div class="wrap">
    <div class="page-head">
      <div>
        <h2>沙箱</h2>
        <div class="desc">与机器人直接对话，无需 NapCat；消息走与真实平台相同的链路</div>
      </div>
      <el-tag v-if="USE_MOCK || connected" type="success" effect="plain" size="small">
        已连接
      </el-tag>
      <el-tag v-else type="danger" effect="plain" size="small">连接断开</el-tag>
    </div>

    <div ref="listEl" class="chat panel">
      <el-empty
        v-if="bubbles.length === 0"
        description="发条消息试试"
        :image-size="72"
      />
      <div v-for="(b, i) in bubbles" :key="i" class="row" :class="b.dir">
        <div class="bubble" :class="b.dir">
          <div class="meta mono">{{ b.chat }} · {{ b.time }}</div>
          <div class="text">{{ b.text }}</div>
        </div>
      </div>
    </div>

    <div class="composer">
      <div class="opts">
        <el-radio-group v-model="mode" size="small">
          <el-radio-button value="private">私聊</el-radio-button>
          <el-radio-button value="group">群聊</el-radio-button>
        </el-radio-group>
        <el-checkbox v-if="mode === 'group'" v-model="atBot" size="small">at 机器人</el-checkbox>
        <span class="hint">以「开发者」（10000）身份发消息</span>
      </div>
      <div class="input-row">
        <el-input
          v-model="input"
          type="textarea"
          :rows="2"
          resize="none"
          placeholder="输入消息，Enter 发送，Shift+Enter 换行"
          @keydown.enter.exact.prevent="submit"
        />
        <el-button type="primary" :loading="sending" @click="submit">发送</el-button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.wrap {
  display: flex;
  flex-direction: column;
  height: calc(100vh - 208px);
}

.page-head {
  align-items: center;
}

.chat {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.row {
  display: flex;
}

.row.out {
  justify-content: flex-end;
}

.bubble {
  max-width: 68%;
  padding: 8px 12px;
  border-radius: 10px;
  font-size: 13px;
  line-height: 1.5;
}

.bubble.in {
  background: var(--bg-raised);
  border-bottom-left-radius: 3px;
}

.bubble.out {
  background: rgba(83, 155, 245, 0.18);
  border-bottom-right-radius: 3px;
}

.meta {
  font-size: 10px;
  color: var(--text-3);
  margin-bottom: 2px;
}

.text {
  color: var(--text-1);
  white-space: pre-wrap;
  word-break: break-word;
}

.composer {
  margin-top: 12px;
}

.opts {
  display: flex;
  align-items: center;
  gap: 14px;
  margin-bottom: 8px;
}

.hint {
  font-size: 12px;
  color: var(--text-3);
}

.input-row {
  display: flex;
  gap: 8px;
  align-items: flex-end;
}

.input-row .el-button {
  height: 54px;
}
</style>
