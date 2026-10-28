// 后端 API 封装。
// mock 只在 dev 模式（npm run dev）且未设 VITE_USE_MOCK=false 时生效——
// 后端没起也能开发页面；生产构建（npm run build）永远走真实后端。
// 接口契约对齐 src/console.rs 的控制台命令。

export interface Status {
  core: string
  /** dll.status() 的原始 JSON */
  core_detail?: unknown
  events: number
  replies: number
  /** 各 worker 积压（对齐 stats.rs 的 WORKER_COUNT / WORKER_QUEUE_CAP） */
  workers?: number[]
  business: string
}

export interface PluginInfo {
  name: string
  installed: boolean
  enabled: boolean
  priority: number
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

let mockStatus: Status = { core: '运行中', events: 42, replies: 38, business: '3 个插件' }

const mockPlugins: PluginInfo[] = [
  { name: 'qa', installed: true, enabled: true, priority: 100 },
  { name: 'chat', installed: true, enabled: true, priority: 100 },
  { name: 'status', installed: true, enabled: false, priority: 50 },
  { name: 'weather', installed: false, enabled: false, priority: 100 },
]

// ---- 对外接口 ----

export async function getStatus(): Promise<Status> {
  if (USE_MOCK) {
    await delay(150)
    mockStatus.events += Math.floor(Math.random() * 3)
    mockStatus.replies += Math.floor(Math.random() * 2)
    return { ...mockStatus }
  }
  return http('/api/status')
}

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

export async function reloadNapcat(): Promise<string> {
  if (USE_MOCK) {
    await delay(300)
    return '已触发，核心将退避重启适配器'
  }
  return postMsg('/api/core/reload-napcat')
}

// 消息流 WS 地址；mock 模式下 Messages 页自己造数据，不建连。
// 浏览器 WebSocket API 不能带 Authorization 头，配置了 token 时走
// ?token= 查询参数兜底（宿主 api_ws 同步支持；token 会进访问日志）。
export function wsUrl(): string {
  const proto = location.protocol === 'https:' ? 'wss' : 'ws'
  const token = localStorage.getItem('loquat_token')
  const q = token ? `?token=${encodeURIComponent(token)}` : ''
  return `${proto}://${location.host}/api/ws${q}`
}

// ---- 适配器管理 ----

export interface AdapterConfig {
  adapters: string[]
  napcat: { url: string; token: string; path: string }
  sandbox: { path: string }
}

export interface AdapterState {
  platform: string
  alive: boolean
  pid: number
}

export async function fetchAdapters(): Promise<{ config: AdapterConfig; running: AdapterState[] }> {
  if (USE_MOCK) {
    return {
      config: {
        adapters: ['napcat', 'sandbox'],
        napcat: { url: 'ws://127.0.0.1:3001', token: '', path: 'bin/adapter-napcat.exe' },
        sandbox: { path: 'bin/adapter-sandbox.exe' },
      },
      running: [
        { platform: 'napcat', alive: true, pid: 1234 },
        { platform: 'sandbox', alive: true, pid: 5678 },
      ],
    }
  }
  return http('/api/adapters')
}

/** 应用适配器配置（写盘 + 热应用），返回结果说明。 */
export async function applyAdapters(config: AdapterConfig): Promise<string> {
  if (USE_MOCK) {
    await delay(300)
    return '已应用'
  }
  const r = await http<{ msg?: string }>('/api/adapters/apply', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(config),
  })
  return r.msg ?? '已应用'
}

// ---- 插件配置编辑（SCHEMA 表单）----

export interface SchemaField {
  key: string
  type?: string
  default?: unknown
  desc?: string
  options?: unknown[]
  min?: number
  max?: number
}

export interface PluginConfig {
  schema: SchemaField[]
  values: Record<string, unknown>
  installed: boolean
  enabled: boolean
}

export async function fetchPluginConfig(name: string): Promise<PluginConfig> {
  if (USE_MOCK) {
    return {
      schema: [{ key: 'timeout_seconds', type: 'int', default: 60, desc: '请求超时秒数' }],
      values: { timeout_seconds: 60 },
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

// ---- 服务配置（provider 层）----

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

export async function fetchProviders(): Promise<ProvidersConfig> {
  if (USE_MOCK) {
    return {
      llm: {
        active: 'deepseek',
        providers: {
          deepseek: {
            api_base: 'https://api.deepseek.com/v1',
            api_key: '',
            has_api_key: true,
            model: 'deepseek-chat',
            temperature: 0.8,
            max_tokens: 1024,
            timeout_seconds: 60,
            vision: false,
          },
        },
      },
      search: {
        active: 'deepseek',
        providers: {
          deepseek: { api_key: '' },
          duckduckgo: { api_key: '' },
          bocha: { api_key: '', has_api_key: false },
        },
      },
    }
  }
  return http('/api/providers')
}

/** 保存服务配置：api_key 留空 = 保持已有；置 null = 清除。保存即热生效。 */
export async function saveProviders(cfg: ProvidersConfig): Promise<string> {
  if (USE_MOCK) {
    await delay(200)
    return '已保存，服务配置热生效'
  }
  const r = await http<{ msg?: string }>('/api/providers', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(cfg),
  })
  return r.msg ?? '已保存'
}

// ---- WebUI 扩展机制（插件页面，契约见 docs/ext-ui.md）----

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
  tab: string
  api: {
    get<T = unknown>(path: string): Promise<T>
    post<T = unknown>(path: string, body?: unknown): Promise<T>
  }
  /** Element Plus 消息条的薄封装；type 缺省 info */
  toast(msg: string, type?: 'success' | 'warning' | 'error'): void
}

/** ctx.api 部分：复用前端 http 封装，token 自动携带。toast 依赖
 * ElMessage，由 ExtensionPage.vue 组装完整 ctx。 */
export function extCtxApi(): ExtCtx['api'] {
  return {
    get: <T>(path: string) => http<T>(path),
    post: <T>(path: string, body?: unknown) =>
      http<T>(path, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body ?? {}),
      }),
  }
}

/** 沙箱选项卡的输入通路：注入一条消息进沙箱事件链路。 */
export async function sandboxSend(text: string, mode: string, atBot: boolean): Promise<void> {
  if (USE_MOCK) {
    await delay(200)
    return
  }
  await http('/api/sandbox/send', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ text, mode, atBot }),
  })
}
