#!/usr/bin/env bash
# common.sh — scripts/ 下脚本的公共函数（util 层）。
# 约定：调用方先 cd 到仓库根目录再 source；工具链探测集中在这里，
# 让每个脚本脱离 just 也能独立运行。

RED='\033[31m'; GREEN='\033[32m'; CYAN='\033[36m'; RESET='\033[0m'

# 打印进度横幅
banner() { printf "${CYAN}==> ${GREEN}%s${RESET}\n" "$1"; }

# 报错退出（参数支持 \n 转义）
die() { printf "${RED}${1}${RESET}\n" >&2; exit 1; }

# need <命令> <安装提示>：存在性检查，缺失即退出
need() { command -v "$1" >/dev/null 2>&1 || die "缺少 $1：$2"; }

# init_go：解析 go 工具链到 GO 变量（compile.toml 的 go 字段优先，PATH 兜底），
# 并清空 GOROOT——该环境变量会指向另一套编译器，导致版本不匹配。
init_go() {
	if [ -z "${GO:-}" ]; then
		GO="$(sed -n 's/^[[:space:]]*go[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' compile.toml 2>/dev/null | head -n 1)"
		GO="${GO:-go}"
	fi
	export GOROOT=
}

# gofmt_of <go 路径>：推导同目录的 gofmt；go 是纯命令名时用 PATH 里的 gofmt
gofmt_of() {
	local d
	d="$(dirname "$1")"
	if [ "$d" = "." ]; then
		echo "gofmt"
	else
		echo "$d/gofmt"
	fi
}
