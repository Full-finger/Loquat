# loquat-plugin
# extensions/chat/plugin.py：LLM 对话插件（OpenAI 兼容 /chat/completions）。
# 本插件保留：触发规则（群聊触发词/@机器人）、SCHEMA 配置、工具声明、
# 危险工具的权限门禁；会话历史、QQ→LLM 消息转换、工具循环归内核库
# loquat_agent（跨热重载存活），HTTP 细节归 loquat_llm，联网归 loquat_webtools。

import json
import time
from datetime import datetime

import loquat_agent
import loquat_cfglib
import loquat_llm
import loquat_log
import loquat_webtools

PRIORITY = 200  # 兜底，排在 qa/status 之后

# config 由宿主注入（本目录 config.yaml 的原始值；enabled 为宿主保留键）
config: dict
# providers 由宿主注入（config/providers.yaml 激活项快照）：
# {"llm": {...}, "search": {"provider", "api_key"}}；缺失时用内置缺省兜底
providers: dict

SCHEMA = [
    {"key": "triggers", "type": "list", "default": ["祥子"],
     "desc": "群聊触发词：消息含任一词或 @机器人 才回话；私聊全部回话"},
    {"key": "system_prompt", "type": "string",
     "default": "你是一个友好的聊天机器人。回复保持简短、口语化。",
     "desc": "人设（system prompt）"},
    {"key": "history_limit", "type": "int", "default": 20,
     "desc": "每会话（私聊/各群各一份）保留的最大消息条数"},
    {"key": "max_turns", "type": "int", "default": 6, "min": 1,
     "desc": "单次会话的工具循环轮数上限，防失控"},
]

# LLM / 搜索的接入点与凭据归宿主 config/providers.yaml 统一管理
# （WebUI「服务」选项卡）；这里只留兜底缺省，宿主没注入 providers
# （旧宿主/单测环境）时行为与迁移前一致。
_LLM_DEFAULTS = {
    "api_base": "https://api.deepseek.com/v1",
    "api_key": "",
    "model": "deepseek-chat",
    "temperature": 0.8,
    "max_tokens": 1024,
    "timeout_seconds": 60,
    "vision": False,
}


def _merged_cfg() -> dict:
    """SCHEMA 配置 + provider 层合并：resolve 只管本插件字段，
    LLM 七键与搜索两键由 providers 覆盖。"""
    cfg = loquat_cfglib.resolve(SCHEMA, config)  # noqa: F821
    cfg.update(_LLM_DEFAULTS)
    p = globals().get("providers") or {}
    cfg.update(p.get("llm") or {})
    search = p.get("search") or {}
    cfg["search_provider"] = search.get("provider", "deepseek")
    cfg["search_api_key"] = search.get("api_key", "")
    return cfg

# ── 权限门禁：send_message / recall 影响 bot 之外的世界，须群管理员 ──
# 私聊默认信任（个人 bot 的常见形态）；群聊查 get_group_member_info 的
# role（owner/admin 放行），校验失败按无权限处理（fail closed）。
# 结果缓存 5 分钟：热门群里每条消息都查一次动作往返吃不消。
# 模块级缓存热重载即丢：只影响缓存命中率，不影响正确性。
_ADMIN_TTL_SECONDS = 300
_admin_cache: dict = {}  # {(chat_id, user_id): (到期 monotonic, 是否管理员)}


def _num_id(v):
    """ID 尽力数字化：QQ 系动作（逃生舱）的 ID 字段预期数字。

    ctx 里的 ID 现在是跨平台字符串；能转 int 转 int，
    转不动原样返回（平台扩展场景）。
    """
    try:
        return int(v)
    except (TypeError, ValueError):
        return v


def _is_admin(ctx: dict) -> bool:
    if ctx.get("message_type") != "group":
        return True
    cid, uid = ctx.get("chat_id"), ctx.get("user_id")
    if uid == ctx.get("self_id"):
        return True
    now = time.monotonic()
    hit = _admin_cache.get((cid, uid))
    if hit and now < hit[0]:
        return hit[1]
    try:
        resp = core.send_action(  # noqa: F821 —— core 为宿主注入的能力桥
            "get_group_member_info",
            json.dumps({"group_id": _num_id(cid), "user_id": _num_id(uid)}),
            10_000,
        )
        role = _role_of(resp)
    except Exception as e:  # noqa: BLE001 —— 有意兜底：校验失败按无权限
        loquat_log.warn("chat", f"权限校验失败，按非管理员处理: {e}")
        return False
    ok = role in ("owner", "admin")
    _admin_cache[(cid, uid)] = (now + _ADMIN_TTL_SECONDS, ok)
    return ok


def _role_of(action_resp: str) -> str:
    """从 get_group_member_info 应答 JSON 里取 role，缺失为空串。"""
    try:
        data = json.loads(action_resp).get("data")
    except (json.JSONDecodeError, AttributeError):
        return ""
    if not isinstance(data, dict):
        return ""
    return data.get("role") if isinstance(data.get("role"), str) else ""


@loquat_agent.tool
def now() -> str:
    """当前时间（年-月-日 时:分:秒 星期）"""
    return datetime.now().astimezone().strftime("%Y-%m-%d %H:%M:%S %A")


