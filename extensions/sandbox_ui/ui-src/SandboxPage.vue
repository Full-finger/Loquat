<script setup lang="ts">
// 「沙箱」页（原内置 Sandbox.vue 迁移为插件）。会话 CRUD 走 ctx.api，
// WS 地址走 ctx.wsUrl；媒体 URL 的 token 查询参数逻辑随页面本地保留。
import { computed, nextTick, onMounted, onUnmounted, ref } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import type { SandboxRecord, SandboxSegment, SandboxSession } from '@webui/api'
import PageHead from '@webui/components/PageHead.vue'
import StateDot from '@webui/components/StateDot.vue'
import { useCtx } from '@webui/ext/mount'

const ctx = useCtx()

// ---- 会话/历史/发送：ctx.api 薄封装（原 webui api 的非 mock 分支）----

/** 历史里的图片是 /api/sandbox/media/x.png，<img> 带不了头，token 走查询参数。 */
function sandboxMediaUrl(path: string): string {
  if (!path.startsWith('/api/sandbox/media/')) return path
  const token = localStorage.getItem('loquat_token')
  return token ? `${path}?token=${encodeURIComponent(token)}` : path
}

async function listSandboxSessions(): Promise<SandboxSession[]> {
  const r = await ctx.api.get<{ sessions: SandboxSession[] }>('/api/sandbox/sessions')
  return r.sessions
}

async function createSandboxSession(mode: 'private' | 'group'): Promise<SandboxSession> {
  const r = await ctx.api.post<{ session: SandboxSession }>('/api/sandbox/sessions', { mode })
  return r.session
}

async function renameSandboxSession(chatId: string, name: string): Promise<SandboxSession> {
  const r = await ctx.api.patch<{ session: SandboxSession }>(
    `/api/sandbox/sessions/${encodeURIComponent(chatId)}`,
    { name },
  )
  return r.session
}

async function deleteSandboxSession(chatId: string): Promise<void> {
  await ctx.api.del(`/api/sandbox/sessions/${encodeURIComponent(chatId)}`)
}

async function fetchSandboxHistory(chatId: string, limit = 200): Promise<SandboxRecord[]> {
  const q = new URLSearchParams({ session: chatId, limit: String(limit) })
  const r = await ctx.api.get<{ messages: SandboxRecord[] }>(`/api/sandbox/history?${q}`)
  return r.messages
}

/** 注入段列表进沙箱事件链路，返回注入消息 ID（detail 是 protojson 应答）。 */
async function sandboxSend(
  segments: SandboxSegment[],
  mode: string,
  atBot: boolean,
  session = '',
): Promise<string> {
  const r = await ctx.api.post<{ ok?: boolean; detail?: string }>('/api/sandbox/send', {
    segments,
    mode,
    atBot,
    session,
  })
  let detail: { messageId?: string; error?: string } = {}
  try {
    detail = JSON.parse(r.detail ?? '{}')
  } catch {
    // 空/非 JSON 应答按无 ID 处理
  }
  if (detail.error) throw new Error(detail.error)
  return detail.messageId ?? ''
}

// 气泡存结构化段（protojson 形态）：文本、图片、回复引用都能原样渲染，
// 与注入请求共用一套类型，发送 = 把段数组原样交给后端。
// 会话切换时整列替换为历史记录，发送/收帧只做增量。
interface Bubble {
  /** 稳定 key：WS 帧没有消息 ID，用本地序号兜底 */
  key: number
  /** in = 我（开发者 10000）发出的注入消息；out = 机器人回复 */
  dir: 'in' | 'out'
  /** 平台消息 ID：引用回复的锚点（out 帧没有） */
  id: string
  segs: SandboxSegment[]
  /** 毫秒时间戳：跨天分隔与时间显示都靠它（后端记录带的字段） */
  ts: number
  /** 乐观发送中：气泡先出现，回执到了再定稿 */
  pending?: boolean
  /** 发送失败原因（挂在气泡下方，失败消息留在原地不消失） */
  error?: string
}

interface PendingImage {
  name: string
  dataUrl: string
}

const bubbles = ref<Bubble[]>([])
const sessions = ref<SandboxSession[]>([])
const activeChat = ref('')
const connected = ref(false)
const atBot = ref(true)
const input = ref('')
const sending = ref(false)
const loadingHistory = ref(false)
const listEl = ref<HTMLElement>()
const taEl = ref<HTMLTextAreaElement>()

// 会话列：窄屏用抽屉、桌面常驻；新建菜单与改名行内编辑
const railOpen = ref(false)
const newOpen = ref(false)
const renaming = ref('')
const renameText = ref('')

// 待发图片与引用目标（参考 llama.cpp ChatForm：附件 chips + 发送前可移除）
const pendingImages = ref<PendingImage[]>([])
const replyTarget = ref<Bubble | undefined>()
const fileEl = ref<HTMLInputElement>()

// 滚动跟随：贴底才自动跟随新消息，用户上翻则挂起并亮出「回到底部」
const stick = ref(true)
const atTop = ref(true)
const unread = ref(0)

// 等待机器人回复：发出去到收到 out 帧之间显示打字指示
const awaiting = ref(false)
// 聚焦态：驱动输入卡片的高亮与阴影（原生 textarea 没有 wrapper 可挂样式）
const focused = ref(false)
const atId = 'sandbox-at-bot'

// 触摸设备没有 hover：点气泡才浮出操作条
const isTouch = window.matchMedia('(hover: none)').matches
const tappedKey = ref<number | null>(null)

const MAX_IMAGE_BYTES = 5 * 1024 * 1024
/** 等回复的最长亮灯时间：插件不回复时不能让打字动画一直转 */
const AWAIT_TIMEOUT = 15_000
const SUGGESTIONS = ['/ping', '/status', '帮我看下今天的排班']
const ACTIVE_KEY = 'loquat_sandbox_active'

let ws: WebSocket | undefined
let disposed = false
let seq = 0
let awaitTimer: number | undefined
let ro: ResizeObserver | undefined
// 自己触发的滚动截止时间（毫秒）：平滑滚动期间不把「还没到」误判成用户上翻
let selfScrollUntil = 0
// 历史请求序号：快速切会话时丢弃过期响应
let historySeq = 0
let listTimer: number | undefined

const activeSession = computed(() => sessions.value.find((s) => s.chatId === activeChat.value))
/** 模式是会话属性（决定 chat kind 与是否可 @），不再随时切换 */
const mode = computed<'private' | 'group'>(() => activeSession.value?.mode ?? 'private')
const sessionName = computed(() => activeSession.value?.name ?? '会话')
const hasContent = computed(() => input.value.trim().length > 0 || pendingImages.value.length > 0)
const online = computed(() => connected.value)

/** 跨天分隔：只有当天第一条消息带标签 */
const dayLabels = computed(() => {
  const map = new Map<number, string>()
  let prev = ''
  for (const b of bubbles.value) {
    const d = new Date(b.ts)
    const k = `${d.getFullYear()}-${d.getMonth()}-${d.getDate()}`
    if (k !== prev) {
      map.set(b.key, dayLabel(b.ts))
      prev = k
    }
  }
  return map
})

