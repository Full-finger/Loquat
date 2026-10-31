<script setup lang="ts">
// 「服务」页（原内置 Providers.vue 迁移为插件）。数据与保存走宿主注入的
// ctx.api；未保存守卫改走 ctx.guardDirty（插件页没有 vue-router）。
import { computed, onMounted, ref } from 'vue'
import { ElMessage } from 'element-plus'
import type { ProviderEntry, ProvidersConfig } from '@webui/api'
import { useResource } from '@webui/composables/use-resource'
import PageHead from '@webui/components/PageHead.vue'
import Panel from '@webui/components/Panel.vue'
import FieldRow from '@webui/components/FieldRow.vue'
import { useCtx } from '@webui/ext/mount'

const ctx = useCtx()

const saving = ref(false)
const cfg = ref<ProvidersConfig>({
  llm: { active: '', providers: {} },
  search: { active: '', providers: {} },
})

// 卡片编辑走本视图的 cfg（深改耦合 useResource 的 data 不划算）；
// 快照整体对比得 dirty（守卫/按钮），逐字段对比得条目高亮
const snap = ref<ProvidersConfig | null>(null)

const { loading, refresh } = useResource(() => ctx.api.get<ProvidersConfig>('/api/providers'), {
  onSuccess: (c) => {
    cfg.value = c
    snap.value = JSON.parse(JSON.stringify(c))
  },
})

const dirty = computed(() => JSON.stringify(cfg.value) !== JSON.stringify(snap.value))

// 插件页没有路由离开事件：守卫注册到宿主挂载器（ExtensionPage），
// 由它拦路由离开与刷新关页
ctx.guardDirty(() => dirty.value)

/** 单条配置是否改过：给 FieldRow 亮未保存警示条。快照里没有的
 * provider（新增未保存）全部字段都算改过。 */
function fieldDirty(section: 'llm' | 'search', name: string, key: keyof ProviderEntry): boolean {
  return (
    JSON.stringify(cfg.value[section].providers[name]?.[key]) !==
    JSON.stringify(snap.value?.[section].providers[name]?.[key])
  )
}

// 新增 provider 的名字输入
const newName = ref('')

function llmNames(): string[] {
  return Object.keys(cfg.value.llm.providers)
}

function searchNames(): string[] {
  return Object.keys(cfg.value.search.providers)
}

function addLlm() {
  const name = newName.value.trim()
  if (!name) return
  if (cfg.value.llm.providers[name]) {
    ElMessage.warning(`${name} 已存在`)
    return
  }
  cfg.value.llm.providers[name] = { api_base: '', api_key: '', model: '' }
  newName.value = ''
}

function removeLlm(name: string) {
  if (Object.keys(cfg.value.llm.providers).length <= 1) {
    ElMessage.warning('至少保留一个 LLM provider')
    return
  }
  delete cfg.value.llm.providers[name]
  if (cfg.value.llm.active === name) {
    cfg.value.llm.active = llmNames()[0] ?? ''
  }
}

// 卡片体不用 v-model：本文件经历过 vue-tsc 3.x 作用域误报的折腾，
// 统一走显式 update 回调，行为等价且类型推断稳定
function setLlm(name: string, key: string, v: unknown) {
  const e = cfg.value.llm.providers[name]
  if (e) (e as Record<string, unknown>)[key] = v
}

function setSearchKey(name: string, v: string) {
  const e = cfg.value.search.providers[name]
  if (e) e.api_key = v
}

function clearKey(section: 'llm' | 'search', name: string) {
  const e = cfg.value[section].providers[name]
  if (e) e.api_key = null as unknown as string
}

async function save() {
  saving.value = true
  try {
    const r = await ctx.api.post<{ msg?: string }>('/api/providers', cfg.value)
    ElMessage.success(r.msg ?? '已保存')
    await refresh()
  } catch (e) {
    ElMessage.error(String(e))
  } finally {
    saving.value = false
  }
}

const SEARCH_HINTS: Record<string, string> = {
  deepseek: '官方 Search/Fetch API，复用 LLM 的 key，国内直连，按成功计费',
  duckduckgo: '免 key（需代理）',
  bocha: '需要自己的 api_key',
}

// 模板属性里放字面 URL（含 //）会踩 vue-tsc 3.3.13 的生成代码作用域 bug
// （// 被当注释截断虚拟 TS 的行，后续 v-for 闭包全丢），URL 一律走 script 常量
const API_BASE_PLACEHOLDER = 'https://api.deepseek.com/v1'

onMounted(refresh)
</script>

