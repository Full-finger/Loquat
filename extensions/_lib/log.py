# extensions/_lib/log.py：插件日志库，宿主注册为 loquat_log。
# 插件里：import loquat_log; loquat_log.error("chat", "LLM 调用失败: ...")
# 第一个参数是日志源名（插件名），与 Go 的 core、适配器的 adapter 并列。
# 格式对齐 hclog 文本输出，统一走 stderr，重定向时三边日志可一把抓：
#   2026-10-19T22:17:37.839+0800 [ERROR] chat: LLM 调用失败: 超时

import sys
from datetime import datetime

# hclog 把级别字段对齐到 8 列：[INFO] 后 2 空格，[ERROR] 后 1 空格。
_PAD = 6


def _stamp() -> str:
    now = datetime.now().astimezone()
    return (now.strftime("%Y-%m-%dT%H:%M:%S")
            + f".{now.microsecond // 1000:03d}" + now.strftime("%z"))


def _emit(level: str, source: str, msg: str) -> None:
    print(f"{_stamp()} [{level}]" + " " * (_PAD - len(level))
          + f"{source}: {msg}", file=sys.stderr)


def info(source: str, msg: str) -> None:
    _emit("INFO", source, msg)


def warn(source: str, msg: str) -> None:
    _emit("WARN", source, msg)


def error(source: str, msg: str) -> None:
    _emit("ERROR", source, msg)
