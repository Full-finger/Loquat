# extensions/qa.py：问答业务。宿主调用 handle(text) -> list[str]。
# 只用标准库。返回空列表表示不回复。
# 群聊消息的 / 前缀在这里剥（宿主只负责判断要不要转发）。
#
# 全局 core 是宿主注入的能力桥（Go 核心能力）：
#   core.system_status() -> str  系统状态 JSON

import json
from typing import Any

# core 由宿主注入（能力桥，Go 核心能力），见 src/business.rs 的 attach_core_bridge
core: Any

RULES = {
    "ping": "pong",
    "你是谁": "我是 loquat，一个用 Go+Rust+Python 拼出来的机器人",
    "在吗": "在",
    "原神？": "启动！",
}


def handle(text: str) -> list[str]:
    t = _normalize(text)
    if not t:
        return []
    if t in ("状态", "服务器状态"):
        return [format_status(core.system_status())]  # noqa: F821 —— core 由宿主运行时注入
    if t == "帮助":
        return ["当前关键词：ping、你是谁、在吗、状态、帮助"]
    if t in RULES:
        return [RULES[t]]
    return []


def _normalize(text: str) -> str:
    t = text.strip()
    if t.startswith("/"):
        t = t.lstrip("/").strip()
    return t


def format_status(raw: str) -> str:
    d = json.loads(raw)
    if "error" in d:
        return f"状态采集失败：{d['error']}"
    return (
        f"系统：{d['os']} ({d['arch']})\n"
        f"开机时长：{d['uptime_hours']:.1f} 小时\n"
        f"CPU：{d['cpu_percent']:.1f}%\n"
        f"内存：已用 {d['mem_used_percent']:.1f}%（共 {d['mem_total_gb']:.1f} GB）\n"
        f"磁盘（工作目录）：已用 {d['disk_used_percent']:.1f}%\n"
        f"核心：{d['goroutines']} 个协程，堆 {d['go_heap_mb']:.1f} MB"
    )
