// 后端 API 封装（宿主壳专用）。
// mock 只在 dev 模式（npm run dev）且未设 VITE_USE_MOCK=false 时生效——
// 后端没起也能开发壳页；生产构建（npm run build）永远走真实后端。
// 插件页面不走这里的运行时函数（它们用注入的 ctx.api），但共享这里的
// 接口类型（type-only import，构建期擦除）：这些 interface 就是核心
// API 的契约文档，一处声明。
// 接口契约对齐 src/console.rs 的控制台命令。

/** 单阶段延迟分位（ms），n 为样本数 */
export interface StageLatency {
  p50?: number
  p95?: number
  p99?: number
  n: number
}

export interface Status {
  core: string
  /** dll.status() 的原始 JSON：started / adapters（含探测到的 cpu、mem）/ events_dropped */
  core_detail?: {
    started?: boolean
    adapters?: { platform: string; alive: boolean; pid: number; cpu?: number; mem?: number }[]
    events_dropped?: number
  }
  events: number
  replies: number
  /** 各 worker 积压（对齐 stats.rs 的 WORKER_COUNT / WORKER_QUEUE_CAP） */
  workers?: number[]
  /** 各 worker 累计处理完成数 */
  workers_done?: number[]
  /** 主循环待处理事件数（无界队列的占用观测，见 stats.rs） */
  main_backlog?: number
  /** worker 队列满被丢的消息数 */
  worker_dropped?: number
  /** 分阶段延迟（ms）：parse 解析信封 / chain 插件责任链 / send 出站发送 */
  latency?: Partial<Record<'parse' | 'chain' | 'send', StageLatency>>
  /** 积压中的会话（正在处理或排队） */
  active_chats?: string[]
  /** WebUI 已运行秒数 */
  uptime_sec?: number
  /** 宿主进程占用 + 硬件静态信息（mem 单位字节，cpu_freq 单位 MHz） */
  sys?: {
    os_long?: string
    arch?: string
    cpu_brand?: string
    cpu_cores?: number
    cpu_freq?: number
    gpus?: string[]
    mem_total?: number
    host_cpu?: number
    host_mem?: number
    /** 全局 CPU 占用 % 与全系统已用内存（字节） */
    sys_cpu?: number
    sys_mem_used?: number
  }
  business: string
}

export interface PluginInfo {
  name: string
  installed: boolean
  enabled: boolean
  priority: number
  /** 目录下有 ui.js（提供 WebUI 扩展页，可贡献侧栏 tab） */
  has_ui: boolean
}

export const USE_MOCK = import.meta.env.DEV && import.meta.env.VITE_USE_MOCK !== 'false'

const delay = (ms: number) => new Promise((r) => setTimeout(r, ms))

async function http<T>(path: string, init?: RequestInit): Promise<T> {
  // token 由 config/webui.yaml 配置后手动存入 localStorage（登录页二期）
  const token = localStorage.getItem('loquat_token') ?? ''
  const res = await fetch(path, {
    ...init,
    headers: {
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
      ...init?.headers,
    },
  })
  if (!res.ok) throw new Error(`${init?.method ?? 'GET'} ${path}: HTTP ${res.status}`)
  return res.json() as Promise<T>
}

const post = (path: string) => http<unknown>(path, { method: 'POST' })

async function postMsg(path: string): Promise<string> {
  const r = (await post(path)) as { msg?: string }
  return r.msg ?? '完成'
}

// ---- mock 数据 ----

const mockPlugins: PluginInfo[] = [
  { name: 'chat', installed: true, enabled: true, priority: 100, has_ui: false },
  { name: 'demo_ui', installed: true, enabled: true, priority: 100, has_ui: true },
  { name: 'qa', installed: true, enabled: true, priority: 100, has_ui: false },
  { name: 'status', installed: true, enabled: false, priority: 50, has_ui: false },
  { name: 'weather', installed: false, enabled: false, priority: 100, has_ui: false },
]

// ---- 插件管理（原生页唯一保留的内置 tab：启停插件的入口不能插件化）----

