# extensions/_lib/agent.py：agent 会话引擎，宿主注册为 loquat_agent。
# 内核配套库，非插件（无签名、不进责任链），设计见 docs/agent.md。
# 职责：会话历史管理（跨热重载存活）、QQ 文本 → LLM 文本转换、
# 工具循环（@tool 声明 + dispatch）。
# LLM 传输由调用方注入（chat_fn 契约同 llm_api.chat），本库不含 HTTP。

import inspect
import json
import os
import re
import sys

import loquat_log

# 会话状态锚点：宿主启动时注册的 loquat_state 模块，进程内永不重载。
# _lib 库热重载是整模块替换，模块属性存状态会随重载丢失，故锚在外面。
# 锚点缺失（异常路径）退化为库内 dict：热重载会丢历史，打告警提示。
_state = sys.modules.get("loquat_state")
if _state is None:
    loquat_log.warn("agent", "loquat_state 锚点缺失，历史退化为进程内（热重载即丢）")
    import types

    _state = types.SimpleNamespace(sessions={}, stats={}, persist_dir="data/sessions")

sessions: dict = _state.sessions  # {(message_type, chat_id): [OpenAI 消息, ...]}
# 引擎计数（responds / turns）：agents 控制台命令读。统计口径：
# 跨 worker 并发 += 在 GIL 下可能丢增量，非记账，允许误差。
stats: dict = _state.stats

_CQ_RE = re.compile(r"\[CQ:[^\]]*\]")

# 工具参数的 Python 注解 → OpenAI JSON schema 类型；无注解按 string。
_PY_TYPES = {
    str: "string",
    int: "integer",
    float: "number",
    bool: "boolean",
    list: "array",
    dict: "object",
}


class Tool:
    """@tool 装饰的函数：携带 OpenAI tools schema，respond 时随参传入。

    工具不做全局注册：热重载后旧引用自然消失，无需清理逻辑。
    参数 ctx 由 dispatch 注入当前会话上下文，不进 schema；
    docstring 首段作 description。
    """

    def __init__(self, fn):
        self.fn = fn
        self.name = fn.__name__
        self.description = (inspect.getdoc(fn) or "").split("\n\n")[0].strip()
        sig = inspect.signature(fn)
        self._wants_ctx = "ctx" in sig.parameters
        self.parameters: dict = {"type": "object", "properties": {}, "required": []}
        for pname, param in sig.parameters.items():
            if pname == "ctx":
                continue
            self.parameters["properties"][pname] = {
                "type": _PY_TYPES.get(param.annotation, "string")
            }
            if param.default is inspect.Parameter.empty:
                self.parameters["required"].append(pname)

    def schema(self) -> dict:
        return {
            "type": "function",
            "function": {
                "name": self.name,
                "description": self.description,
                "parameters": self.parameters,
            },
        }


def tool(fn):
    """装饰器：把函数声明为 agent 工具（respond 的 tools 参数用）。"""
    return Tool(fn)


def clean_text(ctx: dict) -> str:
    """剥离 CQ 码后的纯文本；触发词判断与历史记录共用。"""
    return _CQ_RE.sub("", ctx.get("text") or "").strip()


def observe(ctx: dict, text: str) -> None:
    """记一条用户消息进会话历史，不产生回复（群聊未触发场景）。"""
    content = _llm_text(ctx, text)
    if content:
        key = _key(ctx)
        _append(key, {"role": "user", "content": content})
        _persist(key)


