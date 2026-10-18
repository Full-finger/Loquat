#!/usr/bin/env python
# ────────────────────────────────────────────────────────────────
# checklist.py — checkList 生命周期工具（解析 / 校验 / 聚合 / 回填 / 新建）
#
# 子命令：
#   check            校验所有清单：勾选 ⟷ 完成情况表 ⟷ 哈希可追溯
#   digest           生成 docs/checkList/OPEN.md 未解决总览（勿手编）
#   sync [--write]   把完成情况表 / git log 里的提交哈希回填正文状态行
#                    （默认 dry-run，--write 才落盘）
#   new              按今天的日期与序号生成下一份清单骨架
#
# 约定（新清单骨架里会带上，见 new）：
#   ID = 文件号#条目号（如 26101801#1），跨文件引用与 commit message 都写全
#   reopen：把 [x] 翻回 [ ]，状态行追加 `reopen ×N（原 abc1234，原因）`
#   哈希提取自完成情况表的提交列；正文状态行缺哈希时由 sync 回填
# ────────────────────────────────────────────────────────────────
import re
import sys
from dataclasses import dataclass, field
from datetime import date
from pathlib import Path

sys.stdout.reconfigure(encoding="utf-8", errors="replace")

ROOT = Path(__file__).resolve().parent.parent
CKPT_DIR = ROOT / "docs" / "checkList"
OPEN_MD = CKPT_DIR / "OPEN.md"

LEVELS = {"🔴": "HIGH", "🟡": "MED", "🟢": "LOW"}
LEVEL_ORDER = {"HIGH": 0, "MED": 1, "LOW": 2}
HEX = re.compile(r"\b[0-9a-f]{6,7}\b")
ITEM_HEAD = re.compile(r"^### (\d+)\. (.+?)\s*$")
CHECKBOX = re.compile(r"^- \[([ xX])\] \*\*位置\*\*")
STATUS_LINE = re.compile(r"^- \*\*状态\*\*：(.*)$")
TABLE_ROW = re.compile(r"^\|\s*(\d+)\s*\|([^|]*)\|([^|]*)\|([^|]*)\|\s*$")
DEFER_ITEM = re.compile(r"^- \[[xX]\] \*\*(.+?)\*\*")


@dataclass
class Item:
    fid: str          # 所属文件号，如 26101701
    num: int
    title: str
    level: str        # HIGH / MED / LOW
    done: bool
    status: str = ""  # 正文状态行内容（可无）
    table_status: str = ""
    table_commit: str = ""
    line_no: int = 0

    @property
    def uid(self) -> str:
        return f"{self.fid}#{self.num}"

    @property
    def reopened(self) -> bool:
        return not self.done and "reopen" in self.status

    @property
    def commit(self) -> str:
        m = HEX.search(self.status)
        return m.group(0) if m else ""


@dataclass
class Doc:
    fid: str
    title: str
    items: list = field(default_factory=list)
    deferred: list = field(default_factory=list)  # 暂解决策标题
    table_nums: set = field(default_factory=set)


def parse_doc(path: Path) -> Doc:
    text = path.read_text(encoding="utf-8")
    doc = Doc(fid=path.stem, title=text.splitlines()[1].lstrip("# ").strip()
              if len(text.splitlines()) > 1 else path.stem)
    level, section = "", ""
    cur: Item | None = None
    for i, line in enumerate(text.splitlines(), 1):
        if line.startswith("## "):
            section = line
            for emoji, lv in LEVELS.items():
                if emoji in line:
                    level = lv
            cur = None
        m = ITEM_HEAD.match(line)
        if m and level:
            cur = Item(fid=doc.fid, num=int(m.group(1)), title=m.group(2),
                       level=level, done=False, line_no=i)
            doc.items.append(cur)
            continue
        if "有意暂缓" in section and not line.startswith("##"):
            dm = DEFER_ITEM.match(line)
            if dm:
                doc.deferred.append(dm.group(1))
            continue
        if cur is not None:
            cm = CHECKBOX.match(line)
            if cm:
                cur.done = cm.group(1) in "xX"
                continue
            sm = STATUS_LINE.match(line)
            if sm:
                cur.status = sm.group(1).strip()
                continue
        if "完成情况" in section:
            tm = TABLE_ROW.match(line)
            if tm:
                num = int(tm.group(1))
                doc.table_nums.add(num)
                for it in doc.items:
                    if it.num == num:
                        it.table_status = tm.group(3).strip()
                        it.table_commit = tm.group(4).strip()
    return doc


