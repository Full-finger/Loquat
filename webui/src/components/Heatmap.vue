<script setup lang="ts">
// GitHub 风格活跃热力图：列 = 周（周一起），行 = 星期，颜色深浅 = 当天消息量。
// 数据来自 /api/status 的 daily（后端按天持久化，见 src/stats.rs 的 DAILY）。
import { computed } from 'vue'

const props = withDefaults(
  defineProps<{
    daily?: Record<string, [number, number]>
    weeks?: number
    metric?: 'events' | 'replies'
  }>(),
  { weeks: 16, metric: 'events' },
)

// 本地时区 YYYY-MM-DD，对齐后端 daily 的 key 生成方式
function dayKey(d: Date): string {
  const p = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`
}

// 从本周周一往回数 weeks 周；今天之后的格子不画（未来没有数据）
const cells = computed(() => {
  const now = new Date()
  const dow = (now.getDay() + 6) % 7 // 周一 = 0
  const cur = new Date(now)
  cur.setDate(now.getDate() - dow - (props.weeks - 1) * 7)

  const out: { key: string; tip: string; level: number }[] = []
  while (cur <= now) {
    const key = dayKey(cur)
    const pair = props.daily?.[key] ?? [0, 0]
    const v = pair[props.metric === 'replies' ? 1 : 0] ?? 0
    const level = v === 0 ? 0 : v < 3 ? 1 : v < 6 ? 2 : v < 12 ? 3 : 4
    out.push({ key, tip: `${cur.getMonth() + 1}月${cur.getDate()}日 · ${v} 条消息`, level })
    cur.setDate(cur.getDate() + 1)
  }
  return out
})

const total = computed(() =>
  props.daily
    ? Object.values(props.daily).reduce((s, p) => s + (p[props.metric === 'replies' ? 1 : 0] ?? 0), 0)
    : 0,
)
</script>

<template>
  <div class="heatmap">
    <div class="grid">
      <el-tooltip
        v-for="c in cells"
        :key="c.key"
        :content="c.tip"
        placement="top"
        :show-after="200"
      >
        <div class="cell" :class="`lv${c.level}`" />
      </el-tooltip>
    </div>
    <div class="legend mono">
      近 {{ weeks }} 周 {{ total }} 条 · <span class="key" />浅至深：3 / 6 / 12 条
    </div>
  </div>
</template>

<style scoped>
.heatmap {
  font-size: 11px;
  color: var(--text-3);
}

/* 7 行一列按列填充：日期顺序 push 进去天然成列，末列不满即"未到的天" */
.grid {
  display: grid;
  grid-auto-flow: column;
  grid-template-rows: repeat(7, 10px);
  grid-auto-columns: 10px;
  gap: 3px;
  width: fit-content;
}

.cell {
  width: 10px;
  height: 10px;
  border-radius: 2px;
  background: var(--bg-frame);
}

.cell.lv1 {
  background: var(--accent);
  opacity: 0.25;
}

.cell.lv2 {
  background: var(--accent);
  opacity: 0.5;
}

.cell.lv3 {
  background: var(--accent);
  opacity: 0.75;
}

.cell.lv4 {
  background: var(--accent);
}

.legend {
  margin-top: 8px;
}
</style>
