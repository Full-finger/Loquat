<script setup lang="ts">
// 迷你趋势线（仪表盘卡片内）：归一化折线 + 渐变面积，无坐标轴。
// points 为每拍增量（速率），全 0 画平线。纯展示，不引图表库。
import { computed } from 'vue'

const props = defineProps<{ points: number[] }>()

const W = 120
const H = 36
const PAD = 3

const path = computed(() => {
  const pts = props.points
  if (pts.length < 2) return ''
  const max = Math.max(...pts, 1) // 全 0 时按 1 归一，画底线
  const step = W / (pts.length - 1)
  const y = (v: number) => H - PAD - (v / max) * (H - PAD * 2)
  const coords = pts.map((v, i) => `${(i * step).toFixed(1)},${y(v).toFixed(1)}`)
  return coords.join(' ')
})

const area = computed(() => {
  if (!path.value) return ''
  return `0,${H} ${path.value} ${W},${H}`
})

const last = computed(() => {
  const pts = props.points
  if (pts.length < 2) return null
  const max = Math.max(...pts, 1)
  return { x: W, y: H - PAD - (pts[pts.length - 1] / max) * (H - PAD * 2) }
})
</script>

<template>
  <svg class="spark" :viewBox="`0 0 ${W} ${H}`" preserveAspectRatio="none" aria-hidden="true">
    <polygon v-if="area" :points="area" class="area" />
    <polyline v-if="path" :points="path" class="line" />
    <circle v-if="last" :cx="last.x" :cy="last.y" r="2.5" class="dot" />
  </svg>
</template>

<style scoped>
.spark {
  width: 100%;
  height: 36px;
  display: block;
}

.area {
  fill: color-mix(in srgb, var(--accent) 14%, transparent);
}

.line {
  fill: none;
  stroke: var(--accent);
  stroke-width: 1.5;
  vector-effect: non-scaling-stroke;
}

.dot {
  fill: var(--accent);
}
</style>
