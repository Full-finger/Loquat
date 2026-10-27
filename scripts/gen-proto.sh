#!/usr/bin/env bash
# ────────────────────────────────────────────────────────────────
# gen-proto.sh — 再生成 internal/pluginapi 的 Go 存根（26102202#8）
#
# 前置：protoc、protoc-gen-go、protoc-gen-go-grpc 在 PATH
#   go install google.golang.org/protobuf/cmd/protoc-gen-go@latest
#   go install google.golang.org/google.golang.org/grpc/cmd/protoc-gen-go-grpc@latest
#   winget install Google.Protobuf   # 或从 github.com/protocolbuffers/protobuf releases
#
# 生成代码入库；跑完 git diff 应无变化（幂等），有变化 = proto 与存根曾漂移
# ────────────────────────────────────────────────────────────────
set -euo pipefail
cd "$(dirname "$0")/.."
. scripts/common.sh

banner "protoc 再生成 internal/pluginapi"

need protoc "winget install Google.Protobuf（或从 protobuf releases 下载）"
need protoc-gen-go "go install google.golang.org/protobuf/cmd/protoc-gen-go@latest"
need protoc-gen-go-grpc "go install google.golang.org/grpc/cmd/protoc-gen-go-grpc@latest"

# well-known types 的 include：winget 版在 protoc.exe 同级的上一级，
# 自编译/解压版通常叫 include。找不到时交给 protoc 自带探测。
protoc_dir="$(dirname "$(command -v protoc)")"
includes=(-I.)
for inc in "$protoc_dir/../include" "$protoc_dir/include"; do
	if [ -f "$inc/google/protobuf/struct.proto" ]; then
		includes+=("-I" "$inc")
		break
	fi
done

protoc "${includes[@]}" \
	--go_out=. --go_opt=paths=source_relative \
	--go-grpc_out=. --go-grpc_opt=paths=source_relative \
	internal/pluginapi/adapter.proto

banner "生成完成：git diff 确认幂等（无漂移）"