function dayLabel(ts: number): string {
  const d = new Date(ts)
  const now = new Date()
  const sameDay = (a: Date, b: Date) =>
    a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate()
  if (sameDay(d, now)) return '今天'
  const y = new Date(now)
  y.setDate(now.getDate() - 1)
  if (sameDay(d, y)) return '昨天'
  if (d.getFullYear() === now.getFullYear()) return `${d.getMonth() + 1} 月 ${d.getDate()} 日`
  return `${d.getFullYear()} 年 ${d.getMonth() + 1} 月 ${d.getDate()} 日`
}

function hms(ts: number): string {
  const d = new Date(ts)
  const p = (n: number) => String(n).padStart(2, '0')
  return `${p(d.getHours())}:${p(d.getMinutes())}:${p(d.getSeconds())}`
}

/** 会话列表里的相对时间：刚刚 / 12 分钟前 / 3 小时前 / 10-29 */
function relTime(ts: number): string {
  const diff = Date.now() - ts
  if (diff < 60_000) return '刚刚'
  if (diff < 3_600_000) return `${Math.floor(diff / 60_000)} 分钟前`
  if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)} 小时前`
  const d = new Date(ts)
  return `${d.getMonth() + 1}-${String(d.getDate()).padStart(2, '0')}`
}

// ---- 会话管理 ----

async function loadSessions(): Promise<void> {
  try {
    sessions.value = await listSandboxSessions()
  } catch (e) {
    ElMessage.error(`会话列表加载失败：${e}`)
  }
}

/** 外部变更（别的会话收到回复、别的标签页在聊）合到一拍里刷新 */
function scheduleListRefresh() {
  if (listTimer) clearTimeout(listTimer)
  listTimer = window.setTimeout(() => {
    listTimer = undefined
    loadSessions()
  }, 400)
}

async function openSession(chatId: string) {
  if (!chatId || chatId === activeChat.value) return
  activeChat.value = chatId
  localStorage.setItem(ACTIVE_KEY, chatId)
  // 切换会话等于换一段上下文：待发、引用、未读全部清掉
  pendingImages.value = []
  replyTarget.value = undefined
  tappedKey.value = null
  unread.value = 0
  stick.value = true
  await loadHistory(chatId)
}

async function loadHistory(chatId: string) {
  const mine = ++historySeq
  loadingHistory.value = true
  try {
    const recs = await fetchSandboxHistory(chatId)
    if (mine !== historySeq) return // 期间又切了会话，丢弃这次结果
    bubbles.value = recs.map(toBubble)
    nextTick(() => scrollToBottom('auto'))
  } catch (e) {
    if (mine === historySeq) ElMessage.error(`历史加载失败：${e}`)
  } finally {
    if (mine === historySeq) loadingHistory.value = false
  }
}

function toBubble(r: SandboxRecord): Bubble {
  return {
    key: ++seq,
    dir: r.dir,
    id: r.id ?? '',
    segs: r.segments?.length ? r.segments : r.text ? [{ text: { text: r.text } }] : [],
    ts: r.ts || Date.now(),
  }
}

async function createSession(m: 'private' | 'group') {
  newOpen.value = false
  try {
    const s = await createSandboxSession(m)
    await loadSessions()
    await openSession(s.chatId)
    railOpen.value = false
    nextTick(() => taEl.value?.focus())
  } catch (e) {
    ElMessage.error(`新建失败：${e}`)
  }
}

function startRename(s: SandboxSession) {
  renaming.value = s.chatId
  renameText.value = s.name
}

async function commitRename(s: SandboxSession) {
  const name = renameText.value.trim()
  renaming.value = ''
  if (!name || name === s.name) return
  try {
    const updated = await renameSandboxSession(s.chatId, name)
    const hit = sessions.value.find((x) => x.chatId === s.chatId)
    if (hit) hit.name = updated.name
  } catch (e) {
    ElMessage.error(`改名失败：${e}`)
  }
}

async function removeSession(s: SandboxSession) {
  try {
    await ElMessageBox.confirm(
      `「${s.name}」的记录和机器人对这段对话的记忆都会被删除，且不可恢复。`,
      '删除会话',
      { type: 'warning', confirmButtonText: '删除', cancelButtonText: '取消' },
    )
  } catch {
    return // 取消
  }
  try {
    await deleteSandboxSession(s.chatId)
    const wasActive = s.chatId === activeChat.value
    await loadSessions()
    if (!wasActive) return
    const next = sessions.value[0]
    if (next) {
      activeChat.value = '' // 强制走一次完整加载
      await openSession(next.chatId)
    } else {
      activeChat.value = ''
      bubbles.value = []
    }
  } catch (e) {
    ElMessage.error(`删除失败：${e}`)
  }
}

// ---- 消息流 ----

function followUp() {
  nextTick(() => {
    if (stick.value) scrollToBottom('smooth')
    else unread.value++
  })
}

function push(b: Omit<Bubble, 'key'>): Bubble {
  bubbles.value.push({ ...b, key: ++seq })
  if (bubbles.value.length > 500) bubbles.value.shift()
  const created = bubbles.value[bubbles.value.length - 1]
  followUp()
  return created
}

function scrollToBottom(behavior: ScrollBehavior = 'smooth') {
  const el = listEl.value
  if (!el) return
  stick.value = true
  atTop.value = false
  unread.value = 0
  // 平滑滚动期间浏览器会持续抛 scroll 事件，位置还没到底，onScroll 会
  // 误判成「用户上翻」把 stick 翻成 false（之后就再也不跟随了）。这段时间内忽略。
  selfScrollUntil = performance.now() + (behavior === 'auto' ? 0 : 700)
  el.scrollTo({ top: el.scrollHeight, behavior })
}

function onScroll() {
  const el = listEl.value
  if (!el) return
  atTop.value = el.scrollTop <= 4
  if (performance.now() < selfScrollUntil) return
  stick.value = el.scrollHeight - el.scrollTop - el.clientHeight <= 48
  if (stick.value) unread.value = 0
}

/** 段列表的纯文本摘要（meta 行与回复条用）。 */
function segsText(segs: SandboxSegment[] | undefined): string {
  const parts: string[] = []
  for (const s of segs ?? []) {
    if (s.text?.text) parts.push(s.text.text)
    else if (s.image) parts.push('[图片]')
    else if (s.at) parts.push(`@${s.at.target === 'sandbox-bot' ? 'sandbox-bot' : s.at.target}`)
    else if (s.reply) parts.push('[回复]')
    else if (s.face) parts.push('[表情]')
  }
  return parts.join(' ')
}

/** 被回复气泡的文本摘要：在本地历史里找 ID。 */
function replyPreview(messageId: string): string {
  const hit = bubbles.value.find((b) => b.id === messageId)
  return hit ? segsText(hit.segs) : `消息 #${messageId}`
}

