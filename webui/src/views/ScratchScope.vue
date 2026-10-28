<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { ElMessage } from 'element-plus'
import { fetchProviders, saveProviders, type ProvidersConfig } from '../api'

const loading = ref(false)
const saving = ref(false)
const cfg = ref<ProvidersConfig>({
  llm: { active: '', providers: {} },
  search: { active: '', providers: {} },
})

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

async function refresh() {
  loading.value = true
  try {
    cfg.value = await fetchProviders()
  } catch (e) {
    ElMessage.error(String(e))
  } finally {
    loading.value = false
  }
}

async function save() {
  saving.value = true
  try {
    const msg = await saveProviders(cfg.value)
    ElMessage.success(msg)
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

onMounted(refresh)
</script>

<template>
  <div>
    <div class="page-head">
      <div>
        <h2>服务</h2>
        <div class="desc">
          LLM 与联网搜索的接入点、凭据，宿主级统一管理；插件零改动，
          切换激活项即换后端。保存即热生效
        </div>
      </div>
      <div>
        <el-button size="small" style="margin-right: 8px" @click="refresh">刷新</el-button>
        <el-button size="small" type="primary" :loading="saving" @click="save">保存</el-button>
      </div>
    </div>

    <div v-loading="loading">
      <!-- LLM 后端 -->
      <div class="panel section">
        <div class="section-head">
          <div>
            <div class="section-title">LLM 后端</div>
            <div class="section-desc">OpenAI 兼容 /chat/completions；激活项供所有会话使用</div>
          </div>
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
        </div>

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

<div class="fitem">
              <span class="flabel">API 地址</span>
              <el-input
                :model-value="cfg.llm.providers[pname].api_base"
                placeholder="x"
                @update:model-value="(v: string) => setLlm(pname, 'api_base', v)"
              />
            </div>
            <div class="fitem">
              <span class="flabel">模型</span>
              <el-input
                :model-value="cfg.llm.providers[pname].model"
                placeholder="deepseek-chat"
                @update:model-value="(v: string) => setLlm(pname, 'model', v)"
              />
            </div>
            <div class="fitem">
              <span class="flabel">API 密钥</span>
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
            </div>
            <div class="field-row">
              <div class="fitem">
                <span class="flabel">温度</span>
                <el-input-number
                  :model-value="cfg.llm.providers[pname].temperature"
                  :min="0"
                  :max="2"
                  :step="0.1"
                  controls-position="right"
                  @update:model-value="(v: number | undefined) => setLlm(pname, 'temperature', v)"
                />
              </div>
              <div class="fitem">
                <span class="flabel">max_tokens</span>
                <el-input-number
                  :model-value="cfg.llm.providers[pname].max_tokens"
                  :min="1"
                  controls-position="right"
                  @update:model-value="(v: number | undefined) => setLlm(pname, 'max_tokens', v)"
                />
              </div>
              <div class="fitem">
                <span class="flabel">超时（秒）</span>
                <el-input-number
                  :model-value="cfg.llm.providers[pname].timeout_seconds"
                  :min="1"
                  controls-position="right"
                  @update:model-value="(v: number | undefined) => setLlm(pname, 'timeout_seconds', v)"
                />
              </div>
            </div>
            <div class="fitem">
              <span class="flabel">识图（vision）</span>
              <div class="switch-row">
                <el-switch
                  :model-value="cfg.llm.providers[pname].vision"
                  @update:model-value="(v: boolean) => setLlm(pname, 'vision', v)"
                />
                <span class="inline-hint">后端支持图片输入时开启</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- 联网搜索 -->
      <div class="panel section">
        <div class="section-head">
          <div>
            <div class="section-title">联网搜索</div>
            <div class="section-desc">web_search / read_webpage 的通道；激活项决定走哪家</div>
          </div>
        </div>

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
            <div class="fitem">
              <span class="flabel">API 密钥</span>
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
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.section {
  padding: 16px;
  margin-bottom: 16px;
}

.section-head {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 12px;
}

.section-title {
  font-size: 14px;
  font-weight: 600;
}

.section-desc {
  font-size: 12px;
  color: var(--text-3);
  margin-top: 4px;
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

.fitem {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-bottom: 10px;
}

.flabel {
  font-size: 12px;
  color: var(--text-3);
}

.key-row {
  display: flex;
  gap: 8px;
  width: 100%;
}

.field-row {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
}

.switch-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.inline-hint {
  font-size: 12px;
  color: var(--text-3);
}
</style>
