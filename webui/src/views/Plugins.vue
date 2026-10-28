<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import { installPlugin, listPlugins, reloadNapcat, setPluginEnabled, type PluginInfo } from '../api'
import { refreshExtTabs } from '../composables/ext-tabs'

const router = useRouter()
const plugins = ref<PluginInfo[]>([])
const loading = ref(false)

// ── 插件配置编辑（SCHEMA 动态表单）──
function openConfig(row: PluginInfo) {
  router.push(`/plugins/${encodeURIComponent(row.name)}/config`)
}


async function refresh() {
  loading.value = true
  try {
    plugins.value = await listPlugins()
    // 手动刷新顺带同步侧栏 tab（插件启停状态变了导航也要跟着变）
    await refreshExtTabs()
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
    // 插件带页面（TABS/ui.js）时侧栏立即增删对应 tab，无需刷新页面
    await refreshExtTabs()
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