// 消息流 WS 与「消息流」页共用：只取沙箱平台的 out 帧（宿主已记账）。
function connect() {
  if (disposed) return
  ws = new WebSocket(ctx.wsUrl())
  ws.onopen = () => {
    connected.value = true
  }
  ws.onclose = () => {
    connected.value = false
    if (!disposed) window.setTimeout(connect, 2000) // 断线重连，沙箱常开
  }
  ws.onmessage = (ev) => {
    try {
      const d = JSON.parse(ev.data as string) as Record<string, unknown>
      if (d.platform !== 'sandbox' || d.dir !== 'out') return
      const cid = String(d.chat_id ?? '')
      // 别的会话的回复：不进当前列表，但列表的条数/排序变了
      if (cid && cid !== activeChat.value) {
        scheduleListRefresh()
        return
      }
      settleReply()
      push({
        dir: 'out',
        id: '',
        segs: [{ text: { text: String(d.text ?? '') } }],
        ts: Date.now(),
      })
      scheduleListRefresh()
    } catch {
      // 忽略非 JSON 帧
    }
  }
}

/** 收到回复：撤掉打字指示与超时定时器。 */
function settleReply() {
  awaiting.value = false
  if (awaitTimer) {
    clearTimeout(awaitTimer)
    awaitTimer = undefined
  }
}

function startAwait() {
  settleReply()
  awaiting.value = true
  awaitTimer = window.setTimeout(() => {
    awaiting.value = false
    awaitTimer = undefined
  }, AWAIT_TIMEOUT)
}

// ---- 输入区 ----

function addFiles(files: FileList | File[] | undefined) {
  for (const f of files ?? []) {
    if (!f.type.startsWith('image/')) continue
    if (f.size > MAX_IMAGE_BYTES) {
      ElMessage.warning(`图片 ${f.name} 超过 5MB，已跳过`)
      continue
    }
    const reader = new FileReader()
    reader.onload = () => {
      pendingImages.value.push({ name: f.name, dataUrl: String(reader.result) })
    }
    reader.readAsDataURL(f)
  }
}

function onPaste(e: ClipboardEvent) {
  addFiles(e.clipboardData?.files)
}

/** 选完文件立刻清空 value：同一张图连选两次也要能触发 change。 */
function onFileChange(e: Event) {
  const el = e.target as HTMLInputElement
  addFiles(el.files ?? undefined)
  el.value = ''
}

/** 点气泡上的「回复」：设引用目标，再点一次或发完取消。 */
function toggleReply(b: Bubble) {
  replyTarget.value = replyTarget.value?.key === b.key ? undefined : b
}

/** 触摸端：点整条气泡切换操作条显隐（桌面端交给 hover，这里直接跳过）。 */
function onRowTap(b: Bubble) {
  if (!isTouch) return
  tappedKey.value = tappedKey.value === b.key ? null : b.key
}

async function copyBubble(b: Bubble) {
  const text = segsText(b.segs)
  if (!text) return
  try {
    await navigator.clipboard.writeText(text)
    ElMessage.success('已复制')
  } catch {
    ElMessage.warning('浏览器拒绝了剪贴板访问')
  }
}

function useSuggestion(s: string) {
  input.value = s
  resize()
  nextTick(() => taEl.value?.focus())
}

/** 单行起步、随内容长高，超过 5 行内部滚动（ChatGPT 式输入区）。 */
function resize() {
  const el = taEl.value
  if (!el) return
  el.style.height = 'auto'
  el.style.height = `${Math.min(el.scrollHeight, 132)}px`
}

function resetComposer() {
  input.value = ''
  pendingImages.value = []
  replyTarget.value = undefined
  nextTick(resize)
}

async function submit() {
  if (sending.value) return
  if (!activeChat.value) {
    ElMessage.warning('先新建一个会话')
    return
  }
  const text = input.value.trim()
  if (!text && pendingImages.value.length === 0) return
  // 段顺序贴近真实消息形态：回复 → 图片 → 文本（at 由后端按 atBot 插最前）
  const segs: SandboxSegment[] = []
  if (replyTarget.value) segs.push({ reply: { messageId: replyTarget.value.id } })
  for (const img of pendingImages.value) segs.push({ image: { url: img.dataUrl, filename: img.name } })
  if (text) segs.push({ text: { text } })

  // 乐观回显：先出气泡再等回执，避免「点了发送界面毫无反应」
  const bubble = push({
    dir: 'in',
    id: '',
    segs,
    ts: Date.now(),
    pending: true,
  })
  resetComposer()

  sending.value = true
  try {
    const id = await sandboxSend(
      segs,
      mode.value,
      mode.value === 'group' && atBot.value,
      activeChat.value,
    )
    bubble.id = id
    bubble.pending = false
    startAwait()
    // 首条消息会让宿主把「新会话」改成前几个字，条数也变了
    scheduleListRefresh()
  } catch (e) {
    bubble.pending = false
    bubble.error = String(e)
  } finally {
    sending.value = false
  }
}

// ---- 新建菜单点空白关闭 ----
function onDocClick() {
  if (newOpen.value) newOpen.value = false
}

onMounted(async () => {
  if (listEl.value) {
    ro = new ResizeObserver(() => {
      if (stick.value) scrollToBottom('auto')
    })
    ro.observe(listEl.value)
  }
  document.addEventListener('click', onDocClick)

  await loadSessions()
  const saved = localStorage.getItem(ACTIVE_KEY)
  const pick = sessions.value.find((s) => s.chatId === saved) ?? sessions.value[0]
  if (pick) {
    activeChat.value = '' // 保证 openSession 走完整加载
    await openSession(pick.chatId)
  }

  connect()
})

onUnmounted(() => {
  disposed = true
  ws?.close()
  ro?.disconnect()
  document.removeEventListener('click', onDocClick)
  if (awaitTimer) clearTimeout(awaitTimer)
  if (listTimer) clearTimeout(listTimer)
})
</script>

