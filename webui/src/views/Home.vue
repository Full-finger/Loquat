<script setup lang="ts">
// 动态首页：「/」落地到排序后的第一个 tab。插件 tab 异步聚合，等第一份
// 清单到了再跳；聚合失败（宿主瞬断）兜底到内置的插件管理页。
import { onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { refreshExtTabs, useExtTabs } from '../composables/ext-tabs'
import { mergeTabs } from '../tabs'
import { loadPrefs } from '../ext/prefs'

const router = useRouter()
const extTabs = useExtTabs()

onMounted(async () => {
  await refreshExtTabs()
  const sorted = mergeTabs(extTabs.value)
  // 落地页偏好优先；指向的 tab 已不存在（插件被禁）时回退第一个
  const prefs = loadPrefs()
  const home =
    prefs.home && sorted.some((t) => t.path === prefs.home)
      ? prefs.home
      : (sorted[0]?.path ?? '/plugins')
  router.replace(home)
})
</script>

<template>
  <div />
</template>
