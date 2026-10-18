# extensions/chat/llm_api.py：OpenAI 兼容 HTTP 客户端，被 plugin.py import。
# 独立成文件也是插件多文件约定的示范：逻辑变大时按主题拆，别堆回单文件。

import json
import urllib.request


def chat(cfg: dict, history: list) -> str | None:
    """调一次 /chat/completions；失败打印日志并返回 None。"""
    url = cfg["api_base"].rstrip("/") + "/chat/completions"
    body = {
        "model": cfg["model"],
        "messages": [{"role": "system", "content": cfg["system_prompt"]}] + history,
        "temperature": cfg["temperature"],
        "max_tokens": cfg["max_tokens"],
    }
    req = urllib.request.Request(
        url,
        data=json.dumps(body, ensure_ascii=False).encode("utf-8"),
        headers={
            "Content-Type": "application/json",
            "Authorization": "Bearer " + cfg["api_key"],
        },
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=cfg.get("timeout_seconds", 60)) as resp:
            data = json.loads(resp.read())
        return data["choices"][0]["message"]["content"].strip()
    except Exception as e:  # noqa: BLE001 —— 有意兜底：网络/超时/JSON/键缺失任何失败都降级为 None，不打挂机器人
        print(f"[chat] LLM 调用失败: {e}")
        return None
