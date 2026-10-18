# Loquat

用 Go、Rust 和 Python 酿造的 QQ 机器人：Go 核心管适配器与队列，Rust 宿主做入口与调度，Python 承载全部业务规则。

```
QQ ──▶ NapCat（WS，部署在 WSL）
         │ OneBot 11
         ▼
adapter-napcat（Go，go-plugin 子进程）   ← 崩溃隔离 / 热重载
         │ gRPC
         ▼
Go 核心（dll，链在 Rust 进程内）        ← 配置中心 / 消息队列 / 插件监督
         │ FFI
         ▼
Rust 宿主（入口进程）                  ← 业务调度 / 控制台命令
         │ pyo3
         ▼
extensions/<名>/plugin.py（Python 业务）    ← 规则全在这里，改完即生效
```

架构决策与实施记录见 [docs/mvp.md](docs/mvp.md)。

## 快速开始

前置：Go 1.24+、Rust、Python 3.10+、protoc（含 go 代码生成插件）、一个 NapCat（WS 服务端模式）。

```bash
just build                 # 构建四件套到 bin/
cp config/loquat.yaml.example config/loquat.yaml
# 编辑 config/loquat.yaml，填入 NapCat 地址与 access_token
just run                   # 或 ./target/debug/loquat.exe
```

QQ 里私聊机器人发 `ping`，回复 `pong` 即为成功。

## 控制台命令

| 命令 | 作用 |
|------|------|
| `status` | 核心与适配器状态、事件/回复计数 |
| `plugins` | 列出业务插件及启停状态 |
| `enable <插件>` / `disable <插件>` | 运行时启停业务插件（重启后回到文件内声明） |
| `reload napcat` | 热重载适配器（子进程级，自动恢复） |
| `help` / `exit` | 帮助 / 退出 |

## 开发

```bash
just lint        # 三语言静态检查 + 架构边界规则 + 清单一致性
just test        # go test + cargo test
just fmt         # 格式化
just checklist   # 生成未解决事项总览（docs/checkList/OPEN.md）
```

业务开发只碰 `extensions/<名>/`：`plugin.py` 写逻辑，`config.yaml` 写参数，
保存后**下一条消息自动生效**（惰性热重载），Go/Rust 零重编。
配置声明写在插件里的 SCHEMA：校验、默认值、缺失时的模板生成全由它派生，
配套库在 `extensions/_lib/config.py`（宿主注册为 `loquat_cfglib`）。
插件目录内可放多个 .py 互相 import（勿与标准库/已装包重名），改动同样热重载。
需要新的核心能力（如系统指标）时：Go 核心加导出 → `src/ffi.rs` 绑定 → `src/pybridge.rs` 注入桥，
业务侧通过全局 `core` 调用。

## 许可证

[Soda-Ware](LICENSE)（MIT 基础 + 请喝汽水条款）。
