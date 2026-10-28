// 子页未保存守卫：路由离开（返回按钮、侧栏切页、浏览器后退）弹确认；
// 刷新/关标签走浏览器原生提示。
import { onUnmounted } from 'vue'
import { onBeforeRouteLeave } from 'vue-router'
import { ElMessageBox } from 'element-plus'

/** dirty 为 true 时拦截一切离开动作，用户确认后放行。 */
export function useDirtyGuard(dirty: () => boolean) {
  onBeforeRouteLeave(async () => {
    if (!dirty()) return true
    try {
      await ElMessageBox.confirm('更改尚未保存，离开后将丢失。', '未保存的更改', {
        type: 'warning',
        confirmButtonText: '不保存并离开',
        cancelButtonText: '留在此页',
      })
      return true
    } catch {
      return false
    }
  })

  const onBeforeUnload = (e: BeforeUnloadEvent) => {
    if (dirty()) {
      e.preventDefault()
      e.returnValue = '' // Chromium 走 returnValue，Firefox 走 preventDefault
    }
  }
  window.addEventListener('beforeunload', onBeforeUnload)
  onUnmounted(() => window.removeEventListener('beforeunload', onBeforeUnload))
}
