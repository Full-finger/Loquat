<script setup lang="ts">
// 「适配器」列表页（原内置 Adapters.vue 迁移为插件）。
// 详情进 tab 子路径 /ext/adapters_ui/detail/<名>（ctx.nav），可深链刷新。
import { computed, onMounted, ref } from 'vue'
import { ElMessage } from 'element-plus'
import type { AdapterConfig, AdapterDef, AdapterState, DiscoveredAdapter } from '@webui/api'
import { useResource } from '@webui/composables/use-resource'
import StateDot from '@webui/components/StateDot.vue'
import { useCtx } from '@webui/ext/mount'

const ctx = useCtx()

const config = ref<AdapterConfig>({ adapters: [] })
const running = ref<AdapterState[]>([])
const discovered = ref<DiscoveredAdapter[]>([])

const META: Record<string, { tag: string; desc: string }> = {
  napcat: { tag: 'QQ 真机', desc: '经 NapCat 连接 QQ，收发真实消息' },
  sandbox: { tag: '开发沙箱', desc: 'WebUI「沙箱」选项卡直接对话，无需真实平台' },
}

const { loading, refresh } = useResource(
  () =>
    ctx.api.get<{
      config: AdapterConfig
      running?: AdapterState[]
      discovered?: DiscoveredAdapter[]
    }>('/api/adapters'),
  {
  onSuccess: (r: { config: AdapterConfig; running?: AdapterState[]; discovered?: DiscoveredAdapter[] }) => {
    config.value = r.config
    running.value = r.running ?? []
    discovered.value = r.discovered ?? []
  },
})

// 发现但未配置的适配器：bin/ 里有二进制，YAML 还没声明
const extra = computed(() =>
  discovered.value.filter((d) => !config.value.adapters.some((a) => a.name === d.name)),
)

/** 启用发现项：写入最小定义（名字即可，path 走 bin/ 约定）并热应用 */
async function enableDiscovered(d: DiscoveredAdapter) {
  config.value.adapters.push({ name: d.name })
  try {
    const r = await ctx.api.post<{ msg?: string }>('/api/adapters/apply', config.value)
    ElMessage.success(r.msg ?? '已应用')
  } catch (e) {
    ElMessage.error(String(e))
  } finally {
    await refresh() // 失败时回到实际状态
  }
}

function stateOf(platform: string): AdapterState | undefined {
  return running.value.find((r) => r.platform === platform)
}

/** 列表页开关即改即用（写盘 + 热应用）。停用保留定义与参数，不删条目。 */
async function toggle(def: AdapterDef, on: boolean) {
  def.enabled = on ? undefined : false
  try {
    const r = await ctx.api.post<{ msg?: string }>('/api/adapters/apply', config.value)
    ElMessage.success(r.msg ?? '已应用')
  } catch (e) {
    ElMessage.error(String(e))
  } finally {
    await refresh() // 失败时回到实际状态
  }
}

onMounted(refresh)
</script>

<template>
  <div>
    <div v-loading="loading" class="cards">
      <div
        v-for="def in config.adapters"
        :key="def.name"
        class="panel card"
        @click="ctx.nav(`/ext/adapters_ui/detail/${def.name}`)"
      >
        <div class="card-head">
          <div>
            <div class="card-title">
              {{ def.name }}
              <el-tag size="small" effect="plain">{{ META[def.name]?.tag ?? '自定义' }}</el-tag>
            </div>
            <div class="card-desc">
              {{ META[def.name]?.desc ?? '通用适配器：按 path 拉起，参数经 env 传入' }}
            </div>
          </div>
          <div class="head-right">
            <StateDot
              v-if="stateOf(def.name)"
              :state="stateOf(def.name)!.alive ? 'on' : 'off'"
            >
              {{ stateOf(def.name)!.alive ? '运行中' : '已停止' }}
            </StateDot>
            <el-switch
              :model-value="def.enabled !== false"
              @click.stop
              @change="(v: any) => toggle(def, Boolean(v))"
            />
          </div>
        </div>
      </div>

      <!-- 发现流：bin/ 里有二进制但尚未配置的适配器，一键启用 -->
      <div v-for="d in extra" :key="d.name" class="panel card fresh">
        <div class="card-head">
          <div>
            <div class="card-title">
              {{ d.name }}
              <el-tag size="small" effect="plain" type="info">未配置</el-tag>
            </div>
            <div class="card-desc">发现 bin/ 下的适配器，启用后写入配置并拉起</div>
          </div>
          <div class="head-right">
            <el-button size="small" type="primary" @click="enableDiscovered(d)">启用</el-button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.cards {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(340px, 1fr));
  gap: 16px;
}

.card {
  padding: 16px;
  cursor: pointer;
}

/* 未配置卡片不可进详情（还没有定义），不可点 */
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
}

.card-desc {
  margin-top: 4px;
  font-size: 12px;
  color: var(--text-3);
}

.head-right {
  display: flex;
  align-items: center;
  gap: 10px;
}
</style>