<template>
  <div class="wrap">
    <PageHead>
      <template #actions>
        <StateDot :state="online ? 'on' : 'off'">
          {{ online ? '已连接' : '连接断开' }}
        </StateDot>
      </template>
    </PageHead>

    <section class="shell" :class="{ 'rail-open': railOpen }">
      <!-- 会话列：桌面常驻，窄屏抽屉 -->
      <aside class="rail">
        <div class="rail-head">
          <span class="rail-title">会话</span>
          <div class="new-wrap">
            <button class="new-btn" title="新建会话" @click.stop="newOpen = !newOpen">
              <el-icon :size="15"><Plus /></el-icon>
            </button>
            <transition name="pop">
              <div v-if="newOpen" class="new-menu" @click.stop>
                <button class="new-item" @click="createSession('private')">
                  <el-icon :size="13"><User /></el-icon> 私聊会话
                </button>
                <button class="new-item" @click="createSession('group')">
                  <el-icon :size="13"><UserFilled /></el-icon> 群聊会话
                </button>
              </div>
            </transition>
          </div>
        </div>

        <div class="rail-list">
          <div
            v-for="s in sessions"
            :key="s.chatId"
            class="sess"
            :class="{ on: s.chatId === activeChat }"
            @click="openSession(s.chatId)"
          >
            <span class="sess-badge" :class="s.mode">{{ s.mode === 'group' ? '群' : '私' }}</span>
            <span class="sess-body">
              <input
                v-if="renaming === s.chatId"
                v-model="renameText"
                class="rename"
                maxlength="40"
                @click.stop
                @keydown.enter.prevent="commitRename(s)"
                @keydown.esc="renaming = ''"
                @blur="commitRename(s)"
              />
              <span v-else class="sess-name" :title="s.name">{{ s.name }}</span>
              <span class="sess-meta mono">{{ s.count }} 条 · {{ relTime(s.updated) }}</span>
            </span>
            <span class="sess-tools">
              <button class="sess-btn" title="改名" @click.stop="startRename(s)">
                <el-icon :size="12"><EditPen /></el-icon>
              </button>
              <button class="sess-btn" title="删除" @click.stop="removeSession(s)">
                <el-icon :size="12"><Delete /></el-icon>
              </button>
            </span>
          </div>
          <p v-if="!sessions.length" class="rail-empty">还没有会话，点右上角 + 新建</p>
        </div>
      </aside>

      <transition name="fade">
        <div v-if="railOpen" class="rail-scrim" @click="railOpen = false" />
      </transition>

      <div class="main-col">
        <header class="shell-bar">
          <button class="rail-toggle" title="会话列表" @click="railOpen = !railOpen">
            <el-icon :size="15"><Memo /></el-icon>
          </button>
          <span class="peer">
            <span class="peer-avatar">
              <svg class="spark" viewBox="0 0 24 24" aria-hidden="true">
                <path d="M12 1.6C12 8 7.9 12.1 1.5 12.1 7.9 12.1 12 16.2 12 22.6 12 16.2 16.1 12.1 22.5 12.1 16.1 12.1 12 8 12 1.6Z" />
              </svg>
            </span>
            <span class="peer-name">sandbox-bot</span>
          </span>
          <span class="peer-sep" />
          <span class="peer-session">{{ sessionName }}</span>
          <span class="mode-chip">{{ mode === 'group' ? '群聊' : '私聊' }}</span>
          <span v-if="loadingHistory" class="loading mono">载入中</span>
        </header>

        <div class="stream-wrap" :class="{ 'at-top': atTop }">
          <div ref="listEl" class="stream" @scroll="onScroll">
            <!-- 空状态：图标 + 一句话 + 可点示例 -->
            <div v-if="!bubbles.length && !loadingHistory" class="empty">
              <span class="empty-orb"><el-icon :size="22"><ChatLineRound /></el-icon></span>
              <p class="empty-title">{{ activeChat ? '开始这段新对话' : '还没有会话' }}</p>
              <p class="empty-sub">
                {{
                  activeChat
                    ? '每个会话有独立的机器人记忆；支持发图、引用回复与群聊 @'
                    : '点左上角 + 新建一个会话'
                }}
              </p>
              <div v-if="activeChat" class="chips">
                <button v-for="s in SUGGESTIONS" :key="s" class="chip" @click="useSuggestion(s)">
                  {{ s }}
                </button>
              </div>
            </div>

            <transition-group v-else name="msg" tag="div" class="rows">
              <div v-for="b in bubbles" :key="b.key" class="item">
                <div v-if="dayLabels.get(b.key)" class="day">
                  <span>{{ dayLabels.get(b.key) }}</span>
                </div>
                <div
                  class="row"
                  :class="[
                    b.dir,
                    { pending: b.pending, failed: !!b.error, revealed: tappedKey === b.key },
                  ]"
                  @click="onRowTap(b)"
                >
                  <span class="avatar" :class="b.dir">
                    <el-icon v-if="b.dir === 'in'" :size="14"><User /></el-icon>
                    <svg v-else class="spark" viewBox="0 0 24 24" aria-hidden="true">
                      <path d="M12 1.6C12 8 7.9 12.1 1.5 12.1 7.9 12.1 12 16.2 12 22.6 12 16.2 16.1 12.1 22.5 12.1 16.1 12.1 12 8 12 1.6Z" />
                    </svg>
                  </span>

                  <div class="col">
                    <div class="byline">
                      <span class="name">{{ b.dir === 'in' ? '我' : 'sandbox-bot' }}</span>
                      <span class="byline-dot">·</span>
                      <span class="time mono">{{ hms(b.ts) }}</span>
                      <span v-if="b.pending" class="sending">发送中</span>
                    </div>

                    <div class="bubble" :class="b.dir">
                      <!-- 引用条：贴在消息正文上方，样式同主流 IM -->
                      <template v-for="(s, j) in b.segs" :key="j">
                        <div v-if="s.reply" class="reply-quote">
                          <el-icon :size="11"><ChatDotRound /></el-icon>
                          <span class="reply-text">{{ replyPreview(s.reply.messageId) }}</span>
                        </div>
                        <el-image
                          v-if="s.image?.url"
                          :src="sandboxMediaUrl(s.image.url)"
                          :preview-src-list="[sandboxMediaUrl(s.image.url)]"
                          :preview-teleported="true"
                          :alt="s.image.filename"
                          fit="cover"
                          class="pic"
                        />
                        <div v-else-if="s.text?.text" class="text">{{ s.text.text }}</div>
                        <div v-else-if="s.at" class="text at">@{{ s.at.target }}</div>
                        <div v-else-if="s.face" class="text face">[表情 {{ s.face.faceId }}]</div>
                      </template>
                    </div>

                    <div v-if="b.error" class="err">
                      <el-icon :size="12"><WarningFilled /></el-icon>
                      发送失败：{{ b.error }}
                    </div>

                    <!-- 悬停操作条：贴气泡外侧，不占排版空间 -->
                    <div class="acts">
                      <button v-if="!b.pending && b.id" class="act" @click="toggleReply(b)">
                        {{ replyTarget?.key === b.key ? '取消' : '回复' }}
                      </button>
                      <button class="act" @click="copyBubble(b)">复制</button>
                    </div>
                  </div>
                </div>
              </div>
            </transition-group>

            <!-- 等待回复：三点呼吸，比转圈更贴近「对方在打字」 -->
            <transition name="msg">
              <div v-if="awaiting" class="row out typing">
                <span class="avatar out">
                  <svg class="spark" viewBox="0 0 24 24" aria-hidden="true">
                    <path d="M12 1.6C12 8 7.9 12.1 1.5 12.1 7.9 12.1 12 16.2 12 22.6 12 16.2 16.1 12.1 22.5 12.1 16.1 12.1 12 8 12 1.6Z" />
                  </svg>
                </span>
                <div class="dots" aria-label="机器人正在回复">
                  <span /><span /><span />
                </div>
              </div>
            </transition>
          </div>

          <transition name="fab">
            <button v-if="!stick" class="jump" title="回到最新" @click="scrollToBottom()">
              <el-icon :size="14"><ArrowDown /></el-icon>
              <span v-if="unread" class="badge">{{ unread > 99 ? '99+' : unread }}</span>
            </button>
          </transition>
        </div>

        <div class="dock">
          <!-- 引用条 -->
          <transition name="collapse">
            <div v-if="replyTarget" class="quote-bar">
              <span class="quote-icon"><el-icon :size="12"><ChatDotRound /></el-icon></span>
              <span class="quote-text">回复：{{ segsText(replyTarget.segs) || '(空)' }}</span>
              <button class="quote-x" title="取消引用" @click="replyTarget = undefined">×</button>
            </div>
          </transition>

          <!-- 待发图片预览（发送前可移除） -->
          <transition-group v-if="pendingImages.length" name="thumb" tag="div" class="thumbs">
            <div v-for="(img, i) in pendingImages" :key="img.dataUrl" class="thumb">
              <img :src="img.dataUrl" :alt="img.name" />
              <button class="thumb-x" title="移除" @click="pendingImages.splice(i, 1)">×</button>
            </div>
          </transition-group>

          <div class="toolbar">
            <transition name="collapse">
              <label v-if="mode === 'group'" class="at-chip" :for="atId">
                <input :id="atId" v-model="atBot" type="checkbox" />
                <span class="at-box"><el-icon :size="10"><Select /></el-icon></span>
                at 机器人
              </label>
            </transition>
            <span class="hint">以「开发者」（10000）身份发送 · 可粘贴截图</span>
          </div>

          <div class="composer" :class="{ focus: focused, filled: hasContent }">
            <textarea
              ref="taEl"
              v-model="input"
              class="ta"
              rows="1"
              aria-label="消息输入框"
              placeholder="发消息给 sandbox-bot…"
              @input="resize"
              @focus="focused = true"
              @blur="focused = false"
              @keydown.enter.exact.prevent="submit"
              @keydown.esc="replyTarget = undefined"
              @paste="onPaste"
            />
            <div class="composer-side">
              <!-- 隐藏的文件选择：附件按钮触发；llama.cpp ChatFormInputFileInputInvisible 同思路 -->
              <input
                ref="fileEl"
                type="file"
                accept="image/*"
                multiple
                hidden
                @change="onFileChange"
              />
              <button class="icon-btn" title="发图（也可直接粘贴）" @click="fileEl?.click()">
                <el-icon :size="16"><Picture /></el-icon>
              </button>
              <button
                class="send"
                :disabled="!hasContent || sending"
                :title="sending ? '发送中' : '发送'"
                @click="submit"
              >
                <el-icon v-if="!sending" :size="16"><Promotion /></el-icon>
                <el-icon v-else :size="16" class="spin"><Loading /></el-icon>
              </button>
            </div>
          </div>

          <p class="foot">Enter 发送 · Shift + Enter 换行 · Esc 取消引用</p>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.wrap {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 380px;
}

