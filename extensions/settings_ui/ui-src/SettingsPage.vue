<script setup lang="ts">
// 「设置」页：UI 偏好的编辑界面。偏好本体与生效逻辑在壳的
// @webui/ext/prefs.ts（单一实现点），这里只收集输入——
// 改动即时保存即时生效，无需「保存」按钮；token 除外（显式保存）。
import { computed, onMounted, ref, watch } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import type { UiTab } from '@webui/api'
import { mergeTabs } from '@webui/tabs'
import { clearPrefs, loadPrefs, savePrefs } from '@webui/ext/prefs'
import PageHead from '@webui/components/PageHead.vue'
import Panel from '@webui/components/Panel.vue'
import FieldRow from '@webui/components/FieldRow.vue'
import { useCtx } from '@webui/ext/mount'

const ctx = useCtx()

const prefs = ref(loadPrefs())
watch(prefs, (p) => savePrefs({ ...p }), { deep: true })

// 配色方案（Windows Terminal 式）：深浅 × 预设色组合成命名方案，
// 左侧色卡预览该方案的派生色（面板底/正文/图表次级/主色），点击整套应用。
// 各模式的令牌基线与 theme.css 同源，预览色卡按同一公式派生。
const MODES: { label: string; value: '' | 'light' }[] = [
  { label: '深色', value: '' },
  { label: '浅色', value: 'light' },
]

const MODE_TOKENS: Record<'' | 'light', { accent: string; panel: string; text: string }> = {
  '': { accent: '#539bf5', panel: '#2d333b', text: '#cdd9e5' },
  light: { accent: '#0969da', panel: '#ffffff', text: '#1f2328' },
}

// 预设色板：Primer 系，外加枇杷橙（项目名的颜色）。首档「默认」用
// 空串表示（清掉覆盖，回到 theme.css 的 token——两模式色值不同）
const PRESETS: { label: string; value: string }[] = [
  { label: '默认蓝', value: '' },
  { label: '枇杷橙', value: '#e09140' },
  { label: '青', value: '#56d4dd' },
  { label: '紫', value: '#986ee2' },
  { label: '绿', value: '#57ab5a' },
]

interface Scheme {
  key: string
  mode: '' | 'light'
  /** 写进 prefs.accent 的值（「默认蓝」为空串 = 跟随主题默认） */
  accentPref: string
  label: string
  /** 色卡预览用：面板底 / 正文 / 图表次级 / 主色（实际生效值） */
  panel: string
  text: string
  secondary: string
  accent: string
}

const SCHEMES: Scheme[] = MODES.flatMap((m) =>
  PRESETS.map((p) => {
    const t = MODE_TOKENS[m.value]
    const accent = p.value === '' ? t.accent : p.value
    return {
      key: `${m.value}|${p.value}`,
      mode: m.value,
      accentPref: p.value,
      label: `${m.label} · ${p.label}`,
      panel: t.panel,
      text: t.text,
      secondary: `color-mix(in srgb, ${accent} 35%, ${t.panel})`,
      accent,
    }
  }),
)

// 当前命中方案：深浅与主色都吻合才亮；自定义取色后全部熄灭
const activeKey = computed(
  () => SCHEMES.find((s) => s.mode === prefs.value.theme && s.accentPref === prefs.value.accent)?.key ?? '',
)

function applyScheme(s: Scheme) {
  prefs.value.theme = s.mode
  prefs.value.accent = s.accentPref
}

// 自定义取色：el-color-picker 空值是 null，映射回偏好的空串语义
function onCustomPick(v: string | null) {
  prefs.value.accent = v ?? ''
}

// 落地页选项 = 侧栏同款清单（内置 + 插件聚合），值存路径
const tabOptions = ref<{ title: string; path: string }[]>([])

async function refreshTabs() {
  try {
    const r = await ctx.api.get<{ tabs: UiTab[] }>('/api/extensions/ui')
    tabOptions.value = mergeTabs(r.tabs).map((t) => ({ title: t.title, path: t.path }))
  } catch (e) {
    ElMessage.error(`页面清单获取失败：${e}`)
  }
}

// token 单独显式保存：它是敏感项且被 http/ws 直接读，不该跟着
// 偏好 watch 隐式落盘
const token = ref(localStorage.getItem('loquat_token') ?? '')

function saveToken() {
  if (token.value) localStorage.setItem('loquat_token', token.value)
  else localStorage.removeItem('loquat_token')
  ElMessage.success('已保存，新请求与新连接生效')
}

