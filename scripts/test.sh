#!/usr/bin/env bash
# 全部测试：go test + cargo test
set -euo pipefail
cd "$(dirname "$0")/.."
. scripts/common.sh
init_go
banner "go test"
"$GO" test ./...
banner "cargo test"
cargo test