def respond(
    ctx: dict,
    *,
    text: str,
    cfg: dict,
    chat_fn,
    tools: tuple = (),
    max_turns: int = 6,
) -> list[str] | None:
    """跑一次会话轮：记用户消息 → 工具循环 → 记最终回复。

    text 应为 clean_text 的结果。chat_fn(cfg, messages, tools) -> Resp | None，
    契约同 llm_api.chat（system 消息由 chat_fn 拼，不入历史）。
    tools 为 @tool 声明的工具集；超过 max_turns 强制收尾并告警。
    返回 [最终回复]；LLM 失败返回 None，本轮新增的消息全部回滚。
    """
    content = _llm_text(ctx, text)
    if not content:
        return []
    stats["responds"] = stats.get("responds", 0) + 1
    history = _append(_key(ctx), {"role": "user", "content": content})
    start = len(history) - 1  # 本轮起点：失败时回滚到这里

    registry = {t.name: t for t in tools}
    schema = [t.schema() for t in tools]
    reply = None
    for _ in range(max_turns):
        stats["turns"] = stats.get("turns", 0) + 1
        resp = chat_fn(cfg, history, schema or None)
        if resp is None:
            del history[start:]  # LLM 失败：本轮新增全部不留底
            return None
        if not resp.tool_calls:
            reply = resp.text
            if reply:
                history.append({"role": "assistant", "content": reply})
                _trim(cfg, _key(ctx))
                _persist(_key(ctx))
            break
        # 带 tool_calls 的 assistant 原样回填，逐个执行、结果作为 tool 消息。
        # 工具轮中途不 trim：裁剪可能拆散 tool 与其配对 assistant
        history.append(
            {"role": "assistant", "content": resp.text or None, "tool_calls": resp.tool_calls}
        )
        for tc in resp.tool_calls:
            result = _dispatch(registry, tc, ctx)
            history.append({"role": "tool", "tool_call_id": tc["id"], "content": result})
    else:
        loquat_log.warn("agent", f"工具循环超过 max_turns={max_turns}，强制收尾")
    return [reply] if reply else []


def _key(ctx: dict) -> tuple:
    return (ctx.get("message_type"), ctx.get("chat_id"))


# 会话历史落盘（原子写）：每次历史变化后调用，失败只告警不断链。
# 聊天频率低，单会话 JSON 毫秒级，不值得引入定时器。
# 目录每次从锚点读：宿主（及测试）可在运行时改 persist_dir。
def _persist(key: tuple) -> None:
    try:
        sess_dir = getattr(_state, "persist_dir", "data/sessions")
        os.makedirs(sess_dir, exist_ok=True)
        path = os.path.join(sess_dir, f"{key[0]}_{key[1]}.json")
        tmp = path + ".tmp"
        with open(tmp, "w", encoding="utf-8") as f:
            json.dump(
                {"version": 1, "messages": sessions.get(key, [])},
                f,
                ensure_ascii=False,
            )
        os.replace(tmp, path)
    except OSError as e:
        loquat_log.warn("agent", f"会话历史落盘失败 {key}: {e}")


# 群聊历史带发言人前缀，私聊原样：给模型区分同一群里不同发言人。
def _llm_text(ctx: dict, text: str) -> str:
    if not text:
        return ""
    if ctx.get("message_type") == "group":
        return f"<{ctx.get('user_id')}> {text}"
    return text


def _append(key: tuple, msg: dict) -> list:
    history = sessions.setdefault(key, [])
    history.append(msg)
    return history


def _dispatch(registry: dict, tc: dict, ctx: dict) -> str:
    """执行单个工具调用；任何失败转错误文本回填，让模型自行调整。"""
    spec = tc.get("function") or {}
    tool = registry.get(spec.get("name", ""))
    if tool is None:
        return f"错误：未知工具 {spec.get('name')!r}"
    try:
        args = json.loads(spec.get("arguments") or "{}")
    except json.JSONDecodeError as e:
        return f"错误：工具参数不是合法 JSON: {e}"
    if not isinstance(args, dict):
        return "错误：工具参数必须是 JSON 对象"
    kwargs = dict(args)
    if tool._wants_ctx:
        kwargs["ctx"] = ctx
    try:
        result = tool.fn(**kwargs)
    except TypeError as e:
        return f"错误：参数不匹配: {e}"
    except Exception as e:  # noqa: BLE001 —— 有意兜底：工具异常回填模型重试
        return f"错误：{type(e).__name__}: {e}"
    if isinstance(result, str):
        return result
    return json.dumps(result, ensure_ascii=False)


def _trim(cfg: dict, key: tuple) -> None:
    limit = cfg.get("history_limit", 20)
    history = sessions.get(key)
    if not history or len(history) <= limit:
        return
    del history[: len(history) - limit]
    # 裁剪点可能切在工具链中间：开头孤儿 tool 消息（前面没有带
    # tool_calls 的 assistant）会被 API 拒绝，删到边界为止
    while history and history[0].get("role") == "tool":
        history.pop(0)