async function resetAll() {
  try {
    await ElMessageBox.confirm('全部偏好回到默认值，不影响宿主配置与插件数据。', '恢复默认', {
      type: 'warning',
      confirmButtonText: '恢复',
      cancelButtonText: '取消',
    })
  } catch {
    return // 取消
  }
  clearPrefs()
  prefs.value = loadPrefs()
  token.value = ''
  localStorage.removeItem('loquat_token')
  ElMessage.success('已恢复默认')
}

onMounted(refreshTabs)
</script>

<template>
  <div class="settings">
    <PageHead>
      <template #actions>
        <el-button size="small" @click="resetAll">恢复默认</el-button>
      </template>
    </PageHead>

    <Panel class="section" title="外观" desc="改动即时生效并记住，只作用于当前浏览器">
      <FieldRow label="配色方案" hint="色卡预览：面板底 · 正文 · 图表次级 · 主色">
        <div class="scheme-grid">
          <button
            v-for="s in SCHEMES"
            :key="s.key"
            class="scheme"
            :class="{ active: activeKey === s.key }"
            @click="applyScheme(s)"
          >
            <span class="scheme-swatches">
              <i :style="{ background: s.panel }" />
              <i :style="{ background: s.text }" />
              <i :style="{ background: s.secondary }" />
              <i :style="{ background: s.accent }" />
            </span>
            <span class="scheme-name">{{ s.label }}</span>
          </button>
        </div>
      </FieldRow>
      <FieldRow label="自定义主题色" hint="全部颜色令牌跟随主色派生；自定义后上方方案取消高亮">
        <div class="swatch-row">
          <el-color-picker
            class="custom-pick"
            :model-value="prefs.accent || null"
            @update:model-value="onCustomPick"
          />
        </div>
      </FieldRow>
    </Panel>

    <Panel class="section" title="启动" desc="打开 WebUI 时落在一个页面">
      <FieldRow label="默认落地页" hint="缺省跟随侧栏排序（第一个）">
        <el-select v-model="prefs.home" style="width: 260px" placeholder="跟随排序">
          <el-option label="跟随排序（第一个）" value="" />
          <el-option
            v-for="t in tabOptions"
            :key="t.path"
            :label="t.title"
            :value="t.path"
          />
        </el-select>
      </FieldRow>
    </Panel>

    <Panel class="section" title="访问" desc="宿主在 config/webui.yaml 配置了 token 时，用它通过鉴权">
      <FieldRow label="访问 token" hint="只存本浏览器；改动对页面内新请求立即生效">
        <div class="key-row">
          <el-input
            v-model="token"
            placeholder="留空 = 无鉴权"
            show-password
            @keyup.enter="saveToken"
          />
          <el-button size="small" @click="saveToken">保存</el-button>
        </div>
      </FieldRow>
    </Panel>
  </div>
</template>

<style scoped>
.settings {
  max-width: 720px;
  /* 与仪表盘 .dash 同款：宽屏下内容居中，不贴侧栏 */
  margin: 0 auto;
}

.section {
  margin-bottom: 16px;
}

.swatch-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.scheme-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px;
}

.scheme {
  display: flex;
  align-items: center;
  gap: 10px;
  border: 1px solid var(--border-subtle);
  background: var(--bg-raised);
  border-radius: var(--r-sm);
  padding: 8px 12px;
  cursor: pointer;
  text-align: left;
  transition: border-color 0.12s ease, background 0.12s ease;
}

.scheme:hover {
  border-color: var(--border);
}

.scheme.active {
  border-color: var(--accent);
  background: color-mix(in srgb, var(--accent) 10%, var(--bg-raised));
}

/* 色卡 2×2：面板底 / 正文 / 图表次级 / 主色，同 WT 配色方案的预览思路 */
.scheme-swatches {
  display: grid;
  grid-template-columns: repeat(2, 12px);
  grid-auto-rows: 12px;
  gap: 2px;
  flex-shrink: 0;
}

.scheme-swatches i {
  border-radius: 3px;
}

.scheme-name {
  font-size: 12px;
  color: var(--text-2);
  white-space: nowrap;
}

.scheme.active .scheme-name {
  color: var(--text-1);
}

.swatch-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.key-row {
  display: flex;
  gap: 8px;
  width: 100%;
  max-width: 420px;
}
</style>
