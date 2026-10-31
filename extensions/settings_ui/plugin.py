# loquat-plugin
# WebUI「设置」页（契约见 docs/ext-ui.md）。
# 编辑 UI 偏好（主题色、落地页、访问 token），契约实现在
# webui/src/ext/prefs.ts：壳启动时应用，本页改动即时生效。
# 不参与消息路由：handle 恒返回空列表，责任链继续。

TABS = [
    {"id": "main", "title": "设置", "icon": "Operation", "order": 90},
]


def handle(ctx):
    return []
