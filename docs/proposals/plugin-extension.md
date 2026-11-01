# 提案：插件扩展面（替代"自动编织"设想）

状态：部分实施——§4.1 数据契约（str | 段列表）与 §6 版本门已落地（评审建议 5）；§3 三种链语义、§4.2 钩子、§4.4 能力、§4.5 事件未实施。前序评审见 `docs/pipeline-review.md`。

## 1. 背景

曾考虑过一条更"自由"的路线：对插件函数做副作用分析，推断出流程链，再用一张可被其他插件申请的转义表，
把 `X.A → X.B` 无感改写成 `X.A → Y.C → X.B`。

这条路被否决，理由不是性能，而是切点由推断得来就不再是契约：X 重构即静默脱钩，
且 Python 的副作用面不可判定，近似意味着插入点**有时**落错位置——比不插入更危险。
更根本的一点：宏之所以强大靠卫生性（展开点可见、局部、名字不捕获），而跨插件注入天然不卫生，
要么看不见（无法推理），要么看得见（Y 绑死 X 私有实现）。

但那个动机是成立的：**想扩展而不想 fork**。本提案用一套显式、可枚举、可版本化的窄接口承接它。

## 2. 一条原则

**扩展只发生在边界，不发生在单元内部。**

论证是可枚举性：边界点有固定若干个，能一眼列完；单元内部点是 N×M，随插件数与函数数增长。
系统里每一处"行为可以不是本地的"地方，调试成本都会上升——所以必须能把这类地方**数清楚**。

推论：**每个钩子都是长期契约，钩子数量是第一设计参数。** 目标 ≤5 个，加一个都要论证。

另一条推论，也是本提案对原设想的诚实降级：**透明是不可能的，只能让协作变便宜。**
X 不配合就无法被扩展（它得在合适的位置发布事件或调用 `_lib`）。接受这一点，换来的是系统可推理。

## 3. 三种链语义

现有责任链只有一种语义（首个非空接管），这是它表达力不足的根因。补成三种，各自语义明确，
**不做 onion（`next()`）**：onion 允许中间件不调用 `next()` 或调用两次，顺序推理成本高，
而下面三种已覆盖全部已知需求。

| 语义 | 钩子 | 执行 | 用途 |
|---|---|---|---|
| 路由 | `on_message` | 按优先级，首个非 `None` 独占 | 命令、问答、LLM 兜底 |
| 观察 | `on_observe` | 全部执行，返回值忽略 | 历史记录、审计、埋点、统计 |
| 变换 | `before_send` | 按优先级逐级 fold，`None` = 不改 | 敏感词、长度切分、平台适配 |

关键收益：横切关注点从路由链里解放出来。现在 `chat` 插件的 `observe()` 被 `qa`/`status` 抢答就丢历史，
换成 `on_observe` 后无条件执行。

## 4. 扩展面

### 4.1 数据契约：收发同构

出站从 `list[str]` 升级为段列表，与入站共用同一份 schema（`adapter.proto` 的 `Segment`）：

```python
return "纯文本"                                        # 语法糖，等价于一个 text 段
return [{"text": {"text": "看这个"}},
        {"image": {"url": "https://..."}},
        {"reply": {"messageId": ctx["message_id"]}}]   # 扁平列表，不嵌套
```

配一个 `_lib` 里的极薄构造器，避免手写 dict：

```python
from loquat import seg
return [seg.text("看这个"), seg.image(url), seg.reply(ctx["message_id"])]
```

约束：**扁平列表，不支持嵌套树**。嵌套是平台适配器该操心的事，不是框架的事。

### 4.2 钩子清单（5 个）

```python
API = 2                                    # 插件 API 主版本，与首行签名一致

def on_message(ctx) -> str | list | None   # 路由：首个非 None 接管
def on_observe(ctx) -> None                # 观察：全部执行
def before_send(ctx, segments) -> list | None   # 变换：逐级 fold
def on_event(topic, payload) -> None       # 插件间广播，topic 为具名字符串
def on_unload() -> None                    # 热重载/停用前清理（线程、句柄、缓存）
```

`on_unload` 在热重载下是刚需：现在插件起的线程/定时器在模块被换掉后无人回收。

