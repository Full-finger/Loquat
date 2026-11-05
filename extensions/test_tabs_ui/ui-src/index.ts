// 侧栏压测插件入口：8 个 tab 共用一套占位页面，只差标题。
// definePage 必须防重复：热更后宿主重新 import 本文件。
import { h } from 'vue'
import { definePage } from '@webui/ext/mount'

function makePage(n: string) {
  return () =>
    h('div', { style: 'max-width:720px;padding:24px;' }, [
      h('h2', { style: 'margin:0 0 8px;font-size:16px;' }, `测试页 ${n}`),
      h('p', { style: 'margin:0;color:var(--text-3,#8b93a1);font-size:13px;' },
        '占位页面：用于验证选项卡很多时侧边栏的排布与滚动表现，验证完可整目录删除。'),
    ])
}

for (const n of ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H']) {
  definePage(`loquat-page-test_tabs_ui-t${n.charCodeAt(0) - 64}`, makePage(n))
}
