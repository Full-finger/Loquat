// 侧栏对齐测量：量汉堡按钮与菜单图标在展开/折叠两态下的中心 x 与关键 computed style。
// 用法：先起宿主（7900），再 node scripts/measure-sidebar.mjs
import { chromium } from 'playwright-core'

const browser = await chromium.launch({ channel: 'msedge', headless: true })
const page = await browser.newPage({ viewport: { width: 1200, height: 800 } })
await page.goto('http://127.0.0.1:7900/')
await page.waitForTimeout(1200)

async function measure(label) {
  const data = await page.evaluate(() => {
    const cx = (el) => {
      if (!el) return null
      const r = el.getBoundingClientRect()
      return Math.round((r.left + r.width / 2) * 10) / 10
    }
    const rect = (el) => {
      if (!el) return null
      const r = el.getBoundingClientRect()
      return { left: Math.round(r.left * 10) / 10, width: Math.round(r.width * 10) / 10 }
    }
    const cs = (el, props) => {
      if (!el) return null
      const s = getComputedStyle(el)
      return Object.fromEntries(props.map((p) => [p, s[p]]))
    }
    const btn = document.querySelector('.fold-btn')
    const menu = document.querySelector('.menu')
    const item = document.querySelector('.menu .el-menu-item')
    const icon = document.querySelector('.menu .el-menu-item .el-icon')
    const span = document.querySelector('.menu .el-menu-item span')
    return {
      汉堡按钮: { 中心: cx(btn), ...rect(btn), style: cs(btn, ['width', 'height']) },
      顶栏: cs(document.querySelector('.topbar'), ['paddingLeft']),
      菜单: cs(menu, ['paddingLeft', 'paddingRight']),
      首项: {
        中心图标: cx(icon),
        ...rect(icon),
        style: cs(item, ['paddingLeft', 'display', 'justifyContent']),
      },
      图标槽: cs(icon, ['width', 'height', 'display', 'marginRight']),
      文字: cs(span, ['display', 'width']),
    }
  })
  console.log(`${label}:`)
  console.log(JSON.stringify(data, null, 2))
}

await measure('展开态')
await page.click('.fold-btn')
await page.waitForTimeout(500)
await measure('折叠态')

await browser.close()
