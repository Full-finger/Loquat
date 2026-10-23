# extensions/_lib/webtools.py：联网工具（搜索 + 网页阅读），宿主注册为 loquat_webtools。
# 内核配套库，非插件（无签名、不进责任链）。标准库实现（urllib + html.parser），
# 不引第三方依赖——与 llm.py 同约定。
# 设计：search 三 provider——deepseek（官方 Search API，复用 LLM api_key，国内直连）
# 默认、duckduckgo（HTML 结果页解析，免 key）、bocha（API key）；
# fetch 双通道——deepseek 官方 Fetch（返回 Markdown）或本机 urllib 直抓抽正文。
# 网络与解析分离：*_parse/_html_to_text 是纯函数，fixture 单测不打网。
# 错误约定：一律抛异常，由调用方（chat 插件工具）转错误文本回模型。

import json
import re
import urllib.parse
import urllib.request
from html.parser import HTMLParser

_TIMEOUT = 15
# 防大文件：下载读这么多就停，输出再按字符截断
_MAX_DOWNLOAD = 2 * 1024 * 1024
_MAX_CHARS = 4000
_SNIPPET_CAP = 500
_UA = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Loquat/0.1"


def search(query: str, *, count: int = 5, provider: str = "deepseek",
           api_key: str = "", api_base: str = "") -> list[dict]:
    """联网搜索，返回 [{"title", "url", "snippet"}]；失败抛异常。"""
    q = query.strip()
    if not q:
        return []
    if provider == "deepseek":
        results = _search_deepseek(q, count, api_key, api_base)
    elif provider == "bocha":
        results = _search_bocha(q, count, api_key)
    elif provider == "duckduckgo":
        results = _search_ddg(q, count)
    else:
        raise ValueError(
            f"未知搜索引擎 {provider!r}（可选 deepseek / duckduckgo / bocha）")
    for r in results:
        r["title"] = r["title"][:100]
        r["snippet"] = r["snippet"][:_SNIPPET_CAP]
    return results


def fetch(url: str, *, max_chars: int = _MAX_CHARS, provider: str = "",
          api_key: str = "", api_base: str = "") -> str:
    """读网页：deepseek 通道走官方 Fetch API（返回 Markdown）；
    其余本机直抓——HTML 抽正文，text/plain 与 JSON 原样返回，超限截断。"""
    scheme = urllib.parse.urlparse(url).scheme
    if scheme not in ("http", "https"):
        raise ValueError(f"只支持 http/https 链接，收到 {scheme or '空'!r}")
    if provider == "deepseek":
        return _fetch_deepseek(url, api_key, api_base)[:max_chars]
    req = urllib.request.Request(url, headers={"User-Agent": _UA})
    with urllib.request.urlopen(req, timeout=_TIMEOUT) as resp:
        ctype = (resp.headers.get("Content-Type") or "").lower()
        raw = resp.read(_MAX_DOWNLOAD)
        if not any(m in ctype for m in
                   ("text/html", "text/plain", "application/json", "xhtml")):
            raise ValueError(f"不是文本页面（Content-Type: {ctype or '未知'}）")
        text = raw.decode(_charset(ctype), errors="replace")
    body = _html_to_text(text) if "html" in ctype else text.strip()
    return body[:max_chars]


# ── 网页正文抽取 ──────────────────────────────────────────────

class _TextExtract(HTMLParser):
    """块级标签处断行，跳过 script/style/noscript，收集可见文本。"""

    SKIP = ("script", "style", "noscript")
    BLOCK = ("p", "div", "br", "li", "tr", "h1", "h2", "h3", "h4", "h5", "h6",
             "section", "article", "table", "ul", "ol", "pre", "blockquote")

    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.parts: list[str] = []
        self._skip = 0

    def handle_starttag(self, tag, attrs):
        if tag in self.SKIP:
            self._skip += 1
        elif tag in self.BLOCK:
            self.parts.append("\n")

    def handle_endtag(self, tag):
        if tag in self.SKIP:
            if self._skip:
                self._skip -= 1
        elif tag in self.BLOCK:
            self.parts.append("\n")

    def handle_data(self, data):
        if not self._skip and data.strip():
            self.parts.append(data)


def _html_to_text(html: str) -> str:
    parser = _TextExtract()
    try:
        parser.feed(html)
    except Exception:  # noqa: BLE001, S110 —— 残缺 HTML 尽力抽，不值得上日志
        pass
    text = "".join(parser.parts)
    text = re.sub(r"[ \t\r\f\v]+", " ", text)  # 合并横向空白
    text = re.sub(r"\n\s*\n+", "\n", text)     # 合并空行
    return text.strip()


def _charset(ctype: str) -> str:
    m = re.search(r"charset=([\w-]+)", ctype)
    return m.group(1) if m else "utf-8"


# ── DeepSeek 官方 Search / Fetch（默认，复用 LLM 的 api_key，国内直连）──
# POST /v1/tools/search：query 限 1-128 字符，结果已按相关性排序，
#   summary（600-800 字）是给模型消费的长摘要，优先于 snippet（给 UI 的短摘要）
# POST /v1/tools/fetch：返回 {"title", "content"(markdown)}
# 计费按成功次数（各 5 元/千次），失败/空结果不计费。

def _tools_url(api_base: str) -> str:
    """chat 插件的 api_base（…/v1 或不带）规整成 …/v1，拼 tools 端点用。"""
    base = (api_base or "https://api.deepseek.com").rstrip("/")
    if not base.endswith("/v1"):
        base += "/v1"
    return base


