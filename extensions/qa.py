# extensions/qa.py：日常问答插件。
# 约定：handle(text) -> list[str]，返回空列表 = 不归我管，路由链继续。
# 可选声明 PRIORITY（越小越先被路由，缺省 100）。
#
# 全局 core 是宿主注入的能力桥（Go 核心能力）。

PRIORITY = 100

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
    if t == "帮助":
        return ["问答关键词：ping、你是谁、在吗、原神？\n状态查询由 status 插件提供：发「状态」"]
    if t in RULES:
        return [RULES[t]]
    return []


def _normalize(text: str) -> str:
    t = text.strip()
    if t.startswith("/"):
        t = t.lstrip("/").strip()
    return t
