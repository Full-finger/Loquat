// 插件贡献 tab 的共享状态（模块级单例）：App.vue 渲染侧栏，
// 插件启停/安装等变更点调 refreshExtTabs() 即时更新，另有 10s
// 轮询兜底外部变更源（控制台命令、手改插件目录、其他浏览器窗口）。
import { ref } from 'vue'
import { fetchUiTabs, type UiTab } from '../api'

const extTabs = ref<UiTab[]>([])

export function useExtTabs() {
  return extTabs
}

export async function refreshExtTabs(): Promise<void> {
  try {
    extTabs.value = await fetchUiTabs()
  } catch {
    // 宿主瞬断时保留旧值，恢复后下一轮自然追上
  }
}
