#!/usr/bin/env bash
# Go 静态检查：vet（含 dll 构建标签）+ gofmt
set -euo pipefail
cd "$(dirname "$0")/.."
. scripts/common.sh
init_go
GOFMT="$(gofmt_of "$GO")"

banner "Go vet（含 dll 构建标签）"
"$GO" vet ./...
"$GO" vet -tags dll ./...

banner "gofmt check"
files="$("$GOFMT" -s -l cmd internal)"
if [ -n "$files" ]; then
	die "需要格式化：\n$files\n运行 just fmt 修复"
fi
