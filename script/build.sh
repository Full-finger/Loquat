#!/usr/bin/env bash
# 构建全部产物到 bin/：核心（exe 自测 + dll）、适配器插件、Rust 宿主
set -e
cd "$(dirname "$0")/.."
mkdir -p bin
go build -o bin/core.exe ./cmd/core
go build -o bin/adapter-napcat.exe ./cmd/adapter-napcat
go build -tags dll -buildmode=c-shared -ldflags="-s -w" -o bin/core.dll ./cmd/core
cargo build
cp target/debug/loquat.exe bin/
echo "构建完成：bin/core.exe bin/adapter-napcat.exe bin/core.dll bin/loquat.exe"