def load_all() -> list:
    return sorted(
        (parse_doc(p) for p in CKPT_DIR.glob("????????.md")),
        key=lambda d: d.fid,
    )


# ── check ─────────────────────────────────────────────────────

def cmd_check() -> int:
    problems = []
    docs = load_all()
    for doc in docs:
        where = doc.fid
        items = {it.num: it for it in doc.items}
        if not items:
            problems.append(f"{where}: 没有解析到任何条目（格式漂移？）")
        for n in set(items) - doc.table_nums:
            problems.append(f"{where}#{n}: 正文有条目但完成情况表缺行")
        for n in doc.table_nums - set(items):
            problems.append(f"{where}: 表格行 #{n} 没有对应正文条目")
        for it in doc.items:
            table_open = "待修" in it.table_status or it.table_status == ""
            if it.done and table_open:
                problems.append(
                    f"{it.uid}: 正文已勾 [x] 但表格状态为「{it.table_status or '空'}」")
            if not it.done and not table_open and "reopen" not in it.table_status:
                problems.append(
                    f"{it.uid}: 正文 [ ] 但表格非待修（{it.table_status}）——"
                    f"若是回归请翻勾并在状态行标 reopen")
            if it.done and not (HEX.search(it.table_commit)
                                or "本次提交" in it.table_commit):
                problems.append(
                    f"{it.uid}: 已勾 [x] 但表格提交列（{it.table_commit}）"
                    f"既无哈希也无「本次提交」，追溯断链")
            if it.reopened and "reopen" not in it.table_status:
                problems.append(f"{it.uid}: 正文状态行标了 reopen，表格状态请同步")
    if problems:
        print(f"[checklist] ✗ {len(problems)} 处不一致：")
        for p in problems:
            print(f"  - {p}")
        return 1
    n_open = sum(1 for d in docs for i in d.items if not i.done)
    print(f"[checklist] ✓ 全部清单一致（未解决 {n_open} 项）")
    return 0


# ── digest ────────────────────────────────────────────────────

def cmd_digest() -> int:
    docs = load_all()
    open_items = [i for d in docs for i in d.items if not i.done]
    reopened = [i for i in open_items if i.reopened]
    todo = [i for i in open_items if not i.reopened]
    closed = sum(1 for d in docs for i in d.items if i.done)
    deferred = sum(len(d.deferred) for d in docs)

    lines = [
        "# 未解决事项（自动生成，勿手编）",
        "",
        (f"> 生成于 {date.today().isoformat()}；来源 {len(docs)} 份清单："
         f"待修 {len(todo)} · reopen {len(reopened)} · "
         f"已关闭 {closed} · 暂缓决策 {deferred}"),
        "",
    ]
    by_level = {lv: [i for i in todo if i.level == lv]
                for lv in ("HIGH", "MED", "LOW")}
    heads = {"HIGH": "## 🔴 高优先级", "MED": "## 🟡 中优先级",
             "LOW": "## 🟢 低优先级"}
    for lv in ("HIGH", "MED", "LOW"):
        lines.append(heads[lv])
        lines.append("")
        if not by_level[lv]:
            lines.append("（无）")
            lines.append("")
            continue
        lines.append("| ID | 标题 | 来源 |")
        lines.append("|----|------|------|")
        for it in sorted(by_level[lv], key=lambda x: x.uid):
            lines.append(f"| {it.uid} | {it.title} | {it.fid}.md |")
        lines.append("")
    lines += ["## ⚠ reopen（修过又回来了）", ""]
    if reopened:
        lines.append("| ID | 标题 | 来源 | 状态行 |")
        lines.append("|----|------|------|--------|")
        for it in reopened:
            brief = it.status if len(it.status) <= 60 else it.status[:57] + "…"
            lines.append(f"| {it.uid} | {it.title} | {it.fid}.md | {brief} |")
    else:
        lines.append("（无）")
    lines.append("")
    OPEN_MD.write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
    print(f"[checklist] OPEN.md 已生成：待修 {len(todo)} · reopen {len(reopened)}")
    return 0


