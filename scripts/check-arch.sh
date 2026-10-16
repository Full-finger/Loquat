#!/usr/bin/env bash
# ────────────────────────────────────────────────────────────────
# check-arch.sh — 架构与边界规则静态检查（纯 bash，零依赖）
#
# 规则：
#   R1 适配器插件的 stdout 归 go-plugin 握手专用，禁止打印（日志走 hclog→stderr）
#   R2 internal/ 是库形态代码（exe/dll 共用），禁止 os.Exit，错误必须返回
#   R3 FFI 事件回调禁止 unwrap/expect——panic 跨 C 边界会直接炸宿主进程
#   R4 单文件不超过 400 行，超了就拆
# ────────────────────────────────────────────────────────────────
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
RED='\033[31m'; GREEN='\033[32m'; RESET='\033[0m'
violations=0

fail() {
	echo -e "${RED}[违规] $1${RESET}"
	violations=$((violations + 1))
}

# R1: 适配器禁止写 stdout（fmt.Fprint* 到 stderr 允许）
if grep -rnE 'fmt\.(Print|Printf|Println)\(' "$ROOT_DIR/cmd/adapter-napcat" --include="*.go"; then
	fail "R1: adapter-napcat 禁止 fmt.Print*/Println（stdout 是 go-plugin 握手通道，日志用 hclog）"
fi

# R2: internal 禁止 os.Exit
if grep -rn "os\.Exit" "$ROOT_DIR/internal" --include="*.go"; then
	fail "R2: internal/ 禁止 os.Exit（错误向上返回，退出只发生在 cmd 的 main）"
fi

# R3: FFI 回调禁止 unwrap/expect
if sed -n '/extern "C" fn on_event/,/^}/p' "$ROOT_DIR/src/main.rs" | grep -q "unwrap\|expect"; then
	fail "R3: on_event 回调禁止 unwrap/expect（panic 跨 C 边界会炸宿主进程）"
fi

# R4: 文件行数上限
while IFS= read -r f; do
	lines=$(wc -l < "$f")
	if [ "$lines" -gt 400 ]; then
		fail "R4: $f 共 ${lines} 行，超过 400 行上限，请拆分"
	fi
done < <(find "$ROOT_DIR/cmd" "$ROOT_DIR/internal" "$ROOT_DIR/src" \( -name "*.go" -o -name "*.rs" \) ! -name "*_test.go")

if [ "$violations" -gt 0 ]; then
	echo -e "${RED}共 ${violations} 条规则违规，见上${RESET}"
	exit 1
fi
echo -e "${GREEN}架构规则：全部通过${RESET}"
