# extensions/qa.py：问答业务。宿主调用 handle(text) -> list[str]。
# 只用标准库。返回空列表表示不回复。
# 群聊消息的 / 前缀在这里剥（宿主只负责判断要不要转发）。
#
# 全局 core 是宿主注入的能力桥（Go 核心能力）：
#   core.system_status() -> str  系统状态 JSON

import gc
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

    adapter = d.get("adapter_rss_mb")
    adapter_line = (
        f"适配器子进程：{adapter:.1f} MB" if isinstance(adapter, (int, float))
        else "适配器子进程：未运行"
    )

    return (
        f"Loquat 运行状态\n"
        f"机器：CPU {d['cpu_percent']:.1f}% | 内存 {d['mem_used_percent']:.1f}%"
        f"（共 {d['mem_total_gb']:.1f} GB） | 磁盘 {d['disk_used_percent']:.1f}%"
        f" | 开机 {d['uptime_hours']:.1f} 小时\n"
        f"宿主进程（Rust+Go+Python）：{d['host_rss_mb']:.1f} MB\n"
        f"{adapter_line}\n"
        f"Go 核心：{d['goroutines']} 协程，堆 {d['go_heap_mb']:.1f} MB\n"
        f"Python 解释器：{len(gc.get_objects())} 个跟踪对象\n"
        f"系统：{d['os']} ({d['arch']})"
    )
