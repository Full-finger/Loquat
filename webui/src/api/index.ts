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
export const WS_URL = `ws://${location.host}/api/ws`
