# Agent 会话引擎设计（草案）

把 LLM 调用从"插件里的单轮请求"升级为内核持有的 agent loop（多轮 + 工具调用）。
参照 pi-agent 的分层：loop 是纯控制流内核，业务通过 tools 与事件参与。

参考实现：pi `packages/agent/src/agent-loop.ts`（runLoop ~80 行，两层循环 +
事件流 + AgentMessage/LLM Message 双轨 + Tool 抽象）。

## 目标

1. 一次会话可多轮：LLM 调工具 → 看结果 → 再调 → 最终回复（现状是单请求）
2. 会话状态归内核：历史不再随插件热重载丢失
3. 管线异步化：慢会话不阻塞其他聊天，不阻塞控制台（= 清单 26101801 #1）
4. 业务参与方式 = 声明 persona + tools，不碰 loop 控制流

## 非目标（后置）

- 流式输出（QQ 不需要 delta 推送）
- token 级 compaction（先用条数裁剪）
- 多 provider 抽象（`llm_api.py` 单 OpenAI 兼容后端够用）
- 运行中插话（steering）：agent 跑时的新消息进同 chat 队列，本轮结束后作为
  下一轮输入；不做运行中注入，行为不可控且 MVP 无需求

## 分层（对照 pi）

```
extensions/<名>/plugin.py     persona、触发规则、工具声明     ← pi coding-agent
loquat_agent（_lib/agent.py） loop + 工具执行 + 会话状态       ← pi agent
llm_api.py                    LLM HTTP（演进：带 tools 参数）  ← pi ai
Go 核心 / Rust 宿主           队列、适配器、发送通道（不动）
```

`loquat_agent` 放 `extensions/_lib/`，随宿主注册为 `loquat_agent`（同
`loquat_cfglib` 机制，`sharedlib.rs` 的 LIBS 清单加一行）。它是宿主注册的
内核配套库，不是插件：无签名、不参与责任链。不放 Rust 内嵌（include_str!）
的原因：放弃"改库零重编"的开发体验，不值。

## 内核侧：会话调度（Rust）

主循环只收事件和命令，处理挪到 worker：

```
main 循环（mpsc，不动）
  Event → parse_incoming → 按 chat_id hash 到 worker(N=4)
  Command → 原地处理（控制台响应恢复，#1 的另一半）

worker：跑责任链 business.handle(ctx) → 拿 replies → worker 内
dll.send_message()（统一发送：文本包成段列表，核心转适配器翻译成平台消息；
CoreDll 已是 Send+Sync，Go 侧导出并发安全）
```

- 同一 chat 永远落同一 worker：天然串行，`chat` 插件 `_histories` 等模块级
  状态不需要加锁
- 跨 chat 并行。并行度来自 LLM 的 HTTP 等待（socket 等待自动放 GIL），
  CPU 密集型无收益——记录预期，不追
- 每 worker 入队前检查积压：队列深度超阈值丢消息并记日志（与 Go 核心队列
  同语义）
- Ctrl+C / exit：`exit(0)` 时 worker 在途请求直接死，与现状等价，不处理

## Python 内置库 loquat_agent

### 状态锚点

`_lib` 热重载 = 整模块替换，模块属性存状态会丢（`_histories` 的老问题）。
状态放 Rust 侧持久 PyDict（`pybridge.rs` 创建一次，`OnceLock` 持有），
key 为 `(message_type, chat_id)`，value 为 messages list。
`loquat_agent` 每次重载后从锚点取，状态跨重载存活。

### 接口

```python
# extensions/_lib/agent.py

def respond(ctx: dict, *, system_prompt: str, tools=(),
            max_turns: int = 6) -> list[str]:
    """跑一次 agent 会话，返回最终回复文本（0 或多条）。
    ctx 即插件 handle 收到的 dict；会话 key 默认 (message_type, chat_id)。
    loop：调 LLM → 无 tool_calls 则收尾；有则逐个执行、结果回填、继续。
    超过 max_turns 强制收尾，日志告警。单请求超时由 llm_api 的
    timeout_seconds 管，循环失控由 max_turns 管。"""

def tool(fn):
    """装饰器：从函数签名与 docstring 推导 OpenAI tools schema。
    支持 str/int/float/bool/list/dict 注解；docstring 首段作 description。
    工具不做全局注册——插件每次 respond 传入自己的 tools，
    热重载后旧引用自然消失，无需清理逻辑。"""
```

### 工具桥（内核工具）

`pybridge` 注入到 `loquat_agent` 可用范围：

| 工具 | 实现 | 说明 |
|------|------|------|
| `send_message(message_type, chat_id, text)` | `dll.send_action` | 主动发独立消息（发图 CQ 码等场景）；与最终回复并存，顺序天然正确 |
| `system_status()` | 现有 core 桥 | 已有 |
| `now()` | 标准库 | 时间常识兜底 |

## chat 插件改造后

```python
# loquat-plugin
PRIORITY = 200
import loquat_agent
from loquat_agent import tool

@tool
def send_sticker(...) -> str: ...

def handle(ctx) -> list[str]:
    # 触发判断保留现状；历史管理从本模块删除，归 loquat_agent
    return loquat_agent.respond(ctx, system_prompt=cfg["system_prompt"],
                                tools=[send_sticker])
```

责任链语义、热重载、install/enable 全部不动；历史从 `_histories` 挪入
状态锚点后，重启前丢失改为持久于进程生命周期。

## llm_api 演进

```python
class Resp: text: str; tool_calls: list[dict]   # name/arguments/id

def chat(cfg, messages, tools=None) -> Resp | None
```

非流式，OpenAI `/chat/completions` 的 `tools` / `tool_calls` 字段。

## 实施步骤（PR 粒度）

1. **worker 池异步化**：main.rs 拆路由 + worker，发送挪 worker。关清单
   26101801 #1，改清单并勾掉
   - 验证：人为 sleep 5s 的消息期间，另一 chat 消息正常回、`status` 即时返回
2. **loquat_agent 单轮**：状态锚点 + `respond`（无工具）+ `llm_api` 返回
   `Resp` + chat 插件迁移 + 删 `_histories`
   - 验证：改 chat 插件触发热重载，历史不丢；ping-pong 语义不变
3. **工具循环**：tools 解析 + dispatch（异常转 error result 回填模型）+
   内核工具桥
   - 验证：让模型调 `now()` 回答时间；调 `send_message` 发图
4. **（可选）`agents` 控制台命令**：worker 队列深度、活跃会话、轮次计数

## 已知取舍

- hash worker：单个 worker 内多 chat 排队，极端分布不均时个别会话等待——
  QQ 机器人规模下可接受，真实流量证明不够再拆
- GIL：多 worker 同时跑 Python 计算无并行收益；等待型（LLM/HTTP）无损失
- Python 崩溃影响宿主：维持清单 26101801"暂缓"项的既有决策，不变
- cargo test 并行：新增多线程 Python 调用，沿用 `RUST_TEST_THREADS=1` 纪律
