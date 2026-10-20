# MVP 方案：问答机器人

## 目标

QQ 里发消息，机器人按 Python 侧定义的规则回复。
业务规则全部放在 Python，改业务不重编 Go 和 Rust。
适配器以 go-plugin 子进程运行，获得崩溃隔离和热重载能力。

设计原则：隔离单位按"变更频率和信任度"划，不按语言边界划。
核心是自己的稳定代码，进程内直链；插件是热换的代码，子进程隔离。

## 总体架构

```
QQ 客户端
   ↕
NapCat（WS 服务端，跑在 WSL）
   ↕ OneBot 11 / 正向 WebSocket
adapter-napcat（Go，go-plugin 插件，独立子进程）
   ↕ gRPC（hashicorp/go-plugin）
Go 核心（dll，链在 Rust 进程内：配置 / 会话缓存 / 消息队列 / 插件监督）
   ↕ 进程内 FFI（c-shared）
Rust 宿主（入口，业务调度，pyo3 嵌入 Python）
   ↕ pyo3
qa.py（Python 业务）
```

| 组件 | 语言 | 形态 | 职责 |
|------|------|------|------|
| loquat 宿主 | Rust | 入口进程 | 加载核心 dll、调 Python 业务、控制台命令 |
| Go 核心 | Go | c-shared dll，进程内 | 配置中心、会话缓存、消息队列、go-plugin 宿主（插件监督与重启） |
| adapter-napcat | Go | 插件子进程 | NapCat WS 连接、OneBot 11 编解码、事件上报、动作下发 |
| qa.py | Python | 嵌入解释器 | 问答规则。输入一条消息文本，输出若干条回复 |

崩溃与恢复的边界：

- 适配器崩溃或卡死：只死子进程，Go 核心退避重启它，宿主无感
- Go 核心崩溃：整个宿主进程陪葬（进程内直链的代价）。核心是稳定代码，可接受；
  宿主崩了就整个重启
- qa.py 抛异常：pyo3 捕获后按错误消息处理，不影响宿主

## 通信链路

### NapCat ↔ 适配器（OneBot 11，正向 WebSocket）

- NapCat 是服务端：`ws://127.0.0.1:3001`，鉴权走 `Authorization: Bearer` 头（token 不落 URL）
- 适配器是客户端：收事件、发动作
- MVP 只处理 message 事件，忽略 meta / notice / request
- 动作只实现 `send_private_msg` / `send_group_msg`
- 动作调用带 `echo` 字段，用于匹配响应

### 适配器 ↔ Go 核心（go-plugin，gRPC）

- 插件接口的 proto 定义在 `internal/pluginapi`
- 事件方向：适配器流式上报
- 动作方向：核心调用适配器的 SendMsg
- 核心读到的配置经插件接口在启动时下发给适配器

### Go 核心 ↔ Rust 宿主（进程内 FFI）

- Go 核心编译为 c-shared dll，Rust 启动时加载
- Rust 把事件回调的函数指针（C ABI）注册给核心；事件以 JSON 字节上行
- 动作经核心导出的函数下行（发消息等），同样 JSON 字节
- 无 RPC、无序列化框架，就一层函数调用

### Rust ↔ Python（pyo3 嵌入）

- qa.py 源码喂给解释器执行，宿主调用其中的 `handle(text) -> list[str]`
- **多插件责任链**：extensions/<名>/plugin.py 为入口（首行 `# loquat-plugin` 签名，
  无签名目录忽略），按 `PRIORITY`（缺省 100，越小越先）
  + 目录名排序逐个调用，第一个返回非空的接管；返回空则链继续。丢目录/删文件/改文件
  均在下一条消息前惰性同步，单插件加载失败只跳过自身
- **插件安装**：声明了 SCHEMA 且 config.yaml 缺失 = 未安装，不加载，
  控制台提示 `install <插件名>`；install 按 SCHEMA 生成 config.yaml 并写 .disabled 保持未启用，
  `enable <插件名>` 激活。config.yaml 是运行时状态，不入库
- **插件启停**：两层——插件 config.yaml 里 `enabled: false`（永久）+ 控制台
  `disable/enable <插件名>`（写 .disabled 标记，持久）；被禁用的插件仍跟踪热重载，
  重新启用无缝回归
- **能力桥**：Go 核心的能力（如 `LoquatSystemStatus`）包装成 Python 函数，
  以 `core` 名字注入每个插件——业务用 `core.system_status()` 取数据，自身保持纯 Python；
  热重载后桥自动重新挂载
- 热重载采用惰性策略：每次调用前比对脚本 mtime，变了才重新加载模块；
  重载失败沿用旧版本继续服务。不引入文件监视线程，对问答场景即改即生效

## 目录结构

