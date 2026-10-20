# loquat 构建入口
# 用法：just（列命令）、just build、just lint ...
# 分层：scripts/ 是可独立运行的原子脚本（工具链探测也在那边），
# 这里只负责项目配置、变量过桥和工作流编排。

# ── bash 解释器 ─────────────────────────────────────────────
# Windows 的 PATH 可能先撞上 WSL 的 bash 存根（system32\bash.exe），
# 它跑不了构建脚本，还会把路径里的反斜杠当转义符吃掉。
# 探测逻辑在 scripts/find-bash.sh；探测不到就回退 PATH 里的 bash。
bash_exe := `scripts/find-bash.sh`
bash := if bash_exe == "" { "bash" } else { bash_exe }

# 子进程（脚本里的 find/sort 等）也要先命中 git 的工具，
# 否则 system32 的 find.exe 会抢位，静默跳过检查。
# 只动 Windows 风格 PATH（pwsh 等）；git bash 的 POSIX 风格 PATH 不碰。
export PATH := if env_var("PATH") =~ ";" { parent_directory(bash) + ";" + env_var("PATH") } else { env_var("PATH") }

# ── 项目配置 ────────────────────────────────────────────────
app      := "loquat"
bin_dir  := "bin"
ld_flags := "-s -w"

# 传给 scripts/ 下的脚本；脚本内有同名兜底，脱离 just 也能跑。
export BIN_DIR := bin_dir
export LDFLAGS := ld_flags

# 显示本帮助（裸 just 等效）
help:
    @just --list

# 默认入口（不进清单）
[private]
default: help

# ── 构建与运行 ──────────────────────────────────────────────

# 构建产物；release 统一进 bin/。目标 all（默认）/ core / adapter / dll / rust
build mode="debug" target="all":
    @{{bash}} scripts/build.sh {{mode}} {{target}}

# 构建并启动宿主（mode: debug / release；对比内存占用用 release，
# 且先跑一次 just build release 让核心 dll 与适配器也换 release 版）
run mode="debug":
    @{{bash}} scripts/build.sh {{mode}} rust
    ./bin/{{app}}.exe

# ── 质量 ────────────────────────────────────────────────────

# 全部测试（go + cargo）
test:
    @{{bash}} scripts/test.sh

# 三语言静态检查 + 架构规则 + 清单一致性
lint: lint-go lint-rust lint-py arch (checklist "check")

# Go：vet（含 dll 构建标签）+ gofmt
lint-go:
    @{{bash}} scripts/lint-go.sh

# Rust：clippy
lint-rust:
    @echo "==> Rust clippy..."
    cargo clippy -- -D warnings

# Python：ruff
lint-py:
    @{{bash}} scripts/lint-py.sh

# 架构边界规则
arch:
    @{{bash}} scripts/check-arch.sh

# 未解决事项总览（docs/checkList/OPEN.md）：默认生成，传 check 只校验
checklist mode="digest":
    @python scripts/checklist.py {{mode}}

# 格式化（gofmt + cargo fmt）
fmt:
    @{{bash}} scripts/fmt.sh

# ── 组合工作流 ──────────────────────────────────────────────

# 提交前全套：静态检查 + 测试
ci: lint test

# ── 清理 ────────────────────────────────────────────────────

# 清理构建产物
clean:
    @rm -rf {{bin_dir}}
    @cargo clean
