# extensions/status.py：服务器状态插件。
# 数据来自能力桥 core.system_status()（Go 核心 gopsutil 采集），
# 用 Pillow 渲染为状态卡片 PNG，以 CQ base64 图片码回给 NapCat 显示。

import base64
import gc
import io
import json

from PIL import Image, ImageDraw, ImageFont

PRIORITY = 10  # 状态查询优先于日常问答

KEYWORDS = ("状态", "服务器状态")
FONT_CANDIDATES = (
    "C:/Windows/Fonts/msyh.ttc",    # 微软雅黑
    "C:/Windows/Fonts/simhei.ttf",  # 黑体
)


def _font(size: int):
    for path in FONT_CANDIDATES:
        try:
            return ImageFont.truetype(path, size)
        except OSError:
            continue
    return ImageFont.load_default()


def handle(text: str) -> list[str]:
    t = text.strip()
    if t.startswith("/"):
        t = t.lstrip("/").strip()
    if t not in KEYWORDS:
        return []
    d = json.loads(core.system_status())  # noqa: F821 —— core 由宿主运行时注入
    if "error" in d:
        return [f"状态采集失败：{d['error']}"]
    png = _render(d)
    b64 = base64.b64encode(png).decode()
    return [f"[CQ:image,file=base64://{b64}]"]


def _lines(d: dict) -> list[str]:
    return [
        f"机器      CPU {d['cpu_percent']:.1f}%   内存 {d['mem_used_percent']:.1f}%（共 {d['mem_total_gb']:.1f} GB）",
        f"          磁盘 {d['disk_used_percent']:.1f}%   开机 {d['uptime_hours']:.1f} 小时",
        f"宿主进程  {d['host_rss_mb']:.1f} MB（Rust + Go + Python）",
        f"适配器    {_adapter_rss(d)}",
        f"Go 核心   {d['goroutines']} 协程 · 堆 {d['go_heap_mb']:.1f} MB",
        f"Python    {_py_objects()} 个跟踪对象",
        f"系统      {d['os']} ({d['arch']})",
    ]


def _adapter_rss(d: dict) -> str:
    v = d.get("adapter_rss_mb")
    return f"{v:.1f} MB" if isinstance(v, (int, float)) else "未运行"


def _py_objects() -> int:
    return len(gc.get_objects())


def _render(d: dict) -> bytes:
    width, pad = 640, 30
    title_h, line_h = 64, 44
    lines = _lines(d)
    height = pad * 2 + title_h + line_h * len(lines)

    img = Image.new("RGB", (width, height), "#12141c")
    draw = ImageDraw.Draw(img)
    draw.rectangle([0, 0, 6, height], fill="#f5a623")  # 枇杷色边条
    draw.text((pad, pad), "Loquat 运行状态", font=_font(36), fill="#f5a623")

    y = pad + title_h
    for text in lines:
        draw.text((pad, y), text, font=_font(27), fill="#e8eaf0")
        y += line_h

    buf = io.BytesIO()
    img.save(buf, "PNG")
    return buf.getvalue()
