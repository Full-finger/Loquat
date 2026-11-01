# extensions/_lib/llm.py：OpenAI 兼容 HTTP 客户端，宿主注册为 loquat_llm。
# 内核配套库，非插件（无签名、不进责任链）。会话引擎 loquat_agent 的
# chat_fn 即本模块的 chat；返回契约见 Resp。

import json
import random
import time
import urllib.error
import urllib.request

import loquat_log

# 一次重试（评审建议 4）：只对 429/5xx/连接类错误，带 200~800ms 抖动；
# 重试仍受本次 timeout 总量约束，剩余不足 1s 不再试。
RETRY_JITTER = (0.2, 0.8)
RETRY_MIN_REMAINING = 1.0


class Resp:
    """/chat/completions 的助手消息：文本与原始 tool_calls（都可空）。

    tool_calls 保留 OpenAI 原始结构（id/type/function），agent 回填
    assistant 消息时原样使用。
    """

    __slots__ = ("text", "tool_calls")

    def __init__(self, message: dict):
        self.text = (message.get("content") or "").strip()
        self.tool_calls = message.get("tool_calls") or []


def chat(cfg: dict, messages: list, tools: list | None = None) -> Resp | None:
    """调一次 /chat/completions；配置问题或请求失败返回 None。

    messages 不含 system（cfg["system_prompt"] 在这里拼头部）；
    tools 为 OpenAI tools schema，None/空则不传（省 token）。
    cfg 所需键：api_base/api_key/model/temperature/max_tokens/system_prompt，
    timeout_seconds 可选。
    """
    if problem := _cfg_problem(cfg):
        loquat_log.error("llm", problem)
        return None
    body = {
        "model": cfg["model"],
        "messages": [{"role": "system", "content": cfg["system_prompt"]}] + messages,
        "temperature": cfg["temperature"],
        "max_tokens": cfg["max_tokens"],
    }
    if tools:
        body["tools"] = tools
    url = cfg["api_base"].rstrip("/") + "/chat/completions"
    req = urllib.request.Request(
        url,
        data=json.dumps(body, ensure_ascii=False).encode("utf-8"),
        headers={
            "Content-Type": "application/json",
            "Authorization": "Bearer " + cfg["api_key"],
        },
        method="POST",
    )
    timeout = cfg.get("timeout_seconds", 60)
    started = time.monotonic()
    for attempt in (0, 1):
        remaining = timeout - (time.monotonic() - started)
        try:
            with urllib.request.urlopen(req, timeout=max(remaining, 0.1)) as resp:
                data = json.loads(resp.read())
            return Resp(data["choices"][0]["message"])
        except Exception as e:  # noqa: BLE001 —— 有意兜底：网络/超时/JSON/键缺失任何失败都降级为 None，不打挂机器人
            if attempt == 0 and _retryable(e) and remaining > RETRY_MIN_REMAINING:
                loquat_log.warn("llm", f"LLM 调用失败，重试一次: {e}")
                time.sleep(random.uniform(*RETRY_JITTER))
                continue
            loquat_log.error("llm", f"LLM 调用失败: {e}")
            return None
    return None


def _retryable(e: Exception) -> bool:
    """429 / 5xx / 连接与超时类错误值得再试一次；4xx 等语义错误重试无益。"""
    if isinstance(e, urllib.error.HTTPError):
        return e.code == 429 or e.code >= 500
    return isinstance(e, (urllib.error.URLError, TimeoutError, ConnectionError))


def _cfg_problem(cfg: dict) -> str | None:
    """请求前的前置校验，返回配置问题描述；无问题返回 None。

    HTTP 头只允许 latin-1：api_key 空、含中文占位符这类问题在编码层
    报错晦涩，提前拦下来给出明确指引。
    """
    key = cfg.get("api_key") or ""
    if not key:
        return "api_key 未配置（插件 config.yaml 里填好后再聊）"
    try:
        ("Bearer " + key).encode("latin-1")
    except UnicodeEncodeError:
        return "api_key 含非 ASCII 字符，疑似还是占位符，请填真实密钥"
    return None
