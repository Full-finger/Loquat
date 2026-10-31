// 插件页冒烟：起宿主（7900）后跑。
// 验证：导航全部由插件注册、默认首页落在仪表盘、六个插件页都能挂载、
// 适配器详情子路径导航与返回、设置页主题色即时生效与落地页偏好回退、
// 侧栏高亮跟随。
// 用法：node scripts/smoke-ext-tabs.mjs
import { chromium } from 'playwright-core'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const base = process.env.SHOT_BASE ?? 'http://127.0.0.1:7900'
const browser = await chromium.launch({ channel: 'msedge', headless: true })
const page = await browser.newPage({ viewport: { width: 1440, height: 900 } })
const fails = []

function check(name, cond) {
  if (cond) console.log(`  ok  ${name}`)
  else {
    fails.push(name)
    console.log(`FAIL  ${name}`)
  }
}

// 1. 首页重定向到仪表盘（order 最小的插件 tab）
await page.goto(base, { waitUntil: 'networkidle' })
await page.waitForTimeout(800)
check('首页落地 /ext/dashboard_ui/main', page.url().includes('/ext/dashboard_ui/main'))

// 2. 侧栏顺序：仪表盘/插件/消息流/服务/适配器/沙箱
const titles = await page.locator('.menu .el-menu-item span').allTextContents()
check(
  `侧栏顺序 [${titles.join('、')}]`,
  JSON.stringify(titles) ===
    JSON.stringify(['仪表盘', '插件', '消息流', '服务', '适配器', '沙箱', '设置']),
)

// 3. 逐页挂载（无错误卡片 + 关键元素在）
const checks = [
  ['/ext/dashboard_ui/main', '.overview'],
  ['/ext/messages_ui/main', '.msg-list'],
  ['/ext/providers_ui/main', '.cards .card'],
  ['/ext/adapters_ui/main', '.cards .card'],
  ['/ext/sandbox_ui/main', '.new-btn'],
  ['/ext/settings_ui/main', '.swatch-row'],
  ['/plugins', '.el-table, .cards, .plugin-list'],
]
for (const [path, sel] of checks) {
  await page.goto(`${base}${path}`, { waitUntil: 'networkidle' })
  await page.waitForTimeout(900)
  const err = await page.locator('.el-result__title').count()
  check(`${path} 挂载${err ? '（出现错误卡片）' : ''}`, err === 0 && (await page.locator(sel).count()) > 0)
}

// 4. 适配器详情：点卡片 → 子路径 URL、面包屑、返回
await page.goto(`${base}/ext/adapters_ui/main`, { waitUntil: 'networkidle' })
await page.waitForTimeout(600)
await page.locator('.cards .card').first().click()
await page.waitForTimeout(900)
check('详情子路径 URL', /\/ext\/adapters_ui\/detail\/.+/.test(page.url()))
check('面包屑显示适配器', (await page.locator('.crumbs').textContent()).includes('适配器'))
check('侧栏高亮仍在适配器', await page
  .locator('.menu .el-menu-item.is-active')
  .filter({ hasText: '适配器' })
  .count())
await page.locator('button', { hasText: '返回列表' }).click()
await page.waitForTimeout(700)
check('返回列表', page.url().endsWith('/ext/adapters_ui/main'))

// 5. 设置页：切主题色即时生效（html 内联变量覆盖 token）
await page.goto(`${base}/ext/settings_ui/main`, { waitUntil: 'networkidle' })
await page.waitForTimeout(600)
await page.locator('.swatch[title="枇杷橙"]').click()
await page.waitForTimeout(200)
const accent = await page.evaluate(() =>
  document.documentElement.style.getPropertyValue('--accent').trim(),
)
check(`主题色即时生效（--accent=${accent}）`, accent === '#e09140')

// 6. 落地页偏好：指向的插件 tab 不存在时回退第一个（Home.vue 逻辑）
await page.evaluate(() =>
  localStorage.setItem('loquat_prefs', JSON.stringify({ accent: '', home: '/ext/nope/main' })),
)
await page.goto(`${base}/`, { waitUntil: 'networkidle' })
await page.waitForTimeout(800)
check(
  '落地页偏好失效时回退第一个 tab',
  page.url().includes('/ext/dashboard_ui/main'),
)
await page.evaluate(() => localStorage.removeItem('loquat_prefs'))

// 7. 亮色主题：偏好生效为 html 无 dark class，EP 与自身 token 都换肤
await page.goto(`${base}/ext/settings_ui/main`, { waitUntil: 'networkidle' })
await page.waitForTimeout(600)
await page.locator('.theme-btn', { hasText: '浅色' }).click()
await page.waitForTimeout(80)
const theming = await page.evaluate(() =>
  document.documentElement.classList.contains('theming'),
)
const isDark = await page.evaluate(() =>
  document.documentElement.classList.contains('dark'),
)
check('切换时挂上过渡（.theming）', theming)
check('亮色主题生效（无 dark class）', !isDark)
await page.waitForTimeout(600)
check(
  '过渡窗口结束自动摘除',
  !(await page.evaluate(() => document.documentElement.classList.contains('theming'))),
)
const bodyBg = await page.evaluate(() =>
  getComputedStyle(document.body).backgroundColor,
)
check(`亮色页面底色（${bodyBg}）`, bodyBg === 'rgb(244, 246, 248)')
await page.evaluate(() => localStorage.removeItem('loquat_prefs'))

// 截图存档（仓库内 .workbuddy，gitignore 范围外，仅本机留档）
const out =
  process.env.SHOT_OUT ??
  path.join(path.dirname(fileURLToPath(import.meta.url)), '../../.workbuddy/shots/ext-tabs')
await page.goto(`${base}/ext/dashboard_ui/main`, { waitUntil: 'networkidle' })
await page.waitForTimeout(900)
await page.screenshot({ path: `${out}/dashboard.png`, fullPage: true })

await browser.close()
if (fails.length) {
  console.error(`\n${fails.length} 项失败：${fails.join('；')}`)
  process.exit(1)
}
console.log('\n全部通过')
