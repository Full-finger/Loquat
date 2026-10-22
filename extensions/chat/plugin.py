# loquat-plugin
# extensions/chat/plugin.py：LLM 对话插件（OpenAI 兼容 /chat/completions）。
# 本插件保留：触发规则（群聊触发词/@机器人）、SCHEMA 配置、工具声明；
# 会话历史、QQ→LLM 文本转换、工具循环归内核库 loquat_agent（跨热重载存活），
# HTTP 细节归 loquat_llm。

import json
from datetime import datetime

import loquat_agent
import loquat_cfglib
import loquat_llm

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
    {"key": "max_turns", "type": "int", "default": 6, "min": 1,
     "desc": "单次会话的工具循环轮数上限，防失控"},
]

@loquat_agent.tool
def now() -> str:
    """当前时间（年-月-日 时:分:秒 星期）"""
    return datetime.now().astimezone().strftime("%Y-%m-%d %H:%M:%S %A")


@loquat_agent.tool
def system_status() -> str:
    """宿主机与机器人核心的运行状态（JSON 文本）"""
    return core.system_status()  # noqa: F821 —— core 为宿主注入的能力桥


@loquat_agent.tool
def send_message(ctx, message_type: str, chat_id: int, text: str) -> str:
    """主动发一条独立消息到指定会话，不等本轮回复。

    message_type: "private" 或 "group"。
    chat_id: 目标会话号（私聊为对方 QQ 号，群聊为群号）。
    text: 消息内容，可含 CQ 码（如 [CQ:image,file=file:///...] 发图）。
    """
    action = "send_private_msg" if message_type == "private" else "send_group_msg"
    key = "user_id" if message_type == "private" else "group_id"
    params = json.dumps({key: chat_id, "message": text}, ensure_ascii=False)
    return core.send_action(action, params, 15_000)  # noqa: F821


@loquat_agent.tool
def recall(ctx, message_id: int) -> str:
    """撤回一条消息。

    message_id 传 send_message 返回结果里的 data.message_id。
    机器人只能撤回自己发的消息（管理员身份可撤他人的，由 NapCat 判定）。
    """
    params = json.dumps({"message_id": message_id})
    return core.send_action("delete_msg", params, 15_000)  # noqa: F821


@loquat_agent.tool
def poke(ctx, user_id: int = 0) -> str:
    """戳一戳对方/群成员。

    私聊戳对方；群聊戳指定成员，user_id 缺省为触发这条消息的人。
    """
    uid = user_id or ctx.get("user_id") or 0
    if ctx.get("message_type") == "group":
        params = json.dumps({"group_id": ctx.get("chat_id"), "user_id": uid})
        return core.send_action("group_poke", params, 15_000)  # noqa: F821
    params = json.dumps({"user_id": uid})
    return core.send_action("friend_poke", params, 15_000)  # noqa: F821


@loquat_agent.tool
def react(ctx, emoji_id: str) -> str:
    """给触发本轮回复的那条消息贴一个表情回应。

    emoji_id 是 QQ 表情 ID 字符串（如 "4" 抱抱、"5" 点赞、"128" 爱心）。
    """
    mid = ctx.get("message_id") or 0
    if not mid:
        return "当前消息缺少 message_id，无法回应"
    params = json.dumps({"message_id": mid, "emoji_id": str(emoji_id)})
    return core.send_action("set_msg_emoji_like", params, 15_000)  # noqa: F821


TOOLS = [now, system_status, send_message, recall, poke, react]


def handle(ctx: dict) -> list[str]:
    cfg = loquat_cfglib.resolve(SCHEMA, config)  # noqa: F821
    text = loquat_agent.clean_text(ctx)
    if not text:
        return []

    # is_at_bot 由宿主解析 message 段得出（校验 qq == self_id，
    # 兼容 CQ 码与数组格式）
    if ctx.get("message_type") == "group" and not (
        ctx.get("is_at_bot") or _hit_trigger(cfg, text)
    ):
        loquat_agent.observe(ctx, text)  # 未触发的群消息也记历史，给模型当上下文
        return []

    reply = loquat_agent.respond(
        ctx, text=text, cfg=cfg, chat_fn=loquat_llm.chat,
        tools=TOOLS, max_turns=cfg.get("max_turns", 6),
    )
    if reply is None:
        return ["（LLM 调用失败，详见控制台日志）"]
    return reply


def _hit_trigger(cfg: dict, text: str) -> bool:
    return any(t and t in text for t in cfg.get("triggers", []))