延后（先不加）：`on_timer`（`SCHEDULE` 声明 + 定时触发）、`on_tick`。等真有第二个用得上它的插件再说。

### 4.3 ctx 契约

保持 **dict**，不加方法、不加对象。理由：dict 是 AI 最容易生成、宿主最容易构造的形状，
一旦变成带方法的上下文对象，就滑向"框架"。

新增一个字段：

| 字段 | 含义 |
|---|---|
| `deadline_ms` | 本条消息的剩余预算（毫秒）。LLM 类插件据此收敛 `max_turns`，超预算先返回部分结果 |

其余字段维持现状（`text` / `message_type` / `chat_id` / `user_id` / `self_id` / `message_id` /
`is_at_bot` / `images` / `segments` / `platform`）。字段变更必须升 `API` 主版本。

### 4.4 能力（capability）

`core.send_action` 保留为逃生舱，但**发送类动作**（`send_private_msg` / `send_group_msg`）
改为内部走统一出站队列——否则逃生舱永远绕过排序、统计与限流。

新增三个显式能力，覆盖 90% 的主动发送，避免插件拼 OneBot JSON：

```python
core.send(chat_id, segments, message_type="group")   # 进队列，返回本地消息 ID
core.reply(ctx, segments)                            # 回复触发消息
core.recall(message_id)                              # 撤回
```

### 4.5 插件间通信

用**具名事件 + `_lib`**，不做函数注册表：

```python
# X 在合适的位置发布（X 需要配合，这是接受的前提）
core.publish("chat.replied", {"chat_id": ..., "text": ...})

# Y 订阅，不需要 import X
def on_event(topic, payload):
    if topic == "chat.replied": ...
```

topic 是契约：命名空间化（`<插件>.<动作>`），一旦发布就要当公开接口维护。
跨插件复用逻辑走 `_lib`（已存在，是共享**词汇**），不走**机制**。

## 5. 代码形状

插件从"实现 handle"变成"声明自己关心哪个边界"：

```python
# loquat-plugin v2
API = 2
PRIORITY = 200
SCHEMA = [...]

def on_observe(ctx):
    # 无条件记历史，不再被抢答截胡
    loquat_agent.observe(ctx, loquat_agent.clean_text(ctx), cfg())

def on_message(ctx):
    if not _triggered(ctx): return None
    return [seg.text(reply)]
```

## 6. 版本与迁移

- 签名首行带版本：`# loquat-plugin v2`，与 `API = 2` 不一致时启动即拒载。
- 无 `API` 声明视为 v1；宿主提供兼容垫片，把旧 `handle` 包成 `on_message`，并打一次告警。
  垫片约 30 行，让存量插件平滑过渡。
- 主版本不匹配 = 硬失败 + 明确日志（"插件 X 需要 API 3，宿主为 2"）。禁止静默降级——
  AI 生成插件的场景下，静默失败最难排查。

## 7. 不做什么

| 不做 | 原因 |
|---|---|
| 自动推断切点 / 副作用分析驱动派发 | 见 §1，推断出的切点不是契约 |
| 允许插件改写他人函数或调用链 | 行为不可枚举，调试需重建 weave |
| onion 中间件（`next()`） | 顺序推理成本高；三种链语义已覆盖 |
| DI 容器 / 服务注册表 | 5 个插件的项目里，容器是纯开销 |
| 嵌套消息树 / 通用元素 DSL | 扁平段列表够用，嵌套归适配器 |
| 插件间直接函数注册与调用 | 演化成隐式全局耦合；用事件与 `_lib` |
| 每个插件一个进程 | 只对 LLM 类重插件做隔离（见评审建议 4） |

## 8. 验收

1. 发图片 / 引用 / at 不再需要 `send_action`，且走统一出站队列（顺序、统计、限流生效）。
2. 加一个黑名单插件挡住消息后，`on_observe` 仍拿到该消息（验证路由与观察分离）。
3. 关掉全部 `on_observe` 插件，路由结果不变（验证观察不参与决策）。
4. 插件写 `API = 3` 时启动即报错，日志能定位到插件名与版本。
5. `just chain` 打印当前生效的钩子表（谁注册了哪个钩子、什么优先级）——
   让组合**可见**，这是对原设想唯一值得保留的部分：不要透明，要可枚举。
