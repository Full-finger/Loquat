<script setup lang="ts">
// 页头只剩右上角操作行：标题/描述已移除，页面位置由顶栏面包屑表达。
// 默认插槽与 #actions 等价（历史两种写法都在用），同时使用会重复渲染
</script>

<template>
  <div class="page-head">
    <div class="actions"><slot /><slot name="actions" /></div>
  </div>
</template>

<style scoped>
.page-head {
  display: flex;
  justify-content: flex-end;
  align-items: center;
  margin-bottom: 20px;
}
</style>
