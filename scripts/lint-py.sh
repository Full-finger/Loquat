#!/usr/bin/env bash
# Python 静态检查：ruff（extensions/ scripts/）
set -euo pipefail
cd "$(dirname "$0")/.."
. scripts/common.sh
banner "Python ruff"
need ruff "pip install ruff（或 winget install astral-sh.ruff）"
ruff check extensions/ scripts/