export async function listPlugins(): Promise<PluginInfo[]> {
  if (USE_MOCK) {
    await delay(150)
    return mockPlugins.map((p) => ({ ...p }))
  }
  return http('/api/plugins')
}

export async function setPluginEnabled(name: string, enabled: boolean): Promise<void> {
  if (USE_MOCK) {
    await delay(150)
    const p = mockPlugins.find((x) => x.name === name)
    if (p) p.enabled = enabled
    return
  }
  await post(`/api/plugins/${encodeURIComponent(name)}/${enabled ? 'enable' : 'disable'}`)
}

export async function installPlugin(name: string): Promise<string> {
  if (USE_MOCK) {
    await delay(300)
    const p = mockPlugins.find((x) => x.name === name)
    if (p) p.installed = true
    return `已生成 ${name}/config.yaml，默认未启用`
  }
  return postMsg(`/api/plugins/${encodeURIComponent(name)}/install`)
}

// 消息流 WS 地址（ctx.wsUrl 的实现源）。
// 浏览器 WebSocket API 不能带 Authorization 头，配置了 token 时走
// ?token= 查询参数兜底（宿主 api_ws 同步支持；token 会进访问日志）。
export function wsUrl(): string {
  const proto = location.protocol === 'https:' ? 'wss' : 'ws'
  const token = localStorage.getItem('loquat_token')
  const q = token ? `?token=${encodeURIComponent(token)}` : ''
  return `${proto}://${location.host}/api/ws${q}`
}

// ---- 适配器管理（页面在 extensions/adapters_ui，这里只留契约类型）----

/** 适配器启动定义：核心对平台零知识，差异全在 path 与 env 里。 */
export interface AdapterDef {
  name: string
  path?: string
  /** GET 载荷附带：解析后的启动文件（path 留空时按 bin/adapter-<name> 约定）。发回时核心忽略 */
  resolved_path?: string
  env?: Record<string, string>
  /** 省略即启用；停用保留定义与参数 */
  enabled?: boolean
}

/** bin/ 下发现的适配器二进制（发现流），可能尚未写入配置 */
export interface DiscoveredAdapter {
  name: string
  path: string
}

export interface AdapterConfig {
  adapters: AdapterDef[]
}

export interface AdapterState {
  platform: string
  alive: boolean
  pid: number
}

// ---- 服务配置（页面在 extensions/providers_ui，这里只留契约类型）----

export interface ProviderEntry {
  api_base?: string
  api_key?: string
  /** 后端标记：是否已配置 key（明文不出后端） */
  has_api_key?: boolean
  model?: string
  temperature?: number
  max_tokens?: number
  timeout_seconds?: number
  vision?: boolean
}

export interface ProviderSection {
  active: string
  providers: Record<string, ProviderEntry>
}

export interface ProvidersConfig {
  llm: ProviderSection
  search: ProviderSection
}

// ---- 插件配置编辑（SCHEMA 表单）----

export interface SchemaField {
  key: string
  /** string/secret/asset/int/float/bool/list/dict，缺省 string（校验渲染由 loquat_cfglib 支持） */
  type?: string
  default?: unknown
  desc?: string
  options?: unknown[]
  min?: number
  max?: number
  /** WebUI 渲染提示：分组标签，同名归组可折叠，未声明的排最前无标题 */
  group?: string
  /** WebUI 渲染提示：string 用多行 textarea（缺省单行输入框） */
  multiline?: boolean
  /** WebUI 渲染提示：values[when.key] ∈ when.oneOf 时才显示；隐藏字段的值原样保存 */
  when?: { key: string; oneOf: unknown[] }
}

export interface PluginConfig {
  schema: SchemaField[]
  values: Record<string, unknown>
  /** 已配置的 secret 字段（配置里是明文，接口只回空串 + 这份清单） */
  secret_keys?: string[]
  installed: boolean
  enabled: boolean
}

