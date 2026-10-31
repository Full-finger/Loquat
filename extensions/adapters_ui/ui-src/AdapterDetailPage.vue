<script setup lang="ts">
// 「适配器」详情页（原内置 AdapterConfig.vue 迁移为插件）。
// 适配器名取 tab 子路径第二段（/ext/adapters_ui/detail/<名>）；
// 顶部自带返回按钮（插件页没有顶栏 meta.sub 返回）。
import { computed, onMounted, ref } from 'vue'
import { ElMessage } from 'element-plus'
import type { AdapterConfig, AdapterState } from '@webui/api'
import { useResource } from '@webui/composables/use-resource'
import PageHead from '@webui/components/PageHead.vue'
import Panel from '@webui/components/Panel.vue'
import FieldRow from '@webui/components/FieldRow.vue'
import StateDot from '@webui/components/StateDot.vue'
import { useCtx } from '@webui/ext/mount'

const ctx = useCtx()
// tab 形如 detail/<名>；名字进过 URL（宿主校验字符集），无注入面
const name = ctx.tab.split('/')[1] ?? ''

const saving = ref(false)
const config = ref<AdapterConfig>({ adapters: [] })
const running = ref<AdapterState>()

// 当前适配器的定义：通用结构（name/path/env），无平台特判；
// env 键的含义由适配器自己解释，已知键配中文标签便于填写
const def = computed(() => config.value.adapters.find((d) => d.name === name))

// 启动文件只读展示：显式 path 优先，否则核心按 bin/adapter-<name> 约定解析
//（GET 载荷的 resolved_path）；自定义位置由手改 loquat.yaml 兑现，UI 不再暴露编辑
const resolvedPath = computed(
  () => def.value?.resolved_path ?? def.value?.path ?? `bin/adapter-${name}`,
)

const ENV_LABELS: Record<string, string> = {
  LOQUAT_NAPCAT_URL: 'WebSocket 地址',
  LOQUAT_NAPCAT_TOKEN: 'access_token',
}
const labelOf = (key: string) => ENV_LABELS[key] ?? key
// 值是否遮显：token/secret 类键用密码框
const secretLike = (key: string) => /TOKEN|SECRET|PASSWORD/i.test(key)

// env 行编辑模型：KV 对象 ↔ 行数组，空行保存时丢弃
interface EnvRow {
  key: string
  value: string
}
const envRows = ref<EnvRow[]>([])
function syncRowsToDef() {
  if (!def.value) return
  const env: Record<string, string> = {}
  for (const row of envRows.value) {
    if (row.key.trim()) env[row.key.trim()] = row.value
  }
  def.value.env = Object.keys(env).length ? env : undefined
}
function loadRowsFromDef() {
  envRows.value = Object.entries(def.value?.env ?? {}).map(([key, value]) => ({
    key,
    value,
  }))
}

const snapshot = ref('')

const { loading, refresh } = useResource(
  () => ctx.api.get<{ config: AdapterConfig; running?: AdapterState[] }>('/api/adapters'),
  {
  onSuccess: (r: { config: AdapterConfig; running?: AdapterState[] }) => {
    config.value = r.config
    running.value = r.running?.find((x) => x.platform === name)
    loadRowsFromDef()
    snapshot.value = JSON.stringify(def.value ?? null)
  },
})

const dirty = computed(() => {
  if (!def.value) return false
  syncRowsToDef()
  return JSON.stringify(def.value) !== snapshot.value
})

// 未保存守卫：注册到宿主挂载器，拦路由离开（含侧栏切页）与刷新关页
ctx.guardDirty(() => dirty.value)

function addRow() {
  envRows.value.push({ key: '', value: '' })
}
function removeRow(i: number) {
  envRows.value.splice(i, 1)
}

async function save() {
  saving.value = true
  try {
    syncRowsToDef()
    const r = await ctx.api.post<{ msg?: string }>('/api/adapters/apply', config.value)
    snapshot.value = JSON.stringify(def.value ?? null)
    ElMessage.success(r.msg ?? '已应用')
  } catch (e) {
    ElMessage.error(String(e))
  } finally {
    saving.value = false
  }
}

function reset() {
  const snap = JSON.parse(snapshot.value || 'null') as AdapterConfig['adapters'][number] | null
  if (!def.value || !snap) return
  Object.assign(def.value, snap)
  loadRowsFromDef()
}

function back() {
  ctx.nav('/ext/adapters_ui/main')
}

onMounted(refresh)
</script>

<template>
  <div v-loading="loading">
    <PageHead>
      <template #actions>
        <div class="head-right">
          <el-button size="small" @click="back">
            <el-icon style="margin-right: 4px"><Back /></el-icon>返回列表
          </el-button>
          <StateDot v-if="running" :state="running.alive ? 'on' : 'off'">
            {{ running.alive ? '运行中' : '已停止' }}
          </StateDot>
          <el-button size="small" :disabled="!dirty" @click="reset">
            重置
          </el-button>
          <el-button size="small" type="primary" :loading="saving" :disabled="!dirty" @click="save">
            保存并应用
          </el-button>
        </div>
      </template>
    </PageHead>

    <Panel>
      <el-empty v-if="!def" description="适配器未定义，请先在列表页添加" :image-size="60" />
      <div v-else style="max-width: 560px">
        <FieldRow
          label="启动文件"
          hint="由 bin/ 目录自动发现；自定义位置请改 config/loquat.yaml 的 path"
        >
          <span class="mono path-value">{{ resolvedPath }}</span>
        </FieldRow>
        <FieldRow
          v-for="(row, i) in envRows"
          :key="i"
          :label="row.key.trim() ? labelOf(row.key) : '参数'"
          :hint="labelOf(row.key) === row.key ? '' : row.key"
        >
          <div class="env-row">
            <el-input v-model="row.key" placeholder="键（如 LOQUAT_X_URL）" class="env-key" />
            <el-input
              v-model="row.value"
              :placeholder="secretLike(row.key) ? '值（遮显）' : '值'"
              :show-password="secretLike(row.key)"
            />
            <el-button text type="danger" @click="removeRow(i)">删除</el-button>
          </div>
        </FieldRow>
        <div class="env-add">
          <el-button size="small" @click="addRow">添加参数</el-button>
          <span class="env-hint">env 键名由适配器自己约定，核心不解释内容</span>
        </div>
      </div>
    </Panel>
  </div>
</template>

<style scoped>
.head-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.env-row {
  display: flex;
  gap: 8px;
}

.env-key {
  flex: 0 0 45%;
}

.env-add {
  padding: 12px 15px;
  display: flex;
  align-items: center;
  gap: 12px;
}

.env-hint {
  font-size: 12px;
  color: var(--text-3);
}

.path-value {
  font-size: 13px;
}
</style>
