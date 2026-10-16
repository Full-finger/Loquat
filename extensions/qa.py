# extensions/qa.py：问答业务。宿主调用 handle(text) -> list[str]。
# 只用标准库。返回空列表表示不回复。
# 群聊消息的 / 前缀在这里剥（宿主只负责判断要不要转发）。

RULES = {
    "ping": "pong",
    "你是谁": "我是 loquat，一个用 Go+Rust+Python 拼出来的机器人",
    "在吗": "在",
    "原神？": "启动！",
}


def handle(text: str) -> list[str]:
    t = text.strip()
    if t.startswith("/"):
        t = t.lstrip("/").strip()
    if not t:
        return []
    if t in RULES:
        return [RULES[t]]
    if t == "帮助":
        return ["当前关键词：" + "、".join(sorted(RULES)) + "、帮助"]
    return []
