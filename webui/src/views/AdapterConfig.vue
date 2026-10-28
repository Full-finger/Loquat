<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRoute } from 'vue-router'
import { ElMessage } from 'element-plus'
import { applyAdapters, fetchAdapters, type AdapterConfig, type AdapterState } from '../api'
import { useDirtyGuard } from '../composables/dirty-leave'

const route = useRoute()
const name = String(route.params.name)

const META: Record<string, { tag: string; desc: string }> = {
  napcat: { tag: 'QQ 真机', desc: '经 NapCat 连接 QQ，收发真实消息' },
  sandbox: { tag: '开发沙箱', desc: 'WebUI「沙箱」选项卡直接对话，无需真实平台' },
}
const meta = computed(() => META[name])

const loading = ref(false)
const saving = ref(false)
const config = ref<AdapterConfig>({
  adapters: [],
  napcat: { url: '', token: '', path: '' },
  sandbox: { path: '' },
})
const running = ref<AdapterState>()

// 当前适配器的配置子对象与快照
function part(): { url?: string; token?: string; path: string } {
  return name === 'napcat' ? config.value.napcat : config.value.sandbox
}
let snapshot = ''

async function refresh() {
  loading.value = true
  try {
    const r = await fetchAdapters()
    config.value = r.config
    running.value = r.running?.find((x) => x.platform === name)
    snapshot = JSON.stringify(part())
  } catch (e) {
    ElMessage.error(String(e))
  } finally {
    loading.value = false
  }
}

const dirty = computed(() => JSON.stringify(part()) !== snapshot)

useDirtyGuard(() => dirty.value)

async function save() {
  saving.value = true
  try {
    const msg = await applyAdapters(config.value)
    snapshot = JSON.stringify(part())
    ElMessage.success(msg)
  } catch (e) {
    ElMessage.error(String(e))
  } finally {
    saving.value = false
  }
}

function reset() {
  if (name === 'napcat') config.value.napcat = JSON.parse(snapshot)
  else config.value.sandbox = JSON.parse(snapshot)
}

onMounted(refresh)
</script>

<template>
  <div v-loading="loading">
    <div class="page-head">
      <div>
        <h2>
          {{ name }}
          <el-tag v-if="meta" size="small" effect="plain">{{ meta.tag }}</el-tag>
          <el-tag v-if="dirty" size="small" type="warning" effect="plain">未保存</el-tag>
        </h2>
        <div class="desc">{{ meta?.desc ?? '未知适配器' }}</div>
      </div>
      <div class="head-right">
        <span v-if="running" class="state" :class="running.alive ? 'on' : 'off'">
          {{ running.alive ? '运行中' : '已停止' }}
        </span>
        <el-button size="small" style="margin-left: 8px" :disabled="!dirty" @click="reset">
          重置
        </el-button>
        <el-button size="small" type="primary" :loading="saving" :disabled="!dirty" @click="save">
          保存并应用
        </el-button>
      </div>
    </div>

    <div class="panel">
      <el-empty v-if="!meta" description="未知适配器" :image-size="60" />
      <el-form v-else label-position="top" size="small" style="max-width: 560px">
        <template v-if="name === 'napcat'">
          <el-form-item label="WebSocket 地址">
            <el-input v-model="config.napcat.url" placeholder="ws://127.0.0.1:3001" />
          </el-form-item>
          <el-form-item label="access_token">
            <el-input v-model="config.napcat.token" placeholder="留空表示无鉴权" show-password />
          </el-form-item>
        </template>
        <el-form-item label="适配器路径">
          <el-input v-model="part().path" placeholder="bin/adapter-sandbox.exe" />
        </el-form-item>
      </el-form>
    </div>
  </div>
</template>

<style scoped>
.head-right {
  display: flex;
  align-items: center;
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
</style>