export async function fetchPluginConfig(name: string): Promise<PluginConfig> {
  if (USE_MOCK) {
    return {
      schema: [
        { key: 'at_bot', type: 'bool', default: true, desc: '需要 @ 机器人才响应' },
        { key: 'use_fallback', type: 'bool', default: false, desc: '无匹配时使用兜底回复' },
        {
          key: 'fallback_text',
          type: 'string',
          multiline: true,
          default: '',
          desc: '兜底回复内容',
          when: { key: 'use_fallback', oneOf: [true] },
        },
        {
          key: 'reply_style',
          default: '简洁',
          options: ['简洁', '详细', 'Markdown'],
          desc: '回复风格',
          group: '行为',
        },
        {
          key: 'timeout_seconds',
          type: 'int',
          default: 60,
          min: 5,
          max: 600,
          desc: '请求超时秒数',
          group: '行为',
        },
        {
          key: 'system_prompt',
          type: 'string',
          multiline: true,
          default: '你是答疑助手',
          desc: '系统提示词',
          group: '行为',
        },
        { key: 'admin_users', type: 'list', default: [], desc: '管理员 QQ 号', group: '权限' },
        { key: 'trigger_words', type: 'list', default: [], desc: '触发词', group: '权限' },
        {
          key: 'headers',
          type: 'dict',
          default: {},
          desc: '透传给上游的自定义请求头',
          group: '高级',
        },
        { key: 'api_key', type: 'secret', default: '', desc: '上游服务密钥', group: '高级' },
        { key: 'badge', type: 'asset', default: '', desc: '回复卡片配图', group: '高级' },
      ],
      values: {
        at_bot: true,
        use_fallback: false,
        fallback_text: '',
        reply_style: '简洁',
        timeout_seconds: 60,
        system_prompt: '你是答疑助手',
        admin_users: ['10000'],
        trigger_words: [],
        headers: { 'X-Trace': 'demo' },
        api_key: '',
        badge: '',
      },
      secret_keys: ['api_key'],
      installed: true,
      enabled: true,
    }
  }
  return http(`/api/plugins/${encodeURIComponent(name)}/config`)
}

export async function savePluginConfig(name: string, values: Record<string, unknown>): Promise<void> {
  if (USE_MOCK) {
    await delay(200)
    return
  }
  await http(`/api/plugins/${encodeURIComponent(name)}/config`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(values),
  })
}

// ---- 插件资源（asset 字段的文件面，宿主落 extensions/<名>/assets/）----

// mock 的资源预览：上传后以 objectURL 顶替，仅本次会话可见
const mockAssetUrls = new Map<string, string>()

/** 资源预览地址。<img> 带不了 Authorization 头，token 走查询参数兜底。 */
export function pluginAssetUrl(name: string, path: string): string {
  if (USE_MOCK) return mockAssetUrls.get(path.split('/').pop() ?? '') ?? ''
  const q = new URLSearchParams({ path })
  const token = localStorage.getItem('loquat_token')
  if (token) q.set('token', token)
  return `/api/plugins/${encodeURIComponent(name)}/assets/view?${q}`
}

/** 上传资源，返回宿主落盘的文件名（重名自动追加 -1/-2）。 */
export async function uploadPluginAsset(name: string, file: File): Promise<string> {
  if (USE_MOCK) {
    await delay(300)
    mockAssetUrls.set(file.name, URL.createObjectURL(file))
    return file.name
  }
  const q = new URLSearchParams({ name: file.name })
  const res = await fetch(`/api/plugins/${encodeURIComponent(name)}/assets?${q}`, {
    method: 'POST',
    headers: { Authorization: `Bearer ${localStorage.getItem('loquat_token') ?? ''}` },
    body: file,
  })
  if (!res.ok) throw new Error(`POST 上传: HTTP ${res.status}`)
  const r = (await res.json()) as { name?: string }
  return r.name ?? file.name
}

export async function deletePluginAsset(name: string, path: string): Promise<void> {
  if (USE_MOCK) {
    await delay(150)
    mockAssetUrls.delete(path.split('/').pop() ?? '')
    return
  }
  await http(`/api/plugins/${encodeURIComponent(name)}/assets?path=${encodeURIComponent(path)}`, {
    method: 'DELETE',
  })
}

