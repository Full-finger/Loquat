<script setup lang="ts">
// 运行状态：色点 + 文本。::before 当色点、颜色跟文字走
// （currentColor），换 state 只换一个 color。
defineProps<{ state: 'on' | 'off' | 'warn' }>()
</script>

<template>
  <span class="state" :class="state"><slot /></span>
</template>

<style scoped>
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

.state.warn {
  color: var(--warn);
}
</style>
