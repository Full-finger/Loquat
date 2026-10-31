# loquat-plugin
# WebUI「沙箱」页（原内置页迁移为插件，契约见 docs/ext-ui.md）。
# 开发沙箱：与 adapter-sandbox 直聊，验证消息链路。
# 页面源码在 ui-src/，构建产物为 ui.js（webui/ 下 npm run plugins）。
# 不参与消息路由：handle 恒返回空列表，责任链继续。

TABS = [
    {"id": "main", "title": "沙箱", "icon": "Monitor", "order": 60},
]


def handle(ctx):
    return []
