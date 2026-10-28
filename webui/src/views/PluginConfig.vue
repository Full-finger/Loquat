<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRoute } from 'vue-router'
import { ElMessage } from 'element-plus'
import { fetchPluginConfig, savePluginConfig, type PluginConfig, type SchemaField } from '../api'
import { useDirtyGuard } from '../composables/dirty-leave'

const route = useRoute()
const name = String(route.params.name)

const loading = ref(false)
const saving = ref(false)
const cfg = ref<PluginConfig>({ schema: [], values: {}, installed: false, enabled: true })
const values = ref<Record<string, unknown>>({})

// 加载时的快照：表单与快照不一致即有未保存更改
let snapshot = '{}'

async function refresh() {
  loading.value = true
  try {
    cfg.value = await fetchPluginConfig(name)
    values.value = { ...cfg.value.values }
    snapshot = JSON.stringify(values.value)
  } catch (e) {
    ElMessage.error(String(e))
  } finally {
    loading.value = false
  }
}

const dirty = computed(() => JSON.stringify(values.value) !== snapshot)

useDirtyGuard(() => dirty.value)

/** list 类型用 textarea 存 JSON；其余标量直绑。 */
function listOf(v: unknown): string {
  return JSON.stringify(v ?? [])
}

function setListOf(field: SchemaField, text: string) {
  try {
    values.value[field.key] = JSON.parse(text || '[]')
  } catch {
    // 输入中的非法 JSON 先保留文本，保存时校验兜底
    values.value[field.key] = text
  }
}

async function save() {
  saving.value = true
  try {
    await savePluginConfig(name, values.value)
    snapshot = JSON.stringify(values.value)
    ElMessage.success(`${name} 配置已保存并热重载`)
  } catch (e) {
    ElMessage.error(String(e))
  } finally {
    saving.value = false
  }
}

function reset() {
  values.value = JSON.parse(snapshot)
}

onMounted(refresh)
</script>

<template>
  <div v-loading="loading">
    <div class="page-head">
      <div>
        <h2>
          配置 · {{ name }}
          <el-tag v-if="dirty" size="small" type="warning" effect="plain">未保存</el-tag>
        </h2>
        <div class="desc">按插件 SCHEMA 动态渲染；保存后写盘并热重载</div>
      </div>
      <div v-if="cfg.schema.length">
        <el-button size="small" style="margin-right: 8px" :disabled="!dirty" @click="reset">
          重置
        </el-button>
        <el-button size="small" type="primary" :loading="saving" :disabled="!dirty" @click="save">
          保存（热重载）
        </el-button>
      </div>
    </div>

    <div class="panel">
      <el-empty v-if="!loading && cfg.schema.length === 0" description="该插件未声明 SCHEMA" :image-size="60" />
      <el-form v-else label-position="top" size="small">
        <el-form-item v-for="f in cfg.schema" :key="f.key" :label="f.desc || f.key">
          <el-select v-if="f.options" v-model="values[f.key]" style="width: 100%">
            <el-option v-for="o in f.options" :key="String(o)" :value="o" :label="String(o)" />
          </el-select>
          <el-switch v-else-if="f.type === 'bool'" v-model="values[f.key]" />
          <el-input-number
            v-else-if="f.type === 'int' || f.type === 'float'"
            v-model="values[f.key] as number"
            :precision="f.type === 'int' ? 0 : 2"
            :min="f.min"
            :max="f.max"
            style="width: 100%"
          />
          <el-input
            v-else-if="f.type === 'list'"
            :model-value="listOf(values[f.key])"
            type="textarea"
            :rows="2"
            placeholder='JSON 数组，如 ["a", "b"]'
            @update:model-value="(t: string) => setListOf(f, t)"
          />
          <el-input
            v-else
            v-model="values[f.key] as string"
            :autosize="{ minRows: 1, maxRows: 6 }"
            type="textarea"
          />
        </el-form-item>
      </el-form>
    </div>
  </div>
</template>