@loquat_agent.tool
def system_status() -> str:
    """宿主机与机器人核心的运行状态（JSON 文本）"""
    return core.system_status()  # noqa: F821 —— core 为宿主注入的能力桥


@loquat_agent.tool
def web_search(ctx, query: str) -> str:
    """联网搜索。返回编号结果列表（标题、链接、摘要）；
    需要读某个结果的原文时搭配 read_webpage。"""
    cfg = _merged_cfg()
    provider = cfg.get("search_provider", "deepseek")
    try:
        results = loquat_webtools.search(
            query,
            provider=provider,
            api_key=_search_key(cfg, provider),
            api_base=cfg.get("api_base", ""),
        )
    except Exception as e:  # noqa: BLE001 —— 网络失败转错误文本回模型
        return f"错误：搜索失败：{e}"
    if not results:
        return "没有搜到相关结果"
    return "\n".join(
        f"{i}. {r['title']}\n   {r['url']}\n   {r['snippet']}"
        for i, r in enumerate(results, 1)
    )


# deepseek 通道复用 LLM 的 api_key（search_api_key 留空即可）；
# 其余 provider 用 search_api_key（bocha）。
def _search_key(cfg: dict, provider: str) -> str:
    if provider == "deepseek":
        return cfg.get("api_key") or ""
    return cfg.get("search_api_key") or ""


@loquat_agent.tool
def read_webpage(ctx, url: str) -> str:
    """读取网页的正文文本（deepseek 通道返回 Markdown），
    适合搭配 web_search 的链接。"""
    cfg = _merged_cfg()
    try:
        if cfg.get("search_provider", "deepseek") == "deepseek":
            return loquat_webtools.fetch(
                url, provider="deepseek",
                api_key=cfg.get("api_key") or "",
                api_base=cfg.get("api_base", ""),
            )
        return loquat_webtools.fetch(url)
    except Exception as e:  # noqa: BLE001
        return f"错误：读取网页失败：{e}"


@loquat_agent.tool
def send_message(ctx, message_type: str, chat_id: str, text: str) -> str:
    """主动发一条独立消息到指定会话，不等本轮回复。

    message_type: "private" 或 "group"。
    chat_id: 目标会话号字符串（私聊为对方账号，群聊为群号）。
    text: 消息内容，可含 CQ 码（如 [CQ:image,file=file:///...] 发图）。
    仅群管理员可用（防滥用：本工具可向任意会话发消息）。
    """
    if not _is_admin(ctx):
        return "错误：向指定会话发消息需要群管理员权限，已拒绝"
    action = "send_private_msg" if message_type == "private" else "send_group_msg"
    key = "user_id" if message_type == "private" else "group_id"
    params = json.dumps({key: _num_id(chat_id), "message": text}, ensure_ascii=False)
    return core.send_action(action, params, 15_000)  # noqa: F821


@loquat_agent.tool
def recall(ctx, message_id) -> str:
    """撤回一条消息。

    message_id 传 send_message 返回结果里的 data.message_id。
    机器人只能撤回自己发的消息（管理员身份可撤他人的，由 NapCat 判定）。
    仅群管理员可用。
    """
    if not _is_admin(ctx):
        return "错误：撤回消息需要群管理员权限，已拒绝"
    params = json.dumps({"message_id": _num_id(message_id)})
    return core.send_action("delete_msg", params, 15_000)  # noqa: F821


@loquat_agent.tool
def poke(ctx, user_id: int = 0) -> str:
    """戳一戳对方/群成员。

    私聊戳对方；群聊戳指定成员，user_id 缺省为触发这条消息的人。
    """
    uid = _num_id(user_id or ctx.get("user_id") or 0)
    if ctx.get("message_type") == "group":
        params = json.dumps({"group_id": _num_id(ctx.get("chat_id")), "user_id": uid})
        return core.send_action("group_poke", params, 15_000)  # noqa: F821
    params = json.dumps({"user_id": uid})
    return core.send_action("friend_poke", params, 15_000)  # noqa: F821


@loquat_agent.tool
def react(ctx, emoji_id: str) -> str:
    """给触发本轮回复的那条消息贴一个表情回应。

    emoji_id 是 QQ 表情 ID 字符串（如 "4" 抱抱、"5" 点赞、"128" 爱心）。
    """
    mid = _num_id(ctx.get("message_id") or 0)
    if not mid:
        return "当前消息缺少 message_id，无法回应"
    params = json.dumps({"message_id": mid, "emoji_id": str(emoji_id)})
    return core.send_action("set_msg_emoji_like", params, 15_000)  # noqa: F821


TOOLS = [now, system_status, web_search, read_webpage,
         send_message, recall, poke, react]


def handle(ctx: dict) -> list[str]:
    cfg = _merged_cfg()
    text = loquat_agent.clean_text(ctx)
    if not text:
        return []

    # is_at_bot 由适配器判定（at 段 target == bot 自身 ID），经 ctx 透传
    if ctx.get("message_type") == "group" and not (
        ctx.get("is_at_bot") or _hit_trigger(cfg, text)
    ):
        loquat_agent.observe(ctx, text, cfg)  # 未触发的群消息也记历史，给模型当上下文
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
