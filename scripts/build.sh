#!/usr/bin/env bash
# 构建产物：go 产物进 bin/，Rust 宿主构建后也拷入 bin/（产物位置固定）。
# 用法：build.sh [模式] [目标]
#   模式  debug（默认）/ release（go 加 -trimpath -s -w，cargo --release）
#   目标  all（默认）/ core / adapter / dll / rust
# 环境变量：GO、BIN_DIR（输出目录，默认 bin）、LDFLAGS（dll 链接参数）、
#           V=1（排查：go 打印每条编译命令，cargo 打印 rustc 调用）
set -euo pipefail
cd "$(dirname "$0")/.."
. scripts/common.sh
init_go
BIN_DIR="${BIN_DIR:-bin}"
LDFLAGS="${LDFLAGS:--s -w}"

usage() {
	cat <<EOF
用法：build.sh [模式] [目标]
  模式  debug（默认）/ release
  目标  all（默认）/ core / adapter / dll / rust
  排查  V=1 打印每条编译命令
EOF
}

mode="${1:-debug}"
target="${2:-all}"
GO_ARGS=(-v) # go 默认打印重编的包名，对齐 cargo 观感
CARGO_ARGS=()
[ "${V:-0}" = "1" ] && GO_ARGS+=(-x) && CARGO_ARGS+=(--verbose)
case "$mode" in
debug)
	EXE_LDFLAGS="" # debug 不加链接参数
	CARGO_BIN="target/debug/loquat.exe"
	;;
release)
	GO_ARGS+=(-trimpath)
	EXE_LDFLAGS="-s -w"
	CARGO_ARGS+=(--release)
	CARGO_BIN="target/release/loquat.exe"
	;;
*)
	usage
	die "未知模式：$mode"
	;;
esac

build_go_exe() { # <名> <包>（debug 时 ldflags 为空串，go 视作未设置）
	"$GO" build "${GO_ARGS[@]}" -ldflags="$EXE_LDFLAGS" -o "$BIN_DIR/$1" "$2"
}

build_rust() {
	cargo build "${CARGO_ARGS[@]}"
	cp -f "$CARGO_BIN" "$BIN_DIR/loquat.exe"
}

SECONDS=0
mkdir -p "$BIN_DIR"
case "$target" in
all)
	banner "构建全部产物（$mode）"
	build_go_exe core.exe ./cmd/core
	build_go_exe adapter-napcat.exe ./cmd/adapter-napcat
	"$GO" build "${GO_ARGS[@]}" -tags dll -buildmode=c-shared -ldflags="$LDFLAGS" -o "$BIN_DIR/core.dll" ./cmd/core
	build_rust
	echo "构建完成（$mode，耗时 ${SECONDS}s）：$BIN_DIR/core.exe $BIN_DIR/adapter-napcat.exe $BIN_DIR/core.dll $BIN_DIR/loquat.exe"
	;;
core)
	banner "构建核心 exe（$mode）"
	build_go_exe core.exe ./cmd/core
	;;
adapter)
	banner "构建适配器插件（$mode）"
	build_go_exe adapter-napcat.exe ./cmd/adapter-napcat
	;;
dll)
	banner "构建核心 dll（$mode）"
	"$GO" build "${GO_ARGS[@]}" -tags dll -buildmode=c-shared -ldflags="$LDFLAGS" -o "$BIN_DIR/core.dll" ./cmd/core
	;;
rust)
	banner "构建 Rust 宿主（$mode）"
	build_rust
	echo "构建完成（$mode，耗时 ${SECONDS}s）：$BIN_DIR/loquat.exe"
	;;
*)
	usage
	die "未知目标：$target"
	;;
esac
