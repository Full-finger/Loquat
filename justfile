# loquat 构建入口
# 用法：just（列命令）、just build、just lint ...

set shell := ["bash", "-cu"]

# ── 项目配置 ────────────────────────────────────────────────
app     := "loquat"
bin_dir := "./bin"
ld_flags := "-s -w"

# ── 编译器 ──────────────────────────────────────────────────
# 来源：compile.toml 的 go 字段（存在且非空）→ 用其路径；否则用 PATH 里的 go。
go_cfg := `sed -n 's/^[[:space:]]*go[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' compile.toml 2>/dev/null | head -n 1`
go := if go_cfg == "" { "go" } else { go_cfg }

# 环境变量 GOROOT 会覆盖 go 的自动推断，指向另一套编译器，
# 导致 "compile: version X does not match go tool version Y"。
# go 把空 GOROOT 当未设置，这里统一清掉。
export GOROOT := ""

# gofmt 跟随所选工具链，避免和 go 版本不一致。
exe := if os() == "windows" { ".exe" } else { "" }
godir := parent_directory(go)
gofmt := if godir == "." { "gofmt" } else { godir / ("gofmt" + exe) }

# ruff 探测：Windows 用 where，其余 command -v。只检查存在性，执行用 PATH 里的命令名，
# 避免 where 输出的反斜杠路径在 bash 里失效。缺失时 lint 显式报错，不静默跳过。
ruff := `if command -v where >/dev/null 2>&1; then where ruff >/dev/null 2>&1 && echo ruff; else command -v ruff; fi || true`

# ── 构建与运行 ──────────────────────────────────────────────

# 构建全部产物（Go exe/dll + 适配器 + Rust 宿主）
build:
    #!/usr/bin/env bash
    set -euo pipefail
    printf "\033[36m==> \033[32mBuilding all artifacts...\033[0m\n"
    {{go}} build -o {{bin_dir}}/core.exe ./cmd/core
    {{go}} build -o {{bin_dir}}/adapter-napcat.exe ./cmd/adapter-napcat
    {{go}} build -tags dll -buildmode=c-shared -ldflags="{{ld_flags}}" -o {{bin_dir}}/core.dll ./cmd/core
    cargo build

# 构建并启动宿主
run:
    #!/usr/bin/env bash
    set -euo pipefail
    printf "\033[36m==> \033[32mStarting {{app}}...\033[0m\n"
    cargo build
    ./target/debug/{{app}}.exe

# ── 质量 ────────────────────────────────────────────────────

# 全部测试（go test + cargo test）
test:
    #!/usr/bin/env bash
    set -euo pipefail
    printf "\033[36m==> \033[32m运行测试...\033[0m\n"
    {{go}} test ./...
    cargo test

# 三语言静态检查（vet/gofmt + clippy + ruff + 架构规则 + 清单一致性）
lint:
    #!/usr/bin/env bash
    set -euo pipefail
    printf "\033[36m==> \033[32mGo vet (incl. dll build tags)...\033[0m\n"
    {{go}} vet ./...
    {{go}} vet -tags dll ./...
    printf "\033[36m==> \033[32mgofmt check...\033[0m\n"
    files=$({{gofmt}} -s -l cmd internal)
    if [ -n "$files" ]; then
        printf '\033[31mFiles need formatting: \033[0m%s\n' "$files"
        exit 1
    fi
    printf "\033[36m==> \033[32mRust clippy...\033[0m\n"
    cargo clippy -- -D warnings
    printf "\033[36m==> \033[32mPython ruff...\033[0m\n"
    if [ -z "{{ruff}}" ]; then
        echo "\033[31mruff not installed: pip install ruff (or: winget install astral-sh.ruff)\033[0m"
        exit 1
    fi
    {{ruff}} check extensions/ scripts/
    printf "\033[36m==> \033[32mArchitecture rules...\033[0m\n"
    bash scripts/check-arch.sh
    printf "\033[36m==> \033[32mChecklist consistency...\033[0m\n"
    python scripts/checklist.py check
    printf "\033[32mAll checks passed\033[0m\n"

# 格式化代码（gofmt + cargo fmt）
fmt:
    #!/usr/bin/env bash
    set -euo pipefail
    printf "\033[36m==> \033[32mFormatting...\033[0m\n"
    {{gofmt}} -s -w cmd internal
    cargo fmt
    printf "\033[36m==> \033[32mDone\033[0m\n"

# 仅运行架构边界规则
arch:
    @bash scripts/check-arch.sh

# 生成未解决事项总览（docs/checkList/OPEN.md）
checklist:
    @python scripts/checklist.py digest

# ── 清理 ────────────────────────────────────────────────────

# 清理构建产物
clean:
    @rm -rf {{bin_dir}}
    @cargo clean
