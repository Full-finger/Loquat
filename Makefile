# ── 项目配置 ────────────────────────────────────────────────
APP     := loquat
BIN_DIR := ./bin
CONFIG  := ./config/loquat.yaml

# Go
GOFMT   := gofmt -s -l
GOVET   := go vet
# Rust / Python
CARGO   := cargo
# ruff 探测：跨平台（Windows 用 where，其余 command -v）。
# 缺失时 lint 的 ruff 步骤会显式报错，不静默跳过。
ifeq ($(OS),Windows_NT)
RUFF_DETECTED := $(shell where ruff 2>/dev/null)
else
RUFF_DETECTED := $(shell command -v ruff 2>/dev/null)
endif
RUFF := $(firstword $(RUFF_DETECTED))

# ── 颜色输出 ────────────────────────────────────────────────
CYAN  := \033[36m
GREEN := \033[32m
YELLOW:= \033[33m
RED   := \033[31m
RESET := \033[0m

define log
	@printf "$(CYAN)==> $(GREEN)$(1)$(RESET)\n"
endef

# ── Phony 声明 ─────────────────────────────────────────────
.PHONY: help build run test lint fmt arch clean

# ── 默认目标 ────────────────────────────────────────────────
help: ## 显示帮助信息
	@printf "$(CYAN)$(APP)$(RESET)  可用命令：\n\n"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  $(YELLOW)%-8s$(RESET) %s\n", $$1, $$2}'

# ── 构建与运行 ──────────────────────────────────────────────
build: ## 构建全部产物（Go exe/dll + 适配器 + Rust 宿主）
	$(call log,构建全部产物...)
	bash scripts/build.sh

run: ## 启动宿主（先构建）
	$(call log,启动 $(APP)...)
	bash scripts/build.sh
	./bin/loquat.exe

# ── 质量 ────────────────────────────────────────────────────
test: ## 全部测试（go test + cargo test）
	$(call log,运行测试...)
	go test ./...
	cargo test

lint: ## 三语言静态检查（vet/gofmt + clippy + ruff + 架构规则）
	$(call log,Go vet（含 dll 构建标签）...)
	@$(GOVET) ./...
	@$(GOVET) -tags dll ./...
	$(call log,gofmt 检查...)
	@files=$$($(GOFMT) cmd internal); \
	if [ -n "$$files" ]; then \
		printf '%b\n' "$(RED)以下文件需要格式化：$(RESET) $$files"; \
		exit 1; \
	fi
	$(call log,Rust clippy...)
	@$(CARGO) clippy -- -D warnings
	$(call log,Python ruff...)
	@if [ -z "$(RUFF)" ]; then \
		echo "$(RED)ruff 未安装：pip install ruff 或 winget install astral-sh.ruff$(RESET)"; \
		exit 1; \
	fi
	@$(RUFF) check extensions/
	$(call log,架构规则...)
	@bash scripts/check-arch.sh
	$(call log,全部通过)

fmt: ## 格式化代码（gofmt + cargo fmt）
	$(call log,格式化...)
	gofmt -s -w cmd internal
	cargo fmt
	$(call log,完成)

arch: ## 仅运行架构边界规则
	$(call log,运行架构规则...)
	bash scripts/check-arch.sh

# ── 清理 ────────────────────────────────────────────────────
clean: ## 清理构建产物
	$(call log,清理...)
	rm -rf $(BIN_DIR)
	cargo clean
