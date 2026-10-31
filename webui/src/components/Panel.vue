<script setup lang="ts">
// 面板：细描边卡片 + 可选头（标题/描述/右侧操作）。pad 关掉内边距
// 给整幅内容用（表格）。底色描边等视觉 token 留在 theme.css——
// 消息流/沙箱还有裸用 .panel 的特型容器。
withDefaults(defineProps<{ title?: string; desc?: string; pad?: boolean }>(), { pad: true })
</script>

<template>
  <section class="panel" :class="{ 'no-pad': !pad }">
    <div v-if="title || desc || $slots.actions" class="panel-head">
      <div>
        <div v-if="title" class="panel-title">{{ title }}</div>
        <div v-if="desc" class="panel-desc">{{ desc }}</div>
      </div>
      <div><slot name="actions" /></div>
    </div>
    <slot />
  </section>
</template>

<style scoped>
.panel.no-pad {
  padding: 0;
}

.panel-head {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 12px;
}

.panel-title {
  font-size: 14px;
  font-weight: 600;
}

.panel-desc {
  font-size: 12px;
  color: var(--text-3);
  margin-top: 4px;
}
</style>
