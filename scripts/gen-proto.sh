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

protoc \
	--go_out=. --go_opt=paths=source_relative \
	--go-grpc_out=. --go-grpc_opt=paths=source_relative \
	internal/pluginapi/adapter.proto

banner "生成完成：git diff 确认幂等（无漂移）"