```
Cargo.toml、src/         Rust 宿主（loquat）：src/ffi.rs 能力桥与 FFI、src/business.rs 业务加载
go.mod                   Go 侧（github.com/full-finger/loquat）
cmd/core/                Go 核心（可编 exe 自测，可编 dll 嵌入）
cmd/adapter-napcat/      NapCat 适配器
internal/core/           核心逻辑：配置、消息队列、插件监督、系统状态采集
internal/pluginapi/      go-plugin 插件接口与握手（含 proto）
extensions/              Python 业务插件（每目录自包含：plugin.py；config.yaml 运行时生成，不入库）
scripts/                 构建、架构检查脚本
justfile                 质量入口：just lint / test / fmt / build
config/loquat.yaml       本机配置（模板 loquat.yaml.example 入库）
docs/                    文档
demo/                    FFI 演示（本地保留，不入库）
```

module：`github.com/full-finger/loquat`

## 实施步骤

### 第 1 步：Go 侧跑通（暂不经过 Rust）

- 做：go.mod、pluginapi proto、Go 核心（读配置、拉插件、事件入队、回固定文本）、
  适配器（连 WS、鉴权、上报事件、实现 SendMsg）。核心先编成 exe 自测
- 验证：QQ 私聊机器人发任意文本，收到固定回复；核心日志打印原始事件 JSON

### 第 2 步：核心嵌入 Rust（本方案的技术风险验证点）

- 做：核心加 c-shared 导出（初始化、注册事件回调、发动作、关闭）；
  Rust 宿主加载 dll、注册回调、收到事件回一条固定文本
- 验证：回复内容出自 Rust 侧；重点确认 go-plugin 宿主在 dll 内
  能正常拉起适配器子进程并完成握手；杀适配器进程，核心自动重启它

### 第 3 步：接入 Python 业务

- 做：pyo3 嵌入 qa.py；事件文本转给业务，回复沿原路下发
- 验证：QQ 发"ping"，收到"pong"，内容来自 qa.py

### 第 4 步：热重载与自动恢复

- 做：
  1. qa.py 惰性热重载：每次消息处理前比对 mtime，变了重新加载模块
  2. 适配器崩溃退避重启：1s 起步、每次翻倍、上限 30s
  3. 宿主控制台命令：`status`（查看插件与业务状态）、`reload napcat`（重载适配器）
- 验证：
  1. 杀掉适配器进程，自动恢复，恢复后消息继续通
  2. 编辑 qa.py 保存，不做任何重启，新规则生效
  3. 控制台输 `reload napcat`，适配器进程更换，连接自动恢复

## MVP 验收标准

1. 私聊发 `ping` 回 `pong`，规则在 Python 侧
2. 群聊只响应 `/` 开头的消息（如 `/ping`），回复发到群里
3. 杀适配器进程，30 秒内自动恢复
4. 改 qa.py 即时生效，Go 和 Rust 零重编

## 群聊约定

MVP 阶段群聊只处理以 `/` 开头的消息。
判断消息是否 @机器人 需要解析消息段（CQ 码或数组格式），往后放。

## 配置

```yaml
# config/loquat.yaml
napcat:
  url: ws://127.0.0.1:3001
  token: <access_token>          # 真实值写在本机配置里，不进仓库
plugins:
  napcat:
    path: bin/adapter-napcat.exe # Go 核心拉起的适配器二进制
business:
  script: extensions/qa.py
```

## 代价与已知取舍

- 核心直链进程内：少一跳 RPC、少一个进程；代价是核心崩 = 宿主崩，
  且改核心要重启整个宿主。核心代码稳定，可接受
- go-plugin 宿主跑在 dll 内的 Go 运行时里，这个组合没有现成背书，
  第 2 步专门验证；失败则退回"核心独立进程 + gRPC"的方案，接口基本不变
- 一条消息的路径：适配器 → 核心（1 次 gRPC）→ 宿主（FFI）→ Python（进程内）。
  跨进程只剩一次，本机毫秒级内
- 消息队列用进程内有界队列：保顺序、给背压，满了丢消息并记日志（聊天场景可丢）

## 注意事项

- **Windows 上 Go c-shared 的链接器坑**：产物体积大时（实测 27MB），链接器会把
  DWARF 调试节区的 RVA 写成越界值（0xdfa40000），LoadLibrary 报错 193
  “不是有效的 Win32 应用程序”。dll 构建统一加 `-ldflags="-s -w"`（去调试信息后
  正常，13MB）；需要符号调试时用 exe 形态，exe 不受影响
- WSL2 网络的 127.0.0.1：Windows 侧默认能直连 WSL 内监听的端口。
  连不上先确认 NapCat 监听的是 0.0.0.0 还是仅 127.0.0.1，再查 WSL 端口转发
- OneBot 11 的字段以 NapCat 实际输出为准，第 1 步联调时先原样打印事件
- 适配器重载会闪断 NapCat 连接，由适配器自动重连；重载瞬间在途消息会丢，可接受
- access_token 是敏感信息：文档和配置示例用占位符，真实值放本机 config，
  `config/` 目录在动工时加进 .gitignore
