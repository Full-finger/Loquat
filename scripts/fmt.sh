#!/usr/bin/env bash
# 格式化：gofmt（cmd internal）+ cargo fmt
set -euo pipefail
cd "$(dirname "$0")/.."
. scripts/common.sh
init_go
GOFMT="$(gofmt_of "$GO")"
banner "格式化"
"$GOFMT" -s -w cmd internal
cargo fmt
banner "完成"
