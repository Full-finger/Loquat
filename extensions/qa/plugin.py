# loquat-plugin
# extensions/qa/plugin.py：日常问答插件。
# 约定：handle(ctx) -> list[str]，ctx 含 text/message_type/chat_id/user_id/self_id；
# 返回空列表 = 不归我管，路由链继续。可选声明 PRIORITY（越小越先，缺省 100）。
# 配置走 SCHEMA 声明 + loquat_cfglib 校验（库见 extensions/_lib/config.py）。

import loquat_cfglib

PRIORITY = 100

# config 由宿主注入（本目录 config.yaml 的原始值；enabled 为宿主保留键）
config: dict

SCHEMA = [
    {"key": "rules", "type": "dict", "default": {"ping": "pong"},
     "desc": "精确匹配的关键词 -> 回复"},
]


def handle(ctx: dict) -> list[str]:
    t = _normalize(ctx.get("text") or "")
    if not t:
        return []
    rules = loquat_cfglib.resolve(SCHEMA, config)["rules"]  # noqa: F821
    if t == "帮助":
        return ["问答关键词：" + "、".join(sorted(rules)) + "\n其他能力由其他插件提供"]
    if t in rules:
        return [rules[t]]
    return []


def _normalize(text: str) -> str:
    t = text.strip()
    if t.startswith("/"):
        t = t.lstrip("/").strip()
    return t
