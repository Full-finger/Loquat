<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { ElMessage } from 'element-plus'
import {
  fetchPluginConfig,
  installPlugin,
  listPlugins,
  reloadNapcat,
  savePluginConfig,
  setPluginEnabled,
  type PluginConfig,
  type PluginInfo,
  type SchemaField,
} from '../api'

const plugins = ref<PluginInfo[]>([])
const loading = ref(false)

// ── 插件配置编辑（SCHEMA 动态表单）──
const cfgVisible = ref(false)
const cfgSaving = ref(false)
const cfgName = ref('')
const cfg = ref<PluginConfig>({ schema: [], values: {}, installed: false, enabled: true })
const cfgValues = ref<Record<string, unknown>>({})

async function openConfig(row: PluginInfo) {
  cfgName.value = row.name
  try {
    cfg.value = await fetchPluginConfig(row.name)
    cfgValues.value = { ...cfg.value.values }
    cfgVisible.value = true
  } catch (e) {
    ElMessage.error(String(e))
  }
}

/** list 类型用 textarea 存 JSON；其余标量直绑。 */
function listOf(v: unknown): string {
  return JSON.stringify(v ?? [])
}

function setListOf(field: SchemaField, text: string) {
  try {
    cfgValues.value[field.key] = JSON.parse(text || '[]')
  } catch {
    // 输入中的非法 JSON 先保留文本，保存时 cfglib 校验兜底
    cfgValues.value[field.key] = text
  }
}

async function saveConfig() {
  cfgSaving.value = true
  try {
    await savePluginConfig(cfgName.value, cfgValues.value)
    ElMessage.success(`${cfgName.value} 配置已保存并热重载`)
    cfgVisible.value = false
  } catch (e) {
    ElMessage.error(String(e))
  } finally {
    cfgSaving.value = false
  }
}

async function refresh() {
  loading.value = true
  try {
    plugins.value = await listPlugins()
  } catch (e) {
    ElMessage.error(String(e))
  } finally {
    loading.value = false
  }
}

function stateOf(p: PluginInfo): { text: string; cls: string } {
  if (!p.installed) return { text: '未安装', cls: 'warn' }
  if (!p.enabled) return { text: '已禁用', cls: 'off' }
  return { text: '运行中', cls: 'on' }
}

async function onSwitch(row: PluginInfo, enabled: boolean) {
  try {
    await setPluginEnabled(row.name, enabled)
    row.enabled = enabled
    ElMessage.success(`${row.name} 已${enabled ? '启用' : '禁用'}`)
  } catch (e) {
    ElMessage.error(String(e))
  }
}

async function onInstall(row: PluginInfo) {
  try {
    const msg = await installPlugin(row.name)
    ElMessage.success(msg)
    await refresh()
  } catch (e) {
    ElMessage.error(String(e))
  }
}

async function onReloadNapcat() {
  try {
    ElMessage.info(await reloadNapcat())
  } catch (e) {
    ElMessage.error(String(e))
  }
}

onMounted(refresh)
</script>

<template>
  <div>
    <div class="page-head">
      <div>
        <h2>插件</h2>
        <div class="desc">责任链顺序执行；启停写入运行时标记，重启后回到文件内声明</div>
      </div>
      <div>
        <el-button size="small" style="margin-right: 8px" @click="onReloadNapcat">
          重载 napcat
        </el-button>
        <el-button size="small" @click="refresh">刷新</el-button>
      </div>
    </div>

    <div class="panel" style="padding: 0">
      <el-table :data="plugins" v-loading="loading" style="width: 100%">
        <el-table-column label="名称" min-width="160">
          <template #default="{ row }">
            <el-icon style="vertical-align: -2px; margin-right: 6px; color: var(--text-3)">
              <Files />
            </el-icon>
            <span style="font-weight: 500">{{ row.name }}</span>
          </template>
        </el-table-column>
        <el-table-column label="优先级" width="100" align="center">
          <template #default="{ row }">
            <span class="mono" style="color: var(--text-2)">{{ row.priority }}</span>
          </template>
        </el-table-column>
        <el-table-column label="状态" width="110">
          <template #default="{ row }">
            <span class="state">
              <span class="state-dot" :class="stateOf(row).cls" />
              {{ stateOf(row).text }}
            </span>
          </template>
        </el-table-column>
        <el-table-column label="启用" width="70" align="center">
          <template #default="{ row }">
            <el-switch
              size="small"
              :model-value="row.enabled"
              :disabled="!row.installed"
              @change="(v: any) => onSwitch(row, Boolean(v))"
            />
          </template>
        </el-table-column>
        <el-table-column label="" width="160" align="right">
          <template #default="{ row }">
            <el-button v-if="row.installed" size="small" @click="openConfig(row)">配置</el-button>
            <el-button v-if="!row.installed" size="small" type="primary" @click="onInstall(row)">
              安装
            </el-button>
          </template>
        </el-table-column>
      </el-table>
    </div>

    <!-- 配置编辑：按插件 SCHEMA 动态渲染 -->
    <el-dialog v-model="cfgVisible" :title="`配置 · ${cfgName}`" width="560px">
      <el-empty v-if="cfg.schema.length === 0" description="该插件未声明 SCHEMA" :image-size="60" />
      <el-form v-else label-position="top" size="small">
        <el-form-item v-for="f in cfg.schema" :key="f.key" :label="f.desc || f.key">
          <el-select
            v-if="f.options"
            v-model="cfgValues[f.key]"
            style="width: 100%"
          >
            <el-option v-for="o in f.options" :key="String(o)" :value="o" :label="String(o)" />
          </el-select>
          <el-switch
            v-else-if="f.type === 'bool'"
            v-model="cfgValues[f.key]"
          />
          <el-input-number
            v-else-if="f.type === 'int' || f.type === 'float'"
            v-model="cfgValues[f.key] as number"
            :precision="f.type === 'int' ? 0 : 2"
            :min="f.min"
            :max="f.max"
            style="width: 100%"
          />
          <el-input
            v-else-if="f.type === 'list'"
            :model-value="listOf(cfgValues[f.key])"
            type="textarea"
            :rows="2"
            placeholder='JSON 数组，如 ["a", "b"]'
            @update:model-value="(t: string) => setListOf(f, t)"
          />
          <el-input
            v-else
            v-model="cfgValues[f.key] as string"
            :autosize="{ minRows: 1, maxRows: 6 }"
            type="textarea"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button size="small" @click="cfgVisible = false">取消</el-button>
        <el-button
          v-if="cfg.schema.length"
          size="small"
          type="primary"
          :loading="cfgSaving"
          @click="saveConfig"
        >
          保存（热重载）
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped>
.state {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: var(--text-2);
}

.state-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
}

.state-dot.on {
  background: var(--ok);
}

.state-dot.off {
  background: var(--text-3);
}

.state-dot.warn {
  background: var(--warn);
}
</style>
