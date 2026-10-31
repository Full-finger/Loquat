# loquat-plugin
# WebUI「仪表盘」页（原内置页迁移为插件，契约见 docs/ext-ui.md）。
# 页面源码在 ui-src/，构建产物为 ui.js（webui/ 下 npm run plugins）。
# TABS 里 order 最小：侧栏第一项，也是「/」的默认落地页（views/Home.vue）。
# 不参与消息路由：handle 恒返回空列表，责任链继续。

TABS = [
    {"id": "main", "title": "仪表盘", "icon": "Odometer", "order": 10},
]


def handle(ctx):
    return []
