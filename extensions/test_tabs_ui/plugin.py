# loquat-plugin
# WebUI 侧栏压测插件（临时，验证完整个目录删掉即可）：
# 连续 8 个 tab 把侧栏撑满，用来看多选项卡时导航的排布与滚动表现。
# 不参与消息路由：handle 恒返回空列表，责任链继续。

TABS = [
    {"id": "t1", "title": "测试 A", "icon": "Bell", "order": 61},
    {"id": "t2", "title": "测试 B", "icon": "Clock", "order": 62},
    {"id": "t3", "title": "测试 C", "icon": "Collection", "order": 63},
    {"id": "t4", "title": "测试 D", "icon": "Files", "order": 64},
    {"id": "t5", "title": "测试 E", "icon": "Folder", "order": 65},
    {"id": "t6", "title": "测试 F", "icon": "HomeFilled", "order": 66},
    {"id": "t7", "title": "测试 G", "icon": "List", "order": 67},
    {"id": "t8", "title": "测试 H", "icon": "Star", "order": 68},
]


def handle(ctx):
    return []
