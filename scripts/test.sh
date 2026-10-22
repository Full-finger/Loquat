#!/usr/bin/env bash
# 全部测试：go test + cargo test
# go 侧对 internal/core/ 额外跑一次 -race：core_race_test 为竞态检测器
# 量身写（清单 26102202#1），不开 -race 等于没测；全仓库开太慢只开这一个包。
set -euo pipefail
cd "$(dirname "$0")/.."
. scripts/common.sh
init_go
banner "go test"
"$GO" test ./...
banner "go test -race (internal/core)"
"$GO" test -race ./internal/core/
banner "cargo test"
cargo test
