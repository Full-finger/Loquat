# extensions/_lib/config.py：插件配置库，宿主注册为 loquat_cfglib。
# 插件里：import loquat_cfglib；声明 SCHEMA 后用
#   cfg = loquat_cfglib.resolve(SCHEMA, config)
# 得到校验并补过默认值的配置 dict。
#
# SCHEMA 字段声明格式（list[dict]）：
#   {"key": "api_base", "type": "string", "default": "...", "desc": "...",
#    "options": ["a", "b"],   # 可选：限定可选值
#    "min": 0, "max": 2}      # 可选：数值范围（仅提示与校验）
# type ∈ string/int/float/bool/list/dict，缺省 string。
# enabled 是宿主保留键，不进 SCHEMA。
# 校验策略：宽松——未知字段忽略、类型/可选值不符告警并回退默认值，
# 宁可带病运行也不把插件打挂。

import json
from pathlib import Path

_TYPES = ("string", "int", "float", "bool", "list", "dict")


def resolve(schema: list, raw: dict) -> dict:
    """按 SCHEMA 校验 raw（宿主注入的 config dict），回填默认值。"""
    raw = raw or {}
    out: dict = {}
    known: set = set()
    for field in schema:
        key = field["key"]
        known.add(key)
        default = field.get("default")
        if key not in raw or raw[key] is None:
            out[key] = default
            continue
        v = raw[key]
        if not _type_ok(v, field.get("type", "string")):
            print(f"[config] {key}: 应为 {field.get('type', 'string')}，"
                  f"实际 {type(v).__name__}，回退默认值")
            out[key] = default
            continue
        opts = field.get("options")
        if opts and v not in opts:
            print(f"[config] {key}: {v!r} 不在可选值 {opts} 内，回退默认值")
            out[key] = default
            continue
        out[key] = _range_warn(key, v, field)
    for key in raw:
        if key not in known and key != "enabled":
            print(f"[config] 忽略未知字段 {key}（不在 SCHEMA 里）")
    return out


def ensure_file(schema: list, dir_path: str, raw: dict) -> bool:
    """config.yaml 缺失时按 SCHEMA 生成带注释的模板。已存在则不动。
    返回是否写了文件。仅供宿主调用。"""
    path = Path(dir_path) / "config.yaml"
    if path.exists():
        return False
    header = (
        "# 本文件由宿主按 plugin.py 的 SCHEMA 生成：改值即可，字段说明以 SCHEMA 为准。\n"
        "# 手写的注释与排版在重新生成时不保留；误删本文件会再生成。\n"
        "# enabled 是宿主保留键。\n\n"
        "enabled: true\n\n"
    )
    path.write_text(header + render(schema, raw), encoding="utf-8")
    return True


# ── 内部 ──────────────────────────────────────────────


def _type_ok(v, t: str) -> bool:
    if t == "string":
        return isinstance(v, str)
    if t == "int":
        return isinstance(v, int) and not isinstance(v, bool)
    if t == "float":
        return isinstance(v, (int, float)) and not isinstance(v, bool)
    if t == "bool":
        return isinstance(v, bool)
    if t == "list":
        return isinstance(v, list)
    if t == "dict":
        return isinstance(v, dict)
    return False


def _range_warn(key: str, v, field: dict):
    lo, hi = field.get("min"), field.get("max")
    if (isinstance(v, (int, float)) and not isinstance(v, bool)
            and ((lo is not None and v < lo) or (hi is not None and v > hi))):
        print(f"[config] {key}: {v} 超出范围 [{lo}, {hi}]")
    return v


def render(schema: list, values: dict) -> str:
    """SCHEMA + 当前值 -> 带注释的 YAML 文本。输出可被标准 YAML 解析。"""
    values = values or {}
    lines: list[str] = []
    for field in schema:
        key = field["key"]
        v = values.get(key, field.get("default"))
        if field.get("desc"):
            lines.append(f"# {field['desc']}")
        note = _note(field)
        if note:
            lines.append(f"# {note}")
        lines.append(f"{key}: {_emit(v)}")
        lines.append("")
    return "\n".join(lines).rstrip("\n") + "\n"


def _note(field: dict) -> str:
    notes = []
    if field.get("options"):
        notes.append("可选值: " + " / ".join(_as_text(o) for o in field["options"]))
    if field.get("min") is not None or field.get("max") is not None:
        notes.append(f"范围: {field.get('min', '-inf')} ~ {field.get('max', 'inf')}")
    return "；".join(notes)


def _as_text(v) -> str:
    return v if isinstance(v, str) else json.dumps(v, ensure_ascii=False)


def _emit(v) -> str:
    """标量/容器 -> YAML 文本。字符串一律 JSON 引用（合法 YAML），多行用块标量。"""
    if isinstance(v, str):
        if "\n" in v:
            return _block(v)
        return json.dumps(v, ensure_ascii=False)
    if isinstance(v, bool) or v is None:
        return json.dumps(v)
    if isinstance(v, (int, float)):
        return json.dumps(v)
    if isinstance(v, list):
        if not v:
            return "[]"
        if all(isinstance(x, (str, int, float, bool)) for x in v):
            return "[" + ", ".join(json.dumps(x, ensure_ascii=False) for x in v) + "]"
        return json.dumps(v, ensure_ascii=False)
    if isinstance(v, dict):
        if not v:
            return "{}"
        return _dict_block(v, 0)
    return json.dumps(v, ensure_ascii=False)


def _block(s: str) -> str:
    body = "\n".join("    " + line for line in s.rstrip("\n").split("\n"))
    return "|-\n" + body


def _dict_block(d: dict, depth: int) -> str:
    pad = "    " * (depth + 1)
    lines = []
    for k, v in d.items():
        key = json.dumps(str(k), ensure_ascii=False)
        if isinstance(v, dict) and v:
            lines.append(f"{pad}{key}:")
            lines.append(_dict_block(v, depth + 1))
        else:
            lines.append(f"{pad}{key}: {_emit(v)}")
    return "\n".join(lines)
