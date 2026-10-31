<script setup lang="ts">
// SCHEMA → 表单的集中渲染器：插件配置页与将来的声明式插件页共用这一份。
// 字段外壳统一走 FieldRow（标签/说明/未保存高亮），这里只按 type 分发控件：
// string（multiline 变多行）/ secret（打码密钥，留空=保持）/ asset（资源
// 上传与预览）/ int·float / bool / 枚举（options ≤4 平铺单选块，多了下拉）/
// list（chips 增删）/ dict（键值行）；when 控制条件显隐（隐藏字段的值
// 原样保存）。values 由视图持有并传入，这里原地改写；dirty 回调逐字段
// 索要高亮状态。
import { computed, reactive, ref } from 'vue'
import { ElMessage } from 'element-plus'
import { pluginAssetUrl, uploadPluginAsset, type SchemaField } from '../api'
import FieldRow from './FieldRow.vue'

const props = defineProps<{
  schema: SchemaField[]
  values: Record<string, unknown>
  dirty?: (key: string) => boolean
  /** 已配置的 secret 字段清单（接口层打码的回执） */
  secretKeys?: string[]
  /** asset 上传需要插件名定位资源目录；缺省隐藏上传入口 */
  plugin?: string
}>()

// 分组：未声明 group 的排最前（无标题、不可折叠），其余按首次出现顺序
// 归组；when 不满足的字段不渲染（值仍在 values 里原样保存）；折叠
// 状态记组名，默认全开。
const closed = reactive(new Set<string>())

function toggleGroup(label: string) {
  if (closed.has(label)) closed.delete(label)
  else closed.add(label)
}

function visible(f: SchemaField): boolean {
  if (!f.when) return true
  return f.when.oneOf.some((v) => JSON.stringify(v) === JSON.stringify(props.values[f.when!.key]))
}

const groups = computed<{ label: string; fields: SchemaField[] }[]>(() => {
  const out: { label: string; fields: SchemaField[] }[] = [{ label: '', fields: [] }]
  for (const f of props.schema) {
    if (!visible(f)) continue
    const label = f.group ?? ''
    if (!label) {
      out[0].fields.push(f)
      continue
    }
    let g = out.find((x) => x.label === label)
    if (!g) out.push((g = { label, fields: [] }))
    g.fields.push(f)
  }
  return out.filter((g) => g.label || g.fields.length)
})

// ── list 条目（chips 增删）──

function itemsOf(f: SchemaField): unknown[] {
  return Array.isArray(props.values[f.key]) ? (props.values[f.key] as unknown[]) : []
}

const draft = reactive<Record<string, string>>({})

function addItem(f: SchemaField) {
  const text = (draft[f.key] ?? '').trim()
  if (!text) return
  props.values[f.key] = [...itemsOf(f), text]
  draft[f.key] = ''
}

function removeItem(f: SchemaField, i: number) {
  props.values[f.key] = itemsOf(f).filter((_, idx) => idx !== i)
}

// ── dict 键值行 ──
// 每次变更都以新对象整体替换（键增删/重排让 Vue 的键复用失灵，原地改
// 不触发依赖收集），快照对比按 JSON 文本进行，顺序即所见顺序。

function entriesOf(f: SchemaField): Record<string, unknown> {
  const v = props.values[f.key]
  return v && typeof v === 'object' && !Array.isArray(v) ? (v as Record<string, unknown>) : {}
}

function addKey(f: SchemaField) {
  const cur = entriesOf(f)
  let n = 1
  while (`key${n}` in cur) n += 1
  props.values[f.key] = { ...cur, [`key${n}`]: '' }
}

function removeKey(f: SchemaField, key: string) {
  const cur = entriesOf(f)
  delete cur[key]
  props.values[f.key] = { ...cur }
}

function setValue(f: SchemaField, key: string, v: string) {
  props.values[f.key] = { ...entriesOf(f), [key]: v }
}

/** 键重命名在 @change（失焦/回车）时提交——逐键即时重建会抖动光标。 */
function renameKey(f: SchemaField, oldKey: string, next: string) {
  const nk = next.trim()
  const cur = entriesOf(f)
  if (nk === oldKey) return
  if (!nk || nk in cur) {
    ElMessage.warning(!nk ? '键名不能为空' : `键 ${nk} 已存在`)
    props.values[f.key] = { ...cur } // 换新对象逼行重渲染，输入框回到现值
    return
  }
  const out: Record<string, unknown> = {}
  for (const [k, v] of Object.entries(cur)) out[k === oldKey ? nk : k] = v
  props.values[f.key] = out
}

// ── secret（打码密钥）──

function hasSecret(f: SchemaField): boolean {
  return props.secretKeys?.includes(f.key) ?? false
}

// ── asset（资源上传与预览）──

const ASSET_MAX_BYTES = 10 * 1024 * 1024

function assetPath(f: SchemaField): string {
  return String(props.values[f.key] ?? '')
}

function isImage(path: string): boolean {
  return /\.(png|jpe?g|gif|webp|bmp|svg|ico)$/i.test(path)
}

// 隐藏 file input 全字段共用：pickingKey 记当前在给哪个字段选文件
const fileEl = ref<HTMLInputElement>()
let pickingKey = ''

function pickAsset(f: SchemaField) {
  if (!props.plugin) return
  pickingKey = f.key
  fileEl.value?.click()
}