/* ---- 外壳：左会话列 + 右对话列 ---- */
.shell {
  position: relative;
  flex: 1;
  min-height: 0;
  display: flex;
  border: 1px solid var(--border-subtle);
  border-radius: var(--r-md);
  background: var(--surface-1);
  overflow: hidden;
  box-shadow: var(--shadow-1);
  animation: rise 0.4s var(--ease-out) 0.05s backwards;
}

@keyframes rise {
  from {
    opacity: 0;
    transform: translateY(8px);
  }
}

.rail {
  width: 216px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  min-height: 0;
  border-right: 1px solid var(--border-subtle);
  background: linear-gradient(180deg, color-mix(in srgb, var(--text-1) 2%, transparent), transparent);
}

.rail-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 40px;
  padding: 0 8px 0 12px;
  border-bottom: 1px solid var(--border-subtle);
  flex-shrink: 0;
}

.rail-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-2);
  letter-spacing: 0.02em;
}

.new-wrap {
  position: relative;
}

.new-btn {
  width: 26px;
  height: 26px;
  display: grid;
  place-items: center;
  border-radius: 8px;
  border: 1px solid var(--border-subtle);
  background: var(--surface-2);
  color: var(--text-2);
  cursor: pointer;
  transition:
    color var(--dur-fast) var(--ease-out),
    background var(--dur-fast) var(--ease-out),
    transform var(--dur) var(--ease-spring);
}

.new-btn:hover {
  color: var(--text-1);
  background: var(--surface-3);
  transform: translateY(-1px);
}

.new-menu {
  position: absolute;
  top: 32px;
  right: 0;
  z-index: 8;
  width: 132px;
  padding: 4px;
  border-radius: 10px;
  border: 1px solid var(--border);
  background: var(--surface-3);
  box-shadow: var(--shadow-3);
}

.new-item {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 7px 9px;
  border: none;
  border-radius: 7px;
  background: transparent;
  color: var(--text-1);
  font-size: 12.5px;
  cursor: pointer;
  transition: background var(--dur-fast) var(--ease-out);
}

.new-item:hover {
  background: color-mix(in srgb, var(--accent) 18%, transparent);
}

.pop-enter-active,
.pop-leave-active {
  transition:
    opacity var(--dur-fast) var(--ease-out),
    transform var(--dur) var(--ease-spring);
  transform-origin: top right;
}

.pop-enter-from,
.pop-leave-to {
  opacity: 0;
  transform: scale(0.94) translateY(-4px);
}

.rail-list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 6px;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.sess {
  position: relative;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 7px 8px;
  border-radius: 8px;
  cursor: pointer;
  transition: background var(--dur-fast) var(--ease-out);
}

.sess:hover {
  background: var(--surface-2);
}

.sess.on {
  background: color-mix(in srgb, var(--accent) 16%, transparent);
  box-shadow: inset 2px 0 0 var(--accent);
}

.sess-badge {
  width: 18px;
  height: 18px;
  flex-shrink: 0;
  display: grid;
  place-items: center;
  border-radius: 5px;
  font-size: 10px;
  font-weight: 600;
  color: var(--accent-text);
  background: color-mix(in srgb, var(--accent) 16%, transparent);
  border: 1px solid color-mix(in srgb, var(--accent) 28%, transparent);
}

.sess-badge.group {
  color: var(--pink-text);
  background: color-mix(in srgb, var(--pink) 16%, transparent);
  border-color: color-mix(in srgb, var(--pink) 30%, transparent);
}

