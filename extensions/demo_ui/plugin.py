# loquat-plugin
# WebUI 扩展页演示插件（契约见 docs/ext-ui.md）：
# - TABS 声明侧栏入口，icon 用 element-plus 图标名（可省略）
# - 页面内容在同目录 ui.js（原生 ES module + 自定义元素，免构建）
# - 不参与消息路由：handle 恒返回空列表，责任链继续

TABS = [
    {"id": "demo", "title": "演示页", "icon": "MagicStick"},
]


def handle(ctx):
    return []