async function onFilePicked(e: Event) {
  const input = e.target as HTMLInputElement
  const file = input.files?.[0]
  input.value = '' // 清掉让同一文件可重复选
  if (!file || !props.plugin || !pickingKey) return
  if (file.size > ASSET_MAX_BYTES) {
    ElMessage.warning('文件超过 10MB 上限')
    return
  }
  try {
    const stored = await uploadPluginAsset(props.plugin, file)
    props.values[pickingKey] = `assets/${stored}`
  } catch (err) {
    ElMessage.error(String(err))
  }
}
</script>

<template>
  <template v-for="g in groups" :key="g.label || '_'">
    <button v-if="g.label" type="button" class="group-head" @click="toggleGroup(g.label)">
      <el-icon :size="12" class="caret" :class="{ closed: closed.has(g.label) }">
        <ArrowDown />
      </el-icon>
      {{ g.label }}
      <span class="group-count">{{ g.fields.length }} 项</span>
    </button>
    <div v-show="!g.label || !closed.has(g.label)">
      <FieldRow
        v-for="f in g.fields"
        :key="f.key"
        :label="f.desc || f.key"
        :dirty="dirty?.(f.key)"
      >
        <!-- 枚举：少选项平铺单选块，多选项下拉 -->
        <el-radio-group v-if="f.options && f.options.length <= 4" v-model="values[f.key]">
          <el-radio-button v-for="o in f.options" :key="String(o)" :value="o">
            {{ o }}
          </el-radio-button>
        </el-radio-group>
        <el-select v-else-if="f.options" v-model="values[f.key]" style="width: 100%">
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
        <div v-else-if="f.type === 'list'" class="chips">
          <el-tag
            v-for="(it, i) in itemsOf(f)"
            :key="i"
            closable
            size="small"
            @close="removeItem(f, i)"
          >
            {{ String(it) }}
          </el-tag>
          <el-input
            v-model="draft[f.key]"
            size="small"
            style="width: 160px"
            placeholder="输入后回车添加"
            @keyup.enter="addItem(f)"
          />
        </div>
        <div v-else-if="f.type === 'dict'" class="dict">
          <div v-for="k in Object.keys(entriesOf(f))" :key="k" class="dict-row">
            <el-input
              :model-value="k"
              size="small"
              style="width: 38%"
              placeholder="键"
              @change="(v: string) => renameKey(f, k, v)"
            />
            <el-input
              :model-value="String(entriesOf(f)[k] ?? '')"
              size="small"
              placeholder="值"
              @update:model-value="(v: string) => setValue(f, k, v)"
            />
            <el-button size="small" text type="danger" @click="removeKey(f, k)">删除</el-button>
          </div>
          <el-button size="small" @click="addKey(f)">添加一项</el-button>
        </div>
        <!-- secret：接口层打码（只回空串 + secret_keys），留空=保持、清除置 null -->
        <div v-else-if="f.type === 'secret'" class="key-row">
          <el-input
            :model-value="String(values[f.key] ?? '')"
            show-password
            :placeholder="hasSecret(f) ? '已配置，留空保存即保持原值' : '输入密钥'"
            @update:model-value="(v: string) => (values[f.key] = v)"
          />
          <el-button v-if="hasSecret(f)" size="small" text @click="values[f.key] = null">
            清除
          </el-button>
        </div>
        <!-- asset：配置值是 assets/<文件名> 相对路径，预览走宿主 view 端点 -->
        <div v-else-if="f.type === 'asset'" class="asset">
          <template v-if="assetPath(f)">
            <img
              v-if="isImage(assetPath(f))"
              :src="pluginAssetUrl(plugin ?? '', assetPath(f))"
              class="asset-thumb"
              alt=""
            />
            <span v-else class="mono asset-name">{{ assetPath(f) }}</span>
            <el-button size="small" text type="danger" @click="values[f.key] = ''">移除</el-button>
          </template>
          <el-button v-if="plugin" size="small" @click="pickAsset(f)">上传…</el-button>
        </div>
        <el-input
          v-else-if="f.multiline"
          v-model="values[f.key] as string"
          type="textarea"
          :autosize="{ minRows: 2, maxRows: 8 }"
        />
        <el-input v-else v-model="values[f.key] as string" />
      </FieldRow>
    </div>
  </template>
  <!-- 全字段共用的文件选择器；pickingKey 记当前目标字段 -->
  <input ref="fileEl" type="file" hidden @change="onFilePicked" />
</template>

<style scoped>
.group-head {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 12px 12px 8px;
  border: none;
  background: none;
  cursor: pointer;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-2);
}

.caret {
  color: var(--text-3);
  transition: transform 0.15s ease;
}

.caret.closed {
  transform: rotate(-90deg);
}

.group-count {
  font-size: 11px;
  font-weight: 400;
  color: var(--text-3);
}

.chips {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
}

.dict {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

/* flex column 会把按钮拉成通栏，收回内容宽度 */
.dict > .el-button {
  align-self: flex-start;
}

.dict-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.key-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.asset {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 10px;
}

.asset-thumb {
  width: 64px;
  height: 64px;
  object-fit: cover;
  border-radius: 6px;
  border: 1px solid var(--border-subtle);
}

.asset-name {
  font-size: 12px;
  color: var(--text-2);
}
</style>
