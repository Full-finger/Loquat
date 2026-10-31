<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import { installPlugin, listPlugins, setPluginEnabled, type PluginInfo } from '../api'
import { refreshExtTabs } from '../composables/ext-tabs'
import { useResource } from '../composables/use-resource'
import StateDot from '../components/StateDot.vue'

const router = useRouter()

// 手动刷新顺带同步侧栏 tab（插件启停状态变了导航也要跟着变）
const { data: plugins, loading, refresh } = useResource(listPlugins, { onSuccess: refreshExtTabs })

// 待安装置底（保留后端的名称序）； fresh 卡片弱化展示，别插在运行中的中间
const sorted = computed(() => {
  const list = [...(plugins.value ?? [])]
  list.sort((a, b) => Number(b.installed) - Number(a.installed))
  return list
})

// 概览行数字
const stats = computed(() => {
  const list = plugins.value ?? []
  return {
    installed: list.filter((p) => p.installed).length,
    enabled: list.filter((p) => p.installed && p.enabled).length,
    fresh: list.filter((p) => !p.installed).length,
  }
})

function stateOf(p: PluginInfo): { text: string; state: 'on' | 'off' | 'warn' } {
  if (!p.installed) return { text: '未安装', state: 'warn' }
  if (!p.enabled) return { text: '已禁用', state: 'off' }
  return { text: '运行中', state: 'on' }
}

// 一句话说明卡片在等什么：没有插件描述元数据，按状态给动作提示
function descOf(p: PluginInfo): string {
  if (!p.installed) return '发现插件目录，安装生成默认配置'
  return p.enabled ? '按优先级顺序参与消息路由' : '已停用，不参与消息路由'
}

// 已安装整卡可点进配置；待安装没有配置可看，点了不动
function openCard(p: PluginInfo) {
  if (p.installed) router.push(`/plugins/${encodeURIComponent(p.name)}/config`)
}

async function onSwitch(p: PluginInfo, enabled: boolean) {
  try {
    await setPluginEnabled(p.name, enabled)
    p.enabled = enabled
    // 插件带页面（TABS/ui.js）时侧栏立即增删对应 tab，无需刷新页面
    await refreshExtTabs()
    ElMessage.success(`${p.name} 已${enabled ? '启用' : '禁用'}`)
  } catch (e) {
    ElMessage.error(String(e))
  }
}

async function onInstall(p: PluginInfo) {
  try {
    const msg = await installPlugin(p.name)
    ElMessage.success(msg)
    await refresh()
  } catch (e) {
    ElMessage.error(String(e))
  }
}

onMounted(refresh)
</script>

<template>
  <div>
    <!-- 概览行：排版对齐仪表盘的 overview -->
    <div v-if="plugins?.length" class="overview">
      <div class="ov-item">
        <div class="ov-label">插件</div>
        <div class="ov-value">{{ stats.installed }}</div>
        <div class="ov-sub">extensions/ 已加载</div>
      </div>
      <div class="ov-item">
        <div class="ov-label">启用</div>
        <div class="ov-value">{{ stats.enabled }}</div>
        <div class="ov-sub">参与消息路由</div>
      </div>
      <div class="ov-item">
        <div class="ov-label">待安装</div>
        <div class="ov-value" :class="{ warn: stats.fresh > 0 }">{{ stats.fresh }}</div>
        <div class="ov-sub" :class="{ warn: stats.fresh > 0 }">
          {{ stats.fresh ? '安装后可启用' : '没有待安装项' }}
        </div>
      </div>
    </div>

    <div v-loading="loading" class="cards">
      <div
        v-for="p in sorted"
        :key="p.name"
        class="panel card"
        :class="{ fresh: !p.installed }"
        @click="openCard(p)"
      >
        <div class="card-head">
          <div class="card-title">
            {{ p.name }}
            <span v-if="p.has_ui" class="ui-pill" title="插件提供 WebUI 扩展页">WebUI</span>
          </div>
          <StateDot :state="stateOf(p).state">{{ stateOf(p).text }}</StateDot>
        </div>
        <div class="card-desc">{{ descOf(p) }}</div>
        <div class="card-foot">
          <span class="mono prio">优先级 {{ p.priority }}</span>
          <!-- 阻止冒泡：开关/按钮不触发整卡跳转 -->
          <div class="foot-actions" @click.stop>
            <el-switch
              v-if="p.installed"
              size="small"
              :model-value="p.enabled"
              @change="(v: any) => onSwitch(p, Boolean(v))"
            />
            <el-button v-if="p.installed" size="small" @click="openCard(p)">配置</el-button>
            <el-button v-else size="small" type="primary" @click="onInstall(p)">安装</el-button>
          </div>
        </div>
      </div>
    </div>

    <el-empty
      v-if="plugins && !plugins.length"
      description="extensions/ 下还没有插件"
      :image-size="60"
    />
  </div>
</template>

<style scoped>
/* ---- 概览行：同仪表盘的字号与分隔语言 ---- */
.overview {
  display: flex;
  align-items: stretch;
  flex-wrap: wrap;
  margin-bottom: 20px;
}

.ov-item {
  flex: 1;
  min-width: 170px;
  padding: 2px 24px;
}

.ov-item:first-child {
  padding-left: 0;
}

.ov-item + .ov-item {
  border-left: 1px solid var(--border-subtle);
}

.ov-label {
  color: var(--text-3);
  font-size: 11px;
  letter-spacing: 0.08em;
  margin-bottom: 6px;
}

.ov-value {
  font-family: var(--mono);
  font-size: 22px;
  font-weight: 600;
  line-height: 1.1;
  color: var(--accent);
}

.ov-value.warn {
  color: var(--warn);
}

.ov-sub {
  color: var(--text-3);
  font-size: 11px;
  margin-top: 6px;
}

.ov-sub.warn {
  color: var(--warn);
}

/* ---- 卡片网格：同适配器页的列宽，窄屏自动单列 ---- */
.cards {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(340px, 1fr));
  gap: 16px;
}

.card {
  cursor: pointer;
}

/* 待安装：还没有配置页，整卡不可点 */
.card.fresh {
  cursor: default;
}

.card-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
}

.card-title {
  font-weight: 600;
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

/* WebUI 扩展页标识：胶囊同活跃会话 tag 的配色，形更收 */
.ui-pill {
  font-size: 11px;
  color: var(--text-2);
  background: var(--bg-raised);
  border: 1px solid var(--border-subtle);
  border-radius: var(--r-pill);
  padding: 1px 8px;
  white-space: nowrap;
}

.card-desc {
  margin-top: 4px;
  font-size: 12px;
  color: var(--text-3);
}

/* 操作行：细线与信息区隔开，左元信息右动作 */
.card-foot {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  margin-top: 12px;
  padding-top: 12px;
  border-top: 1px solid var(--border-subtle);
}

.prio {
  font-size: 12px;
  color: var(--text-3);
}

.foot-actions {
  display: flex;
  align-items: center;
  gap: 12px;
}

.foot-actions .el-button + .el-button {
  margin-left: 0; /* 收回 Element 默认的 12px，统一走 gap */
}
</style>
