<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { ElMessage } from 'element-plus'
import { applyAdapters, fetchAdapters, type AdapterConfig, type AdapterState } from '../api'

const loading = ref(false)
const saving = ref(false)
const config = ref<AdapterConfig>({
  adapters: [],
  napcat: { url: '', token: '', path: '' },
  sandbox: { path: '' },
})
const running = ref<AdapterState[]>([])

const napcatOn = computed(() => config.value.adapters.includes('napcat'))
const sandboxOn = computed(() => config.value.adapters.includes('sandbox'))

function stateOf(platform: string): AdapterState | undefined {
  return running.value.find((r) => r.platform === platform)
}

function toggle(name: 'napcat' | 'sandbox', on: boolean) {
  const set = new Set(config.value.adapters)
  if (on) set.add(name)
  else set.delete(name)
  config.value.adapters = [...set]
}

async function refresh() {
  loading.value = true
  try {
    const r = await fetchAdapters()
    config.value = r.config
    running.value = r.running ?? []
  } catch (e) {
    ElMessage.error(String(e))
  } finally {
    loading.value = false
  }
}

async function save() {
  saving.value = true
  try {
    const msg = await applyAdapters(config.value)
    ElMessage.success(msg)
    await refresh()
  } catch (e) {
    ElMessage.error(String(e))
  } finally {
    saving.value = false
  }
}

onMounted(refresh)
</script>

<template>
  <div>
    <div class="page-head">
      <div>
        <h2>适配器</h2>
        <div class="desc">
          适配器把平台消息翻译成统一格式；改动保存后写盘并热应用，无需重启
        </div>
      </div>
      <div>
        <el-button size="small" style="margin-right: 8px" @click="refresh">刷新</el-button>
        <el-button size="small" type="primary" :loading="saving" @click="save">保存并应用</el-button>
      </div>
    </div>

    <div v-loading="loading" class="cards">
      <!-- napcat -->
      <div class="panel card">
        <div class="card-head">
          <div>
            <div class="card-title">napcat <el-tag size="small" effect="plain">QQ 真机</el-tag></div>
            <div class="card-desc">经 NapCat 连接 QQ，收发真实消息</div>
          </div>
          <div class="head-right">
            <span
              v-if="stateOf('napcat')"
              class="state"
              :class="stateOf('napcat')!.alive ? 'on' : 'off'"
            >
              {{ stateOf('napcat')!.alive ? '运行中' : '已停止' }}
            </span>
            <el-switch :model-value="napcatOn" @change="(v: any) => toggle('napcat', Boolean(v))" />
          </div>
        </div>
        <el-form label-position="top" size="small" :disabled="!napcatOn" class="card-form">
          <el-form-item label="WebSocket 地址">
            <el-input v-model="config.napcat.url" placeholder="ws://127.0.0.1:3001" />
          </el-form-item>
          <el-form-item label="access_token">
            <el-input v-model="config.napcat.token" placeholder="留空表示无鉴权" show-password />
          </el-form-item>
          <el-form-item label="适配器路径">
            <el-input v-model="config.napcat.path" placeholder="bin/adapter-napcat.exe" />
          </el-form-item>
        </el-form>
      </div>

      <!-- sandbox -->
      <div class="panel card">
        <div class="card-head">
          <div>
            <div class="card-title">sandbox <el-tag size="small" effect="plain">开发沙箱</el-tag></div>
            <div class="card-desc">WebUI「沙箱」选项卡直接对话，无需真实平台</div>
          </div>
          <div class="head-right">
            <span
              v-if="stateOf('sandbox')"
              class="state"
              :class="stateOf('sandbox')!.alive ? 'on' : 'off'"
            >
              {{ stateOf('sandbox')!.alive ? '运行中' : '已停止' }}
            </span>
            <el-switch :model-value="sandboxOn" @change="(v: any) => toggle('sandbox', Boolean(v))" />
          </div>
        </div>
        <el-form label-position="top" size="small" :disabled="!sandboxOn" class="card-form">
          <el-form-item label="适配器路径">
            <el-input v-model="config.sandbox.path" placeholder="bin/adapter-sandbox.exe" />
          </el-form-item>
        </el-form>
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
}

.card-head {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 12px;
}

.card-title {
  font-weight: 600;
  display: flex;
  align-items: center;
  gap: 8px;
}

.card-desc {
  font-size: 12px;
  color: var(--text-3);
  margin-top: 4px;
}

.head-right {
  display: flex;
  align-items: center;
  gap: 10px;
}

.state {
  font-size: 12px;
  display: inline-flex;
  align-items: center;
  gap: 5px;
}

.state::before {
  content: '';
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: currentColor;
}

.state.on {
  color: var(--ok);
}

.state.off {
  color: var(--text-3);
}

.card-form {
  margin-top: 4px;
}
</style>
