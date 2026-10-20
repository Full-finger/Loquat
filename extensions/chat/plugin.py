# loquat-plugin
# extensions/chat/plugin.py：LLM 对话插件（OpenAI 兼容 /chat/completions）。
# 私聊全部回话；群聊仅当消息含触发词或 @机器人 时回话，
# 未触发的群消息也记入会话历史，给模型当上下文。
# 历史在内存里，重启即清空；每会话上限 history_limit 条。
# HTTP 细节在本地模块 llm_api.py（同目录 import，宿主挂 sys.path）。

import re

import loquat_cfglib
from llm_api import chat as _http_chat

PRIORITY = 200  # 兜底，排在 qa/status 之后

# config 由宿主注入（本目录 config.yaml 的原始值；enabled 为宿主保留键）
config: dict

SCHEMA = [
    {"key": "api_base", "type": "string", "default": "https://api.deepseek.com/v1",
     "desc": "OpenAI 兼容后端地址（含 /v1）"},
    {"key": "api_key", "type": "string", "default": "",
     "desc": "API 密钥"},
    {"key": "model", "type": "string", "default": "deepseek-chat",
     "desc": "模型名"},
    {"key": "triggers", "type": "list", "default": ["祥子"],
     "desc": "群聊触发词：消息含任一词或 @机器人 才回话；私聊全部回话"},
    {"key": "system_prompt", "type": "string",
     "default": "你是一个友好的聊天机器人。回复保持简短、口语化。",
     "desc": "人设（system prompt）"},
    {"key": "temperature", "type": "float", "default": 0.8, "min": 0, "max": 2,
     "desc": "采样温度"},
    {"key": "max_tokens", "type": "int", "default": 1024,
     "desc": "单次回复 token 上限"},
    {"key": "history_limit", "type": "int", "default": 20,
     "desc": "每会话（私聊/各群各一份）保留的最大消息条数"},
    {"key": "timeout_seconds", "type": "int", "default": 60,
     "desc": "请求超时秒数"},
]

_histories: dict = {}  # (message_type, chat_id) -> [{"role", "content"}, ...]

_CQ_RE = re.compile(r"\[CQ:[^\]]*\]")


def handle(ctx: dict) -> list[str]:
    cfg = loquat_cfglib.resolve(SCHEMA, config)  # noqa: F821
    raw = ctx.get("text") or ""
    is_at = f"[CQ:at,qq={ctx.get('self_id')}]" in raw
    text = _CQ_RE.sub("", raw).strip()
    if not text:
        return []

    key = (ctx.get("message_type"), ctx.get("chat_id"))
    if ctx.get("message_type") == "group":
        if not (is_at or _hit_trigger(cfg, text)):
            _push(key, f"<{ctx.get('user_id')}> {text}")
            return []
        content = f"<{ctx.get('user_id')}> {text}"
    else:
        content = text

    history = _push(key, content)
    reply = _http_chat(cfg, history)
    if reply is None:
        history.pop()  # 失败的这条不留底
        return ["（LLM 调用失败，详见控制台日志）"]
    history.append({"role": "assistant", "content": reply})
    _trim(cfg, key)
    return [reply]


def _hit_trigger(cfg: dict, text: str) -> bool:
    return any(t and t in text for t in cfg.get("triggers", []))


def _push(key, content: str) -> list:
    history = _histories.setdefault(key, [])
    history.append({"role": "user", "content": content})
    return history


def _trim(cfg: dict, key) -> None:
    limit = cfg.get("history_limit", 20)
    history = _histories.get(key)
    if history and len(history) > limit:
        del history[: len(history) - limit]
