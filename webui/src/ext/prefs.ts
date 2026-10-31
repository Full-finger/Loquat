// UI 偏好契约：settings_ui 插件写，壳读（main.ts 启动生效、Home.vue
// 决定落地页）。存 localStorage 单键 JSON——UI 偏好是每浏览器一份，
// 不进宿主配置。应用方式：把 token 写到 <html> 内联样式，覆盖
// theme.css 的同名变量；--el-color-primary 等映射自动跟随。
export interface Prefs {
  /** 'light' = 亮色；'' / 'dark' = 暗色（默认）。应用为 html.dark class */
  theme: '' | 'light'
  /** 主题色（任意 CSS 颜色值）；'' = theme.css 当前主题的默认 */
  accent: string
  /** 落地页路径（如 /ext/dashboard_ui/main）；'' = 排序后的第一个 tab */
  home: string
}

const KEY = 'loquat_prefs'

/** 与 theme.css --accent 同源；预设里的「默认」档用空串表示，取色器回显用它 */
export const DEFAULT_ACCENT = '#539bf5'

export function loadPrefs(): Prefs {
  try {
    return { theme: '', accent: '', home: '', ...JSON.parse(localStorage.getItem(KEY) ?? '{}') }
  } catch {
    return { theme: '', accent: '', home: '' } // 损坏就当默认
  }
}

/** 持久化并立即生效，带一次颜色过渡动画（设置页改动即调）。 */
export function savePrefs(p: Prefs): void {
  localStorage.setItem(KEY, JSON.stringify(p))
  applyPrefs(p, true)
}

/** 清空偏好并回到默认（带过渡动画，与设置页的交互一致）。 */
export function clearPrefs(): void {
  localStorage.removeItem(KEY)
  applyPrefs(loadPrefs(), true)
}

// 切换瞬间的过渡：给 html 挂 .theming 让颜色属性做单次插值，动画结束
// 即摘除——平时不挂全局 transition，hover 等交互动画不受影响。
// 渐变（background-image）不参与插值，会直接跳变，可接受。
let themingTimer: number | undefined

function beginTheming(): void {
  const root = document.documentElement
  root.classList.add('theming')
  void root.offsetWidth // 强制一次样式计算，确保过渡从旧值出发
  clearTimeout(themingTimer)
  themingTimer = window.setTimeout(() => root.classList.remove('theming'), 400)
}

/** 壳启动与偏好变更共用的生效点。animate = 切换时是否带颜色过渡
 *  （首帧应用传 false，那时还没画过任何东西，无需动画）。 */
export function applyPrefs(p: Prefs, animate = false): void {
  if (animate) beginTheming()
  const root = document.documentElement
  root.classList.toggle('dark', p.theme !== 'light')
  const style = root.style
  if (p.accent) style.setProperty('--accent', p.accent)
  else style.removeProperty('--accent')
}
