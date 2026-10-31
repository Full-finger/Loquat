// 数据加载三件套（loading / 错误 / 刷新）的统一实现，收编各视图
// 手写一遍的 try/catch/toast/finally。失败时保留旧数据（轮询场景
// 瞬断不打断页面）；错误默认 toast，要内联展示错误的页面传
// toast: false 后自取 error。
import { ref, type Ref } from 'vue'
import { ElMessage } from 'element-plus'

export interface UseResourceOptions<T> {
  /** 失败弹 ElMessage（默认开）；内联展示错误的页面关掉后自取 error */
  toast?: boolean
  /** 成功回调：存快照、拆包多份数据、联动刷新（如 refreshExtTabs） */
  onSuccess?: (data: T) => void
}

export function useResource<T>(fetcher: () => Promise<T>, opts: UseResourceOptions<T> = {}) {
  const data = ref() as Ref<T | null>
  const loading = ref(false)
  const error = ref('')

  async function refresh(): Promise<void> {
    loading.value = true
    try {
      const d = await fetcher()
      data.value = d
      error.value = ''
      opts.onSuccess?.(d)
    } catch (e) {
      error.value = String(e)
      if (opts.toast !== false) ElMessage.error(String(e))
    } finally {
      loading.value = false
    }
  }

  return { data, loading, error, refresh }
}
