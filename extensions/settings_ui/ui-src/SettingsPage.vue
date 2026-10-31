<script setup lang="ts">
// 「设置」页：UI 偏好的编辑界面。偏好本体与生效逻辑在壳的
// @webui/ext/prefs.ts（单一实现点），这里只收集输入——
// 改动即时保存即时生效，无需「保存」按钮；token 除外（显式保存）。
import { onMounted, ref, watch } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import type { UiTab } from '@webui/api'
import { mergeTabs } from '@webui/tabs'
import { clearPrefs, DEFAULT_ACCENT, loadPrefs, savePrefs } from '@webui/ext/prefs'
import PageHead from '@webui/components/PageHead.vue'
import Panel from '@webui/components/Panel.vue'
import FieldRow from '@webui/components/FieldRow.vue'
import { useCtx } from '@webui/ext/mount'

const ctx = useCtx()

const prefs = ref(loadPrefs())
watch(prefs, (p) => savePrefs({ ...p }), { deep: true })

// 主题：'' = 暗色（默认），'light' = 亮色；应用为 html.dark class
const THEMES: { label: string; value: '' | 'light' }[] = [
  { label: '深色', value: '' },
  { label: '浅色', value: 'light' },
]

// 预设色板：Primer 系，外加枇杷橙（项目名的颜色）。首档「默认」用
// 空串表示（清掉覆盖，回到 theme.css 的 token）
const PRESETS: { label: string; value: string }[] = [
  { label: '默认蓝', value: '' },
  { label: '枇杷橙', value: '#e09140' },
  { label: '青', value: '#56d4dd' },
  { label: '紫', value: '#986ee2' },
  { label: '绿', value: '#57ab5a' },
]

function swatchColor(value: string): string {
  return value || DEFAULT_ACCENT
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
      <FieldRow label="主题">
        <div class="swatch-row">
          <button
            v-for="t in THEMES"
            :key="t.label"
            class="theme-btn"
            :class="{ active: prefs.theme === t.value }"
            @click="prefs.theme = t.value"
          >
            {{ t.label }}
          </button>
        </div>
      </FieldRow>
      <FieldRow label="主题色" hint="全部颜色令牌跟随主色派生">
        <div class="swatch-row">
          <button
            v-for="p in PRESETS"
            :key="p.label"
            class="swatch"
            :class="{ active: prefs.accent === p.value }"
            :style="{ background: swatchColor(p.value) }"
            :title="p.label"
            @click="prefs.accent = p.value"
          />
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
}

.section {
  margin-bottom: 16px;
}

.swatch-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.theme-btn {
  border: 1px solid var(--border-subtle);
  background: var(--bg-raised);
  color: var(--text-2);
  border-radius: 6px;
  padding: 4px 14px;
  font-size: 12px;
  cursor: pointer;
  transition: color 0.12s ease, border-color 0.12s ease;
}

.theme-btn:hover {
  color: var(--text-1);
}

.theme-btn.active {
  border-color: var(--accent);
  color: var(--accent);
}

.swatch {
  width: 26px;
  height: 26px;
  border-radius: 50%;
  border: 2px solid transparent;
  cursor: pointer;
  padding: 0;
  outline-offset: 2px;
}

.swatch.active {
  border-color: var(--text-1);
  box-shadow: 0 0 0 2px var(--bg-page);
}

.custom-pick {
  margin-left: 4px;
  vertical-align: middle;
}

.key-row {
  display: flex;
  gap: 8px;
  width: 100%;
  max-width: 420px;
}
</style>