# ── sync ──────────────────────────────────────────────────────

def cmd_sync(write: bool) -> int:
    changed = 0
    for doc in load_all():
        # 只回填：已勾 [x]、正文状态行无哈希、但完成情况表里有哈希的条目
        targets = [i for i in doc.items
                   if i.done and not i.commit and HEX.search(i.table_commit)]
        if not targets:
            continue
        path = CKPT_DIR / f"{doc.fid}.md"
        lines = path.read_text(encoding="utf-8").split("\n")
        for it in targets:
            h = HEX.search(it.table_commit).group(0)
            # 从条目标题行往下找本条目块内的状态行（30 行内）
            head = next((idx for idx, ln in enumerate(lines)
                         if ln.startswith(f"### {it.num}. ")), None)
            if head is None:
                continue
            for idx in range(head, min(head + 30, len(lines))):
                if STATUS_LINE.match(lines[idx]):
                    if h in lines[idx]:
                        break
                    lines[idx] = lines[idx].rstrip() + f"（提交 `{h}`）"
                    print(f"[sync] {it.uid}: 状态行补哈希 {h}"
                          f"{'（已写入）' if write else '（dry-run）'}")
                    changed += 1
                    break
        if write:
            path.write_text("\n".join(lines), encoding="utf-8", newline="\n")
    if not changed:
        print("[sync] 所有已修条目状态行都有哈希，无需回填")
    elif not write:
        print("[sync] dry-run 结束，加 --write 落盘")
    return 0


# ── new ───────────────────────────────────────────────────────

TEMPLATE = """# 审查/评估清单 {date}（{seq}）

- 审查范围：
- 审查方式：
- 命名规则：`YYMMDD` + 两位序号

## 使用说明

- 每项含【位置 / 问题 / 修法 / 验证】，修完把 `[ ]` 勾成 `[x]` 并附提交哈希
- 跨文件引用 ID 用完整「文件号#条目号」（如 {fid}#1），commit message 里也写全
- reopen：把 `[x]` 翻回 `[ ]`，状态行追加 `reopen ×N（原 abc1234，原因）`
- 未解决总览自动生成：`make checklist`（产物 OPEN.md，勿手编）
- 校验：`python scripts/checklist.py check`（已挂 `make lint`）

---

## 🔴 高优先级

### 1. （标题）

- [ ] **位置**：
- **问题**：
- **修法**：
- **验证**：
- **状态**：

---

## 🟡 中优先级

---

## 🟢 低优先级

---

## 📋 有意暂缓（不修，记录决策）

- [x] **（决策）**：（理由）—— **维持现状**

---

## 完成情况

| # | 级别 | 状态 | 提交 |
|---|------|------|------|
| 1 | 🔴 | ⬜ 待修 | — |
"""


def cmd_new() -> int:
    today = date.today().strftime("%y%m%d")
    seq = 1
    while (CKPT_DIR / f"{today}{seq:02d}.md").exists():
        seq += 1
    path = CKPT_DIR / f"{today}{seq:02d}.md"
    path.write_text(TEMPLATE.format(
        date=date.today().isoformat(), seq=f"{seq:02d}",
        fid=f"{today}{seq:02d}"),
        encoding="utf-8", newline="\n")
    print(f"[checklist] 已生成 {path.name}，编辑后记得补完成情况表")
    return 0


def main() -> int:
    args = sys.argv[1:]
    if args[:1] == ["check"]:
        return cmd_check()
    if args[:1] == ["digest"]:
        return cmd_digest()
    if args[:1] == ["sync"]:
        return cmd_sync(write="--write" in args)
    if args[:1] == ["new"]:
        return cmd_new()
    print(__doc__)
    return 2


if __name__ == "__main__":
    sys.exit(main())
