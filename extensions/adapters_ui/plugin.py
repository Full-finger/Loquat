# loquat-plugin
# WebUI「适配器」页（原内置页迁移为插件，契约见 docs/ext-ui.md）。
# 列表 + 详情两级：详情走 tab 子路径 /ext/adapters_ui/detail/<名>，
# 仍可深链与刷新。页面源码在 ui-src/，构建产物为 ui.js。
# 不参与消息路由：handle 恒返回空列表，责任链继续。

TABS = [
    {"id": "main", "title": "适配器", "icon": "Setting", "order": 50},
]


def handle(ctx):
    return []