.sess-body {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.sess-name {
  font-size: 12.5px;
  color: var(--text-1);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.sess.on .sess-name {
  font-weight: 600;
}

.sess-meta {
  font-size: 10px;
  color: var(--text-3);
}

.rename {
  width: 100%;
  border: 1px solid var(--accent);
  border-radius: 6px;
  background: var(--surface-1);
  color: var(--text-1);
  font: inherit;
  font-size: 12.5px;
  padding: 2px 5px;
  outline: none;
}

.sess-tools {
  display: flex;
  gap: 1px;
  opacity: 0;
  transition: opacity var(--dur-fast) var(--ease-out);
}

.sess:hover .sess-tools,
.sess.on .sess-tools {
  opacity: 1;
}

.sess-btn {
  width: 20px;
  height: 20px;
  display: grid;
  place-items: center;
  border: none;
  border-radius: 5px;
  background: transparent;
  color: var(--text-3);
  cursor: pointer;
  transition:
    color var(--dur-fast) var(--ease-out),
    background var(--dur-fast) var(--ease-out);
}

.sess-btn:hover {
  color: var(--text-1);
  background: var(--surface-3);
}

.rail-empty {
  margin: 16px 8px;
  font-size: 11.5px;
  line-height: 1.6;
  color: var(--text-3);
}

.rail-scrim {
  position: absolute;
  inset: 0;
  z-index: 5;
  background: rgba(0, 0, 0, 0.45);
  backdrop-filter: blur(1px);
}

.main-col {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.shell-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  height: 40px;
  padding: 0 12px;
  border-bottom: 1px solid var(--border-subtle);
  background: linear-gradient(180deg, color-mix(in srgb, var(--text-1) 2%, transparent), transparent);
  flex-shrink: 0;
}

.peer {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  min-width: 0;
}

.peer-avatar {
  width: 20px;
  height: 20px;
  border-radius: 6px;
  display: grid;
  place-items: center;
  color: var(--pink-text);
  background: linear-gradient(140deg, color-mix(in srgb, var(--pink) 45%, transparent), color-mix(in srgb, var(--accent) 28%, transparent));
  border: 1px solid color-mix(in srgb, var(--pink) 34%, transparent);
  flex-shrink: 0;
}

/* 四角星：纯填充路径，任何缩放都锐利 */
.spark {
  width: 11px;
  height: 11px;
  fill: currentColor;
  display: block;
}

.peer-name {
  font-size: 12.5px;
  font-weight: 600;
  color: var(--text-1);
  white-space: nowrap;
}

.peer-sep {
  width: 1px;
  height: 12px;
  background: var(--border);
  flex-shrink: 0;
}

.peer-session {
  font-size: 11.5px;
  color: var(--text-2);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.mode-chip {
  flex-shrink: 0;
  font-size: 10px;
  padding: 1px 6px;
  border-radius: var(--r-pill);
  color: var(--text-3);
  border: 1px solid var(--border-subtle);
}

.loading {
  margin-left: auto;
  font-size: 10.5px;
  color: var(--text-3);
  animation: blink 1.2s var(--ease-out) infinite;
}

.rail-toggle {
  display: none;
  width: 26px;
  height: 26px;
  place-items: center;
  border-radius: 7px;
  border: 1px solid var(--border-subtle);
  background: var(--surface-2);
  color: var(--text-2);
  cursor: pointer;
}

/* ---- 消息流 ---- */
.stream-wrap {
  position: relative;
  flex: 1;
  min-height: 0;
}

.stream-wrap::before {
  content: '';
  position: absolute;
  inset: 0 0 auto;
  height: 26px;
  z-index: 2;
  pointer-events: none;
  background: linear-gradient(var(--surface-1) 12%, transparent);
  opacity: 1;
  transition: opacity var(--dur) var(--ease-out);
}

.stream-wrap.at-top::before {
  opacity: 0;
}

.stream {
  position: absolute;
  inset: 0;
  overflow-y: auto;
  overscroll-behavior: contain;
  padding: 16px max(18px, calc((100% - 900px) / 2)) 12px;
  display: flex;
  flex-direction: column;
  gap: 2px;
  scrollbar-gutter: stable;
}

/* 贴底占位：用 auto 外边距把内容压到底部，而不是 justify-content: flex-end
   —— 后者在内容溢出时会让顶部消息滚不到（已知的弹性容器溢出问题） */
.stream::before {
  content: '';
  margin-top: auto;
  flex: 0 0 auto;
}

/* ---- 空状态 ---- */
.empty {
  margin: auto;
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  gap: 6px;
  padding: 24px 12px;
  animation: rise 0.45s var(--ease-out) both;
}

.empty-orb {
  width: 46px;
  height: 46px;
  border-radius: 14px;
  display: grid;
  place-items: center;
  color: var(--accent-text);
  background: radial-gradient(circle at 30% 25%, color-mix(in srgb, var(--accent) 34%, transparent), color-mix(in srgb, var(--pink) 16%, transparent));
  border: 1px solid color-mix(in srgb, var(--accent) 28%, transparent);
  margin-bottom: 4px;
  box-shadow: 0 8px 24px -12px color-mix(in srgb, var(--accent) 60%, transparent);
}

.empty-title {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
  color: var(--text-1);
}

.empty-sub {
  margin: 0;
  font-size: 12px;
  color: var(--text-3);
  max-width: 320px;
  line-height: 1.6;
  /* 中文短句允许均衡换行，避免「@」单独掉一行 */
  text-wrap: balance;
}

.chips {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 8px;
  margin-top: 10px;
}

.chip {
  border: 1px solid var(--border-subtle);
  background: var(--surface-2);
  color: var(--text-2);
  font-size: 12px;
  padding: 6px 12px;
  border-radius: var(--r-pill);
  cursor: pointer;
  transition:
    color var(--dur-fast) var(--ease-out),
    border-color var(--dur-fast) var(--ease-out),
    transform var(--dur) var(--ease-spring),
    background var(--dur-fast) var(--ease-out);
}

.chip:hover {
  color: var(--text-1);
  border-color: var(--border-strong);
  background: var(--surface-3);
  transform: translateY(-1px);
}

.chip:active {
  transform: translateY(0) scale(0.98);
}

/* ---- 单条消息 ---- */
.rows {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

/* 跨天分隔条 */
.day {
  display: flex;
  align-items: center;
  justify-content: center;
  margin: 2px 0 10px;
}

.day span {
  font-size: 10.5px;
  color: var(--text-3);
  padding: 2px 10px;
  border-radius: var(--r-pill);
  background: var(--surface-2);
  border: 1px solid var(--border-subtle);
}

.row {
  display: flex;
  gap: 10px;
  align-items: flex-start;
  max-width: 100%;
}

/* 我发的贴右：头像在气泡右侧（镜像），与 ChatGPT/Claude 的会话方向一致 */
.row.in {
  flex-direction: row-reverse;
}

.col {
  display: flex;
  flex-direction: column;
  min-width: 0;
  max-width: min(72%, 660px);
}

.row.in .col {
  align-items: flex-end;
}

.avatar {
  width: 28px;
  height: 28px;
  border-radius: 9px;
  display: grid;
  place-items: center;
  flex-shrink: 0;
  margin-top: 18px; /* 与气泡首行对齐，让出 byline 高度 */
  transition: transform var(--dur) var(--ease-spring);
}

.avatar.in {
  color: #dce9ff;
  background: linear-gradient(140deg, var(--accent), color-mix(in srgb, var(--accent) 72%, black));
  box-shadow: 0 4px 12px -6px color-mix(in srgb, var(--accent) 90%, transparent);
}

.avatar.out {
  color: #d5c6f5;
  background: linear-gradient(140deg, var(--pink), color-mix(in srgb, var(--pink) 70%, black));
  border: 1px solid color-mix(in srgb, var(--pink) 30%, transparent);
}

.avatar .spark {
  width: 14px;
  height: 14px;
}

.row:hover .avatar {
  transform: translateY(-1px) scale(1.04);
}

.byline {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 10.5px;
  color: var(--text-3);
  margin-bottom: 4px;
  opacity: 0.9;
  padding: 0 2px;
}

.row.in .byline {
  justify-content: flex-end;
}

.name {
  font-weight: 600;
  color: var(--text-2);
}

.byline-dot {
  opacity: 0.6;
}

.time {
  font-size: 10px;
}

.sending {
  color: var(--accent);
  animation: blink 1.1s var(--ease-out) infinite;
}

@keyframes blink {
  50% {
    opacity: 0.4;
  }
}

.bubble {
  position: relative;
  padding: 9px 13px;
  border-radius: var(--r-lg);
  font-size: 13px;
  line-height: 1.62;
  max-width: 100%;
  transition:
    background var(--dur) var(--ease-out),
    border-color var(--dur) var(--ease-out),
    box-shadow var(--dur) var(--ease-out);
}

.bubble.out {
  background: var(--surface-2);
  border: 1px solid var(--border-subtle);
  border-top-left-radius: 5px;
  color: var(--text-1);
}

.bubble.in {
  background: linear-gradient(180deg, color-mix(in srgb, var(--accent) 22%, transparent), color-mix(in srgb, var(--accent) 13%, transparent));
  border: 1px solid color-mix(in srgb, var(--accent) 30%, transparent);
  border-top-right-radius: 5px;
  color: var(--accent-text);
}

.row:hover .bubble.out {
  border-color: var(--border);
}

.row:hover .bubble.in {
  border-color: color-mix(in srgb, var(--accent) 45%, transparent);
}

.row.pending .bubble {
  opacity: 0.62;
}

.row.failed .bubble.in {
  background: linear-gradient(180deg, rgba(229, 83, 75, 0.2), rgba(229, 83, 75, 0.12));
  border-color: rgba(229, 83, 75, 0.4);
}

.err {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  margin-top: 5px;
  font-size: 11.5px;
  color: var(--danger-text);
}

.reply-quote {
  display: flex;
  align-items: center;
  gap: 6px;
  margin: 0 0 6px;
  padding: 4px 8px;
  border-left: 2px solid currentColor;
  font-size: 11px;
  color: var(--text-2);
  background: color-mix(in srgb, var(--text-1) 10%, transparent);
  border-radius: 6px;
  max-width: 340px;
  opacity: 0.92;
}

.reply-text {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.pic {
  width: 200px;
  max-height: 168px;
  border-radius: 12px;
  margin: 2px 0;
  display: block;
  cursor: zoom-in;
  transition: transform var(--dur) var(--ease-spring), box-shadow var(--dur) var(--ease-out);
}

.pic:hover {
  transform: scale(1.012);
  box-shadow: var(--shadow-2);
}

.text {
  color: inherit;
  white-space: pre-wrap;
  word-break: break-word;
}

.text.at {
  color: var(--accent-text);
  font-weight: 500;
}

.text.face {
  color: var(--text-2);
}

/* 悬停操作条：浮在气泡下方，默认不可见且不吃点击 */
.acts {
  display: flex;
  gap: 2px;
  margin-top: 4px;
  height: 20px;
  opacity: 0;
  transform: translateY(-2px);
  transition:
    opacity var(--dur-fast) var(--ease-out),
    transform var(--dur-fast) var(--ease-out);
  pointer-events: none;
}

.row:hover .acts,
.row:focus-within .acts {
  opacity: 1;
  transform: none;
  pointer-events: auto;
}

.act {
  border: none;
  background: transparent;
  color: var(--text-3);
  font-size: 11px;
  padding: 2px 7px;
  border-radius: 6px;
  cursor: pointer;
  transition: color var(--dur-fast) var(--ease-out), background var(--dur-fast) var(--ease-out);
}

.act:hover {
  color: var(--text-1);
  background: var(--surface-3);
}

/* 触摸设备没有 hover：操作条先收起（不占行高），点气泡后展开 */
@media (hover: none) {
  .acts {
    display: none;
  }

  .row.revealed .acts {
    display: flex;
    opacity: 1;
    transform: none;
    pointer-events: auto;
    animation: popin var(--dur) var(--ease-spring);
  }

  @keyframes popin {
    from {
      opacity: 0;
      transform: translateY(-3px);
    }
  }
}

/* ---- 打字指示 ---- */
.typing {
  margin-top: 12px;
}

.typing .avatar {
  margin-top: 0;
}

.dots {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  height: 34px;
  padding: 0 14px;
  border-radius: var(--r-lg);
  border-top-left-radius: 5px;
  background: var(--surface-2);
  border: 1px solid var(--border-subtle);
}

.dots span {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: var(--text-3);
  animation: bounce 1.25s var(--ease-out) infinite;
}

.dots span:nth-child(2) {
  animation-delay: 0.16s;
}

.dots span:nth-child(3) {
  animation-delay: 0.32s;
}

@keyframes bounce {
  0%,
  70%,
  100% {
    transform: translateY(0);
    opacity: 0.45;
  }
  35% {
    transform: translateY(-4px);
    opacity: 1;
  }
}

/* ---- 新消息入场 ---- */
.msg-enter-active {
  transition:
    opacity var(--dur-slow) var(--ease-out),
    transform var(--dur-slow) var(--ease-out);
}

.msg-enter-from {
  opacity: 0;
  transform: translateY(10px) scale(0.985);
}

.msg-leave-active {
  transition: opacity 0.15s ease;
  position: absolute;
}

.msg-leave-to {
  opacity: 0;
}

/* ---- 回到底部 ---- */
.jump {
  position: absolute;
  right: 18px;
  bottom: 14px;
  z-index: 3;
  width: 32px;
  height: 32px;
  border-radius: 50%;
  display: grid;
  place-items: center;
  border: 1px solid var(--border);
  background: var(--surface-3);
  color: var(--text-1);
  cursor: pointer;
  box-shadow: var(--shadow-2);
  transition:
    transform var(--dur) var(--ease-spring),
    background var(--dur-fast) var(--ease-out);
}

.jump:hover {
  transform: translateY(-2px);
  background: var(--border);
}

.badge {
  position: absolute;
  top: -6px;
  right: -6px;
  min-width: 16px;
  height: 16px;
  padding: 0 4px;
  border-radius: var(--r-pill);
  background: var(--accent);
  color: #fff;
  font-size: 10px;
  line-height: 16px;
  text-align: center;
  font-weight: 600;
  box-shadow: 0 0 0 2px var(--surface-1);
}

.fab-enter-active,
.fab-leave-active {
  transition:
    opacity var(--dur) var(--ease-out),
    transform var(--dur) var(--ease-spring);
}

.fab-enter-from,
.fab-leave-to {
  opacity: 0;
  transform: translateY(6px) scale(0.9);
}

/* ---- 输入区 ---- */
.dock {
  flex-shrink: 0;
  padding: 10px max(14px, calc((100% - 900px) / 2)) 12px;
  border-top: 1px solid var(--border-subtle);
  background: var(--bg-raised);
}

.quote-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  margin-bottom: 8px;
  border-left: 2px solid var(--accent);
  background: var(--surface-2);
  border-radius: 8px;
  font-size: 12px;
  color: var(--text-2);
}

.quote-icon {
  color: var(--accent);
  display: inline-flex;
}

.quote-text {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.quote-x,
.thumb-x {
  border: none;
  background: transparent;
  color: var(--text-3);
  cursor: pointer;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  line-height: 1;
  font-size: 14px;
  display: grid;
  place-items: center;
  transition: color var(--dur-fast) var(--ease-out), background var(--dur-fast) var(--ease-out);
}

.quote-x:hover {
  color: var(--text-1);
  background: var(--surface-3);
}

.thumbs {
  display: flex;
  gap: 8px;
  margin-bottom: 8px;
  flex-wrap: wrap;
}

.thumb {
  position: relative;
  width: 60px;
  height: 60px;
  border-radius: 10px;
  overflow: hidden;
  border: 1px solid var(--border-subtle);
}

.thumb img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
}

.thumb-x {
  position: absolute;
  top: 3px;
  right: 3px;
  background: rgba(0, 0, 0, 0.6);
  color: #fff;
  backdrop-filter: blur(4px);
}

.thumb-x:hover {
  background: rgba(0, 0, 0, 0.85);
  color: #fff;
}

.thumb-enter-active {
  transition:
    opacity var(--dur) var(--ease-out),
    transform var(--dur) var(--ease-spring);
}

.thumb-enter-from {
  opacity: 0;
  transform: scale(0.85);
}

.thumb-leave-active {
  transition:
    opacity 0.15s ease,
    transform 0.15s ease;
  position: absolute;
}

.thumb-leave-to {
  opacity: 0;
  transform: scale(0.9);
}

/* 工具条：at 芯片 + 说明 */
.toolbar {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 8px;
  min-height: 26px;
}

.at-chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--text-2);
  padding: 3px 10px 3px 4px;
  border-radius: var(--r-pill);
  border: 1px solid var(--border-subtle);
  background: var(--surface-2);
  cursor: pointer;
  user-select: none;
  transition: border-color var(--dur-fast) var(--ease-out), color var(--dur-fast) var(--ease-out);
}

.at-chip:hover {
  border-color: var(--border);
  color: var(--text-1);
}

.at-chip input {
  position: absolute;
  opacity: 0;
  pointer-events: none;
}

.at-box {
  width: 15px;
  height: 15px;
  border-radius: 5px;
  border: 1px solid var(--border);
  display: grid;
  place-items: center;
  color: transparent;
  background: var(--surface-1);
  transition:
    background var(--dur-fast) var(--ease-out),
    border-color var(--dur-fast) var(--ease-out),
    color var(--dur-fast) var(--ease-out);
}

.at-chip input:checked + .at-box {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
}

.at-chip input:focus-visible + .at-box {
  box-shadow: var(--ring);
}

.hint {
  font-size: 11.5px;
  color: var(--text-3);
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* 输入卡片：整体是一个圆角容器，聚焦时描边点亮 + 外发光 */
.composer {
  display: flex;
  align-items: flex-end;
  gap: 8px;
  padding: 8px 8px 8px 12px;
  border-radius: var(--r-md);
  border: 1px solid var(--border-subtle);
  background: var(--surface-2);
  transition:
    border-color var(--dur) var(--ease-out),
    box-shadow var(--dur) var(--ease-out),
    background var(--dur) var(--ease-out);
}

.composer:hover {
  border-color: var(--border);
}

.composer.focus {
  border-color: color-mix(in srgb, var(--accent) 60%, transparent);
  box-shadow: var(--ring);
  background: var(--surface-1);
}

.ta {
  flex: 1;
  min-width: 0;
  border: none;
  outline: none;
  resize: none;
  background: transparent;
  color: var(--text-1);
  font: inherit;
  font-size: 13px;
  line-height: 1.62;
  padding: 6px 0;
  max-height: 132px;
  overflow-y: auto;
}

.ta::placeholder {
  color: var(--text-3);
}

.composer-side {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}

.icon-btn {
  width: 32px;
  height: 32px;
  border-radius: 9px;
  display: grid;
  place-items: center;
  border: 1px solid transparent;
  background: transparent;
  color: var(--text-2);
  cursor: pointer;
  transition:
    color var(--dur-fast) var(--ease-out),
    background var(--dur-fast) var(--ease-out),
    transform var(--dur) var(--ease-spring);
}

.icon-btn:hover {
  color: var(--text-1);
  background: var(--surface-3);
  transform: translateY(-1px);
}

.send {
  width: 32px;
  height: 32px;
  border-radius: 10px;
  display: grid;
  place-items: center;
  border: 1px solid color-mix(in srgb, var(--accent) 50%, transparent);
  background: linear-gradient(180deg, var(--accent), color-mix(in srgb, var(--accent) 70%, black));
  color: #fff;
  cursor: pointer;
  transition:
    transform var(--dur) var(--ease-spring),
    box-shadow var(--dur) var(--ease-out),
    filter var(--dur-fast) var(--ease-out),
    opacity var(--dur-fast) var(--ease-out);
}

.send:hover:not(:disabled) {
  transform: translateY(-1px) scale(1.04);
  box-shadow: 0 6px 16px -6px color-mix(in srgb, var(--accent) 90%, transparent);
}

.send:active:not(:disabled) {
  transform: scale(0.94);
}

.send:disabled {
  cursor: default;
  opacity: 0.38;
  filter: saturate(0.4);
}

.composer.filled .send:disabled {
  opacity: 0.7;
  filter: none;
}

.spin {
  animation: spin 0.9s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.foot {
  margin: 7px 0 0;
  font-size: 10.5px;
  color: var(--text-3);
  text-align: center;
  opacity: 0.75;
}

/* 引用条 / at 芯片的展开收起 */
.collapse-enter-active,
.collapse-leave-active {
  transition:
    opacity var(--dur) var(--ease-out),
    transform var(--dur) var(--ease-out);
}

.collapse-enter-from,
.collapse-leave-to {
  opacity: 0;
  transform: translateY(4px);
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity var(--dur) var(--ease-out);
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

/* ---- 响应式 ---- */
@media (max-width: 1024px) {
  .col {
    max-width: 80%;
  }
  .hint {
    display: none;
  }
  .rail {
    width: 186px;
  }
}

@media (max-width: 820px) {
  /* 会话列变抽屉：浮在对话之上，点遮罩关闭 */
  .rail {
    position: absolute;
    top: 0;
    bottom: 0;
    left: 0;
    z-index: 6;
    width: 246px;
    transform: translateX(-100%);
    transition: transform var(--dur-slow) var(--ease-out);
    box-shadow: var(--shadow-3);
    background: var(--surface-1);
  }

  .shell.rail-open .rail {
    transform: none;
  }

  .rail-toggle {
    display: grid;
  }

  .peer-session {
    display: none;
  }

  .peer-sep {
    display: none;
  }

  .stream {
    padding: 12px 12px 6px;
  }

  .col {
    max-width: 86%;
  }

  .bubble {
    font-size: 13.5px;
  }

  .dock {
    padding: 8px 10px calc(10px + env(safe-area-inset-bottom));
  }

  .foot {
    display: none;
  }
}

@media (max-width: 480px) {
  .col {
    max-width: 88%;
  }

  .avatar {
    width: 24px;
    height: 24px;
    border-radius: 8px;
  }


  .pic {
    width: 168px;
  }
}
</style>
