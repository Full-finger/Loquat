<script setup lang="ts">
// 圆环图（仪表盘资源占用）：单环双弧，从顶部同一起点顺时针——
// 灰弧 = 系统总占用（percent2），蓝弧叠其上 = 宿主进程占用（percent）；
// 宿主小于总占用的部分露出灰色。超 warnAt 蓝弧换警示色。
// tip 提供时 hover 弹出具体数值。
const props = withDefaults(
  defineProps<{
    percent: number
    percent2?: number
    label: string
    sub?: string
    tip?: string
    warnAt?: number
  }>(),
  { warnAt: Infinity },
)

const R = 32
const C = 2 * Math.PI * R

const main = Math.min(Math.max(props.percent, 0), 100)
const total = Math.min(Math.max(props.percent2 ?? 0, 0), 100)
const hot = props.percent >= props.warnAt
</script>

<template>
  <el-tooltip :content="tip" placement="top" :disabled="!tip">
    <div class="donut">
      <div class="ring">
        <svg viewBox="0 0 80 80" role="img" :aria-label="label">
          <circle class="track" cx="40" cy="40" :r="R" />
          <!-- 先灰（系统总占用）后蓝（宿主），蓝叠灰之上 -->
          <circle
            v-if="percent2 !== undefined"
            class="fill-total"
            cx="40"
            cy="40"
            :r="R"
            :stroke-dasharray="`${(C * total) / 100} ${C}`"
          />
          <circle
            class="fill"
            :class="{ hot }"
            cx="40"
            cy="40"
            :r="R"
            :stroke-dasharray="`${(C * main) / 100} ${C}`"
          />
        </svg>
        <div class="center">
          <span class="value mono">{{ label }}</span>
        </div>
      </div>
      <div v-if="sub" class="sub">{{ sub }}</div>
    </div>
  </el-tooltip>
</template>

<style scoped>
.donut {
  text-align: center;
}

.ring {
  position: relative;
  width: 96px;
  height: 96px;
}

svg {
  width: 96px;
  height: 96px;
  transform: rotate(-90deg); /* 起点从顶部开始 */
}

.track {
  fill: none;
  stroke: var(--bg-frame);
  stroke-width: 8;
}

.fill-total {
  fill: none;
  stroke: var(--text-3);
  stroke-width: 8;
  stroke-linecap: round;
  transition: stroke-dasharray 0.4s ease;
}

.fill {
  fill: none;
  stroke: var(--accent);
  stroke-width: 8;
  stroke-linecap: round;
  transition: stroke-dasharray 0.4s ease, stroke 0.2s ease;
}

.fill.hot {
  stroke: var(--warn);
}

.center {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}

.value {
  font-size: 15px;
  font-weight: 600;
}

.sub {
  margin-top: 8px;
  font-size: 11px;
  color: var(--text-3);
  /* 脚注比圆环宽时溢出居中而非折行，折行会把圆环挤得高低不齐 */
  white-space: nowrap;
}
</style>
