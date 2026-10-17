# extensions/qa/plugin.py：日常问答插件。
# 约定：handle(text) -> list[str]，返回空列表 = 不归我管，路由链继续。
# 可选声明 PRIORITY（越小越先被路由，缺省 100）。
#
# 全局 config 由宿主注入（来自本目录 config.yaml），规则全在配置里；
# 全局 core 是宿主注入的能力桥（Go 核心能力）。

PRIORITY = 100

# config 由宿主注入（来自本目录 config.yaml），规则全在配置里
config: dict


def handle(text: str) -> list[str]:
    t = _normalize(text)
    if not t:
        return []
    rules = config.get("rules", {})  # noqa: F821 —— config 由宿主运行时注入
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
