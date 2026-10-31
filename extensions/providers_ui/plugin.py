# loquat-plugin
# WebUI「服务」页（原内置页迁移为插件，契约见 docs/ext-ui.md）。
# 页面源码在 ui-src/，构建产物为 ui.js（webui/ 下 npm run plugins）。
# 不参与消息路由：handle 恒返回空列表，责任链继续。

TABS = [
    {"id": "main", "title": "服务", "icon": "Coin", "order": 40},
]


def handle(ctx):
    return []