<template>
  <div>
    <PageHead>
      <template #actions>
        <el-button size="small" @click="refresh">刷新</el-button>
        <el-button size="small" type="primary" :loading="saving" :disabled="!dirty" @click="save">
          保存
        </el-button>
      </template>
    </PageHead>

    <div v-loading="loading">
      <!-- LLM 后端 -->
      <Panel class="section" title="LLM 后端" desc="OpenAI 兼容 /chat/completions；激活项供所有会话使用">
        <template #actions>
          <div class="new-row">
            <el-input
              v-model="newName"
              size="small"
              placeholder="新 provider 名，如 ollama"
              style="width: 180px"
              @keyup.enter="addLlm"
            />
            <el-button size="small" @click="addLlm">新增</el-button>
          </div>
        </template>

        <div class="cards">
          <div
            v-for="pname in llmNames()"
            :key="pname"
            class="card"
            :class="{ active: cfg.llm.active === pname }"
          >
            <div class="card-head">
              <el-radio v-model="cfg.llm.active" :value="pname">{{ pname }}</el-radio>
              <div class="head-right">
                <el-tag v-if="cfg.llm.providers[pname].has_api_key" size="small" effect="plain">
                  已配置 key
                </el-tag>
                <el-button size="small" text type="danger" @click="removeLlm(pname)">删除</el-button>
              </div>
            </div>

            <FieldRow label="API 地址" :dirty="fieldDirty('llm', pname, 'api_base')">
              <el-input
                :model-value="cfg.llm.providers[pname].api_base"
                :placeholder="API_BASE_PLACEHOLDER"
                @update:model-value="(v: string) => setLlm(pname, 'api_base', v)"
              />
            </FieldRow>
            <FieldRow label="模型" :dirty="fieldDirty('llm', pname, 'model')">
              <el-input
                :model-value="cfg.llm.providers[pname].model"
                placeholder="deepseek-chat"
                @update:model-value="(v: string) => setLlm(pname, 'model', v)"
              />
            </FieldRow>
            <FieldRow label="API 密钥" :dirty="fieldDirty('llm', pname, 'api_key')">
              <div class="key-row">
                <el-input
                  :model-value="cfg.llm.providers[pname].api_key"
                  :placeholder="cfg.llm.providers[pname].has_api_key ? '已配置，留空保持不变' : '替换成你的key'"
                  show-password
                  @update:model-value="(v: string) => setLlm(pname, 'api_key', v)"
                />
                <el-button
                  v-if="cfg.llm.providers[pname].has_api_key"
                  size="small"
                  text
                  @click="clearKey('llm', pname)"
                >
                  清除
                </el-button>
              </div>
            </FieldRow>
            <FieldRow label="温度" :dirty="fieldDirty('llm', pname, 'temperature')">
              <el-input-number
                :model-value="cfg.llm.providers[pname].temperature"
                :min="0"
                :max="2"
                :step="0.1"
                controls-position="right"
                @update:model-value="(v: number | undefined) => setLlm(pname, 'temperature', v)"
              />
            </FieldRow>
            <FieldRow label="max_tokens" :dirty="fieldDirty('llm', pname, 'max_tokens')">
              <el-input-number
                :model-value="cfg.llm.providers[pname].max_tokens"
                :min="1"
                controls-position="right"
                @update:model-value="(v: number | undefined) => setLlm(pname, 'max_tokens', v)"
              />
            </FieldRow>
            <FieldRow label="超时（秒）" :dirty="fieldDirty('llm', pname, 'timeout_seconds')">
              <el-input-number
                :model-value="cfg.llm.providers[pname].timeout_seconds"
                :min="1"
                controls-position="right"
                @update:model-value="(v: number | undefined) => setLlm(pname, 'timeout_seconds', v)"
              />
            </FieldRow>
            <FieldRow
              label="识图（vision）"
              hint="后端支持图片输入时开启"
              :dirty="fieldDirty('llm', pname, 'vision')"
            >
              <el-switch
                :model-value="cfg.llm.providers[pname].vision"
                @update:model-value="(v: boolean) => setLlm(pname, 'vision', v)"
              />
            </FieldRow>
          </div>
        </div>
      </Panel>

      <!-- 联网搜索 -->
      <Panel class="section" title="联网搜索" desc="web_search / read_webpage 的通道；激活项决定走哪家">
        <div class="cards search-cards">
          <div
            v-for="sname in searchNames()"
            :key="sname"
            class="card"
            :class="{ active: cfg.search.active === sname }"
          >
            <div class="card-head">
              <el-radio v-model="cfg.search.active" :value="sname">{{ sname }}</el-radio>
              <el-tag v-if="cfg.search.providers[sname].has_api_key" size="small" effect="plain">
                已配置 key
              </el-tag>
            </div>
            <div class="card-desc">{{ SEARCH_HINTS[sname] ?? '自定义通道' }}</div>
            <FieldRow label="API 密钥" :dirty="fieldDirty('search', sname, 'api_key')">
              <div class="key-row">
                <el-input
                  :model-value="cfg.search.providers[sname].api_key"
                  :placeholder="cfg.search.providers[sname].has_api_key ? '已配置，留空保持不变' : '未配置'"
                  show-password
                  :disabled="sname === 'deepseek' || sname === 'duckduckgo'"
                  @update:model-value="(v: string) => setSearchKey(sname, v)"
                />
                <el-button
                  v-if="cfg.search.providers[sname].has_api_key"
                  size="small"
                  text
                  @click="clearKey('search', sname)"
                >
                  清除
                </el-button>
              </div>
            </FieldRow>
          </div>
        </div>
      </Panel>
    </div>
  </div>
</template>

<style scoped>
.section {
  margin-bottom: 16px;
}

.new-row {
  display: flex;
  gap: 8px;
  align-items: center;
}

.cards {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(340px, 1fr));
  gap: 16px;
}

.card {
  border: 1px solid var(--border-subtle);
  border-radius: 8px;
  padding: 12px;
  background: var(--bg-raised);
}

.card.active {
  border-color: var(--accent);
}

.card-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 10px;
}

.head-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.card-desc {
  font-size: 12px;
  color: var(--text-3);
  margin-bottom: 8px;
}

.key-row {
  display: flex;
  gap: 8px;
  width: 100%;
}
</style>