// ---- 沙箱会话（页面在 extensions/sandbox_ui，这里只留契约类型）----

/** 注入段（protojson Segment 形态，与后端 SendMessage 同构）。 */
export interface SandboxSegment {
  text?: { text: string }
  at?: { target: string }
  reply?: { messageId: string }
  image?: { url?: string; filename?: string }
  face?: { faceId: string }
  [key: string]: unknown
}

/** 一个沙箱会话：chat_id 同时是机器人记忆（data/sessions）的键。 */
export interface SandboxSession {
  chatId: string
  mode: 'private' | 'group'
  name: string
  /** 毫秒时间戳 */
  created: number
  updated: number
  count: number
}

/** 历史里的一条记录；in 带 segments，out 只有 text。 */
export interface SandboxRecord {
  dir: 'in' | 'out'
  id?: string
  segments?: SandboxSegment[]
  text?: string
  /** 毫秒时间戳 */
  ts: number
}

// ---- 插件贡献的导航 tab ----

export interface UiTab {
  /** 宿主侧全局唯一：`<插件名>-<tab id>` */
  id: string
  title: string
  /** 前端路由 `/ext/<插件名>/<tab id>`，由 ExtensionPage 挂载 */
  path: string
  /** ui.js 入口（按 v 缓存指纹加载） */
  entry?: string
  /** element-plus 图标组件名，缺省 Link */
  icon?: string
  /** 侧栏排序权重（小在前）；缺省 100，排到内置页之后 */
  order?: number
  /** ui.js 的 mtime 毫秒：热更后变化，击穿浏览器 module 缓存 */
  v?: number
}

/** 插件贡献的导航 tab：宿主聚合已启用插件的 TABS 声明（内置页在前端 tabs.ts）。 */
export async function fetchUiTabs(): Promise<UiTab[]> {
  if (USE_MOCK) {
    return [
      {
        id: 'demo_ui-demo',
        title: '演示页',
        path: '/ext/demo_ui/demo',
        entry: '/ext/demo_ui/ui.js',
        icon: 'MagicStick',
        v: 1,
      },
    ]
  }
  const r = await http<{ tabs: UiTab[] }>('/api/extensions/ui')
  return r.tabs
}

// ---- 插件页面 ctx（ExtensionPage 挂载自定义元素时注入）----

export interface ExtCtx {
  plugin: string
  /** tab id，含子路径（如 `detail/napcat`）；插件自行分段解析 */
  tab: string
  api: {
    get<T = unknown>(path: string): Promise<T>
    post<T = unknown>(path: string, body?: unknown): Promise<T>
    patch<T = unknown>(path: string, body?: unknown): Promise<T>
    del<T = unknown>(path: string): Promise<T>
  }
  /** Element Plus 消息条的薄封装；type 缺省 info */
  toast(msg: string, type?: 'success' | 'warning' | 'error'): void
  /** 跳转前端路由（其他插件 tab、内置页、自己的子路径都走它） */
  nav(path: string): void
  /** 事件流 WS 地址。WebSocket 带不了 Authorization 头，token 走查询参数。 */
  wsUrl(): string
  /** 未保存守卫：插件页在时路由离开/刷新关页会被拦截确认。
   * 返回注销函数。与页面生命周期同宽，离开页面后自动失效。 */
  guardDirty(isDirty: () => boolean): () => void
}

/** ctx.api 部分：复用前端 http 封装，token 自动携带。其余成员依赖
 * 路由 / ElMessage，由 ExtensionPage.vue 组装完整 ctx。 */
export function extCtxApi(): ExtCtx['api'] {
  return {
    get: <T>(path: string) => http<T>(path),
    post: <T>(path: string, body?: unknown) =>
      http<T>(path, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body ?? {}),
      }),
    patch: <T>(path: string, body?: unknown) =>
      http<T>(path, {
        method: 'PATCH',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body ?? {}),
      }),
    del: <T>(path: string) => http<T>(path, { method: 'DELETE' }),
  }
}
