<script setup lang="ts">
// 配置条目行：标签 + 说明 + 控件，行间细分隔线。dirty 由视图按自己的
// 加载快照逐字段算好传入，亮起左侧警示条——未保存的改动一眼可见，
// 保存/重置后快照归位，高亮自然消退。
defineProps<{ label: string; hint?: string; dirty?: boolean }>()
</script>

<template>
  <div class="field-row" :class="{ dirty }">
    <div class="row-head">
      <span class="row-label">{{ label }}</span>
      <span v-if="hint" class="row-hint">{{ hint }}</span>
    </div>
    <div class="row-control">
      <slot />
    </div>
  </div>
</template>

<style scoped>
.field-row {
  position: relative; /* 警示条 ::before 的定位锚点 */
  padding: 10px 12px 12px 15px;
}

.field-row + .field-row {
  border-top: 1px solid var(--border-subtle);
}

/* 未保存警示：左侧短条 + 底色微亮；warn 色在暗底上醒目不刺眼 */
.field-row.dirty {
  background: rgba(198, 144, 38, 0.08);
}

.field-row.dirty::before {
  content: '';
  position: absolute;
  left: 0;
  top: 8px;
  bottom: 8px;
  width: 3px;
  border-radius: 2px;
  background: var(--warn);
}

.row-head {
  display: flex;
  align-items: baseline;
  gap: 10px;
  flex-wrap: wrap;
}

.row-label {
  font-size: 13px;
  font-weight: 500;
}

.row-hint {
  font-size: 12px;
  color: var(--text-3);
}

.row-control {
  margin-top: 8px;
}
</style>
