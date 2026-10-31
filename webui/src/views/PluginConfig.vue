<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRoute } from 'vue-router'
import { ElMessage } from 'element-plus'
import { fetchPluginConfig, savePluginConfig, type PluginConfig } from '../api'
import { useDirtyGuard } from '../composables/dirty-leave'
import { useResource } from '../composables/use-resource'
import PageHead from '../components/PageHead.vue'
import Panel from '../components/Panel.vue'
import SchemaForm from '../components/SchemaForm.vue'

const route = useRoute()
const name = String(route.params.name)

const saving = ref(false)
const cfg = ref<PluginConfig>({ schema: [], values: {}, installed: false, enabled: true })
const values = ref<Record<string, unknown>>({})

// 加载时的快照：整体对比得 dirty（守卫/按钮），逐字段对比得条目高亮。
// 必须深拷贝——dict/list 编辑是原地改写嵌套对象，浅拷贝会共享引用，
// 改动就比对不出来了。ref 保证保存后赋值能触发 dirty 重算。
const clone = (v: Record<string, unknown>) => JSON.parse(JSON.stringify(v)) as Record<string, unknown>
const snapshot = ref<Record<string, unknown>>({})

const { loading, refresh } = useResource(() => fetchPluginConfig(name), {
  onSuccess: (c) => {
    cfg.value = c
    values.value = clone(c.values)
    snapshot.value = clone(c.values)
  },
})

const dirty = computed(() => JSON.stringify(values.value) !== JSON.stringify(snapshot.value))

useDirtyGuard(() => dirty.value)

/** 单条配置是否改过：给 FieldRow 亮未保存警示条。 */
function fieldDirty(key: string): boolean {
  return JSON.stringify(values.value[key]) !== JSON.stringify(snapshot.value[key])
}

async function save() {
  saving.value = true
  try {
    await savePluginConfig(name, values.value)
    // 回读同步 secret 回执（secret_keys 随清除/新配置变化），顺带重置快照
    await refresh()
    ElMessage.success(`${name} 配置已保存并热重载`)
  } catch (e) {
    ElMessage.error(String(e))
  } finally {
    saving.value = false
  }
}

function reset() {
  values.value = clone(snapshot.value)
}

onMounted(refresh)
</script>

<template>
  <div v-loading="loading">
    <PageHead>
      <template v-if="cfg.schema.length" #actions>
        <el-button size="small" :disabled="!dirty" @click="reset">
          重置
        </el-button>
        <el-button size="small" type="primary" :loading="saving" :disabled="!dirty" @click="save">
          保存（热重载）
        </el-button>
      </template>
    </PageHead>

    <Panel>
      <el-empty
        v-if="!loading && cfg.schema.length === 0"
        description="该插件未声明 SCHEMA"
        :image-size="60"
      />
      <SchemaForm
        v-else
        :schema="cfg.schema"
        :values="values"
        :dirty="fieldDirty"
        :secret-keys="cfg.secret_keys ?? []"
        :plugin="name"
      />
    </Panel>
  </div>
</template>