def _search_deepseek(query: str, count: int, api_key: str,
                     api_base: str) -> list[dict]:
    if not api_key:
        raise ValueError(
            "api_key 未配置：deepseek 搜索复用 LLM 的 api_key（chat 插件 config.yaml）")
    body = json.dumps({"query": query[:128], "count": count}).encode()
    req = urllib.request.Request(
        _tools_url(api_base) + "/tools/search", data=body, method="POST",
        headers={"Content-Type": "application/json", "User-Agent": _UA,
                 "Authorization": "Bearer " + api_key})
    with urllib.request.urlopen(req, timeout=_TIMEOUT) as resp:
        return _parse_deepseek(json.loads(resp.read()), count)


def _parse_deepseek(data: dict, count: int) -> list[dict]:
    """Search 应答 -> 统一结果结构；摘要优先 summary，缺失退化 snippet。"""
    results = (data or {}).get("results") or []
    return [{"title": _unmd(p.get("title") or ""), "url": p.get("url") or "",
             "snippet": _unmd(p.get("summary") or p.get("snippet") or "")}
            for p in results[:count]]


def _fetch_deepseek(url: str, api_key: str, api_base: str) -> str:
    if not api_key:
        raise ValueError(
            "api_key 未配置：deepseek 抓页复用 LLM 的 api_key（chat 插件 config.yaml）")
    body = json.dumps({"url": url[:8192]}).encode()
    req = urllib.request.Request(
        _tools_url(api_base) + "/tools/fetch", data=body, method="POST",
        headers={"Content-Type": "application/json", "User-Agent": _UA,
                 "Authorization": "Bearer " + api_key})
    with urllib.request.urlopen(req, timeout=_TIMEOUT) as resp:
        data = json.loads(resp.read())
    title = _unmd((data or {}).get("title") or "")
    content = _unmd((data or {}).get("content") or "")
    return f"# {title}\n\n{content}".strip()


# 官方接口返回的 Markdown 常带转义残留（\/ \# 等），还原成裸字符；
# 只处理常见标点前的反斜杠，正文里的其他反斜杠（如 Windows 路径）不动。
_MD_ESCAPE = re.compile(r"\\([-#*/_.![\]()~`+\\])")


def _unmd(s: str) -> str:
    return _MD_ESCAPE.sub(r"\1", s)


# ── DuckDuckGo（备选，免 key）────────────────────────────────

def _search_ddg(query: str, count: int) -> list[dict]:
    # POST 表单比 GET 稳（GET 偶发拿到要求 JS 的异常页）
    body = urllib.parse.urlencode({"q": query}).encode()
    req = urllib.request.Request(
        "https://html.duckduckgo.com/html/", data=body,
        headers={"User-Agent": _UA}, method="POST")
    with urllib.request.urlopen(req, timeout=_TIMEOUT) as resp:
        html = resp.read(_MAX_DOWNLOAD).decode("utf-8", errors="replace")
    return _parse_ddg(html, count)


def _parse_ddg(html: str, count: int) -> list[dict]:
    """DDG 结果页：result__a 是标题链接（href 走 /l/?uddg= 跳转），
    result__snippet 是摘要，两者同为 <a>，按文档顺序一一配对。"""
    links: list[tuple[str, str]] = []
    snippets: list[str] = []
    for cls, href, text in _collect_anchors(html):
        if "result__snippet" in cls:
            snippets.append(_clean(text))
        elif "result__a" in cls:
            links.append((href, _clean(text)))
    out = []
    for i, (href, title) in enumerate(links[:count]):
        out.append({
            "title": title,
            "url": _ddg_unwrap(href),
            "snippet": snippets[i] if i < len(snippets) else "",
        })
    return out


class _AnchorCollect(HTMLParser):
    """收集全部 <a> 的 (class, href, 拼接文本)，供结果页解析。"""

    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.anchors: list[list] = []  # [class, href, text]
        self._open = -1

    def handle_starttag(self, tag, attrs):
        if tag != "a":
            return
        d = {k: (v or "") for k, v in attrs}
        self.anchors.append([d.get("class", ""), d.get("href", ""), ""])
        self._open = len(self.anchors) - 1

    def handle_endtag(self, tag):
        if tag == "a":
            self._open = -1

    def handle_data(self, data):
        if self._open >= 0:
            self.anchors[self._open][2] += data


def _collect_anchors(html: str) -> list[tuple[str, str, str]]:
    parser = _AnchorCollect()
    try:
        parser.feed(html)
    except Exception:  # noqa: BLE001, S110 —— 同 _html_to_text，尽力解析
        pass
    return [tuple(a) for a in parser.anchors]


def _ddg_unwrap(href: str) -> str:
    if "uddg=" in href:
        real = urllib.parse.parse_qs(
            urllib.parse.urlparse(href).query).get("uddg")
        if real:
            return real[0]
    return "https:" + href if href.startswith("//") else href


# ── 博查（国内直连，需 key）──────────────────────────────────

def _search_bocha(query: str, count: int, api_key: str) -> list[dict]:
    if not api_key:
        raise ValueError(
            "search_api_key 未配置：bocha 需要 key（chat 插件 config.yaml 里填）")
    body = json.dumps({"query": query, "count": count, "summary": True}).encode()
    req = urllib.request.Request(
        "https://api.bochaai.com/v1/web-search", data=body, method="POST",
        headers={"Content-Type": "application/json", "User-Agent": _UA,
                 "Authorization": "Bearer " + api_key})
    with urllib.request.urlopen(req, timeout=_TIMEOUT) as resp:
        data = json.loads(resp.read())
    pages = ((data.get("data") or {}).get("webPages") or {}).get("value") or []
    return [{"title": p.get("name") or "", "url": p.get("url") or "",
             "snippet": p.get("summary") or p.get("snippet") or ""}
            for p in pages[:count]]


def _clean(s: str) -> str:
    return re.sub(r"\s+", " ", s).strip()
