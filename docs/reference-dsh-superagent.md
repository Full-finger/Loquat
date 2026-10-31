# 参考分析：dsh-superagent（DeepSeek 研究原型）

状态：外部参考，**不是本仓依赖**，也不建议引入。
分析对象：`../dsh-superagent-prototype`（克隆于 2026-11-01，未记录 commit）。
用途：为 `docs/pipeline-review.md`、`docs/proposals/plugin-extension.md`、`docs/proposals/message-batching.md`
三份文档提供外部对照，校准其中若干结论。

README 自述要点（引用时请保留）：「Research prototype for reference only, not an official DeepSeek product…
much of the engineering done by coding agents… research code, not production-ready software.」
另有说明：文中讨论的部分设计属于未来方向，原型未实现。

## 1. 它是什么

给**单个 GitHub 仓库**当常驻协作者的 agent：监听 GitHub（可选飞书）→ 判断来者是否有权派活 →
开 Ticket → 每个 Ticket 一个独立 Harness Session 与独立工作目录 → 干完在原会话回话、开 PR →
Web 面板只列「需要人决定的事」。

| 项 | 值 |
|---|---|
| 语言 / 运行时 | TypeScript，Node 22.19+ / 24+ |
| 形态 | DeepSeek Harness 的 bundle，经 `cordis.patch.yml` 组合（Cordis 即 koishi 的 DI 框架） |
| 依赖 | 全部为 `@deepseek-ai/*` alpha 包 + `zod` + `proper-lockfile`；存储用内置 `node:sqlite` |
| 规模 | `src/` 约 75 个 ts 文件；`tests/` 60 个 spec；`docs/` 9 篇共约 1.97k 行；`prompts/` 11 篇 |
| 许可证 | MIT |

**它不是一个通用 agent 框架**，是一个垂直领域的完整应用。这一点决定了后面所有取舍：
它敢做重模型（Ticket/Mandate/Assignment），因为它的工作是「授权一项跨天的、可能对外发 PR 的活」。
聊天机器人照搬这套模型即为过度建模，见 §5。

## 2. 设计骨架

```
Source ──Trigger──> Thread ──batch──> Eval(gate → assessment) ──accept──> Ticket
                                                                          │
                                                          Assignment ─────┘
                                                               │
                                                          Executor(Session + dir)
                                                               │
Channel <──────────────── Source <──────────────────────── Report
```

核心概念（`docs/001-domain.md`，均有「它守什么规则」一列）：

| 概念 | 一句话 |
|---|---|
| Source | 事件流；持有**读位点**与自己的 bot 身份；「重启不得重放事件」 |
| Trigger | 一个不可变事件；**「同一个 Trigger 被接受两次只产生一个 Ticket」** |
| Thread | 时间有序的 Trigger 流；**同一时刻只跑一个 Eval**，其间到达的合并进下一批 |
| Eval | 两层：`gate`（确定性：谁有权发起）+ `assessment`（模型研判）；无状态 |
| Mandate | 授权内容 + Policy 版本；不可变；恢复时按当前 Policy 重新校验 |
| Ticket | 授权的单位；只有 Eval 能创建；自己的状态机，不跟随 Executor |
| Assignment | Ticket ↔ Executor 实例的绑定；可多次 |
| Subscription | Ticket 与 Thread 的多对多；扇出**只加上下文，不改路由** |
| Policy | 版本化的规则集；变更不撤销已发 Mandate，只投递给运行中的 Assignment |

两个值得单独记住的设计约束：

1. **「一个人不是领域概念」**。Trigger 只携带 actor id，允许名单是 Source 的一部分；
   一个人跨 Source 的账号对应关系，是**提示词文本**，不是数据模型。
2. **接受模式默认 `observe`**：inferred 的活照常研判，但只记录「本来会接」，不真接。
   主动型 agent 的失败方式从来不是「干不了」，是「太吵」，所以默认干跑。

`001-domain.md` 末尾有 13 行 **Rejected concepts** 表，逐条列出**拒绝建模**的东西及理由
（不做 Principal/Membership、不做跨 Source 身份映射、不做「等待中/已撤销」的 Eval 状态、
不用自由文本推断「在等人回复」……）。配套规则只有一句：

> 一个业务需求要么成为已有概念的属性，要么成为 agent 遵循的提示词。设计不为它新增概念。

`src/limits.ts` 全文 11 行，是这条规则的执行结果。

## 3. 六个值得学的机制

### 3.1 事务性 outbox（`src/app/effects.ts`，161 行）

副作用在**领域决策的同一个 SQLite 事务**内入队，提交后才派发。文档的原话是
「What dsh-superagent does after a decision … is stored first and done afterwards」。

表结构：`effect(kind, key, seq, generation, attempts, due, since, error)`。关键细节：

- `ON CONFLICT(kind, key)` 做**合并**，`generation + 1`；执行结束时按 `seq + generation` 匹配再删除，
  保证「正在跑的尝试不会删掉一个更新的 payload」。这一条是手写队列最容易漏的。
- `ordered` 类型的行**即使未到 due 也堵在最老一行后面**，顺序不被退避打乱。
- 失败退避 `min(max, base × 2^min(attempts,16))`；错误文本截 1000 字符。
- 关闭时**不删运行中的行**，下次启动自动重试。
- 健康状态按 kind 聚合：`N waiting since T; the last failure: …`，且**连续失败 3 次才升级为 error**。

### 3.2 「容量不足」与「失败」分开建模

`effects.ts` 里有一个 `class Later extends Error`：处理器在「处理暂停」或「worker 槽位满」时抛它，
调度器只把 `due` 推后一个初始间隔，**不递增失败计数**。

这是很干净的区分：背压不是错误。Loquat 现在把两者混在一起——队列满就丢、失败就丢，
统计口径也就分不出来。这一条成本极低，收益很大。

### 3.3 顺序 + 有界重试 + 死信（`src/app/outbox.ts`，52 行）

- 同一 Ticket 发往同一会话的消息，**必须等前一条落定**；前一条连续失败 3 次后放行。
- 目标确实不存在（Issue 被删、消息被撤回）→ settle 成 `dropped` 并**留记录，永不重投**。
- 回复重试退避 `[5000, 300000]`。

顺序、有界重试、死信三件事在一个 52 行的类里说清楚，是这条链路里最值得抄的部分。

### 3.4 幂等做在领域层，不做在消息层

- 幂等键是 **Trigger 的身份**，不是 message id 的 LRU。
- Source 持有读位点，「重启不得重放」。
- 「接受两次只产生一个 Ticket」是领域不变量，不是去重缓存。

Loquat 现状是完全无去重。小版本是 `(platform, message_id)` + TTL；普遍形式是前者。

### 3.5 gate 在模型之前

`Eval` 分两层：`gate` 是确定性的（谁有权、强度如何），过了才交给模型 `assessment`。
准入表只有三格（在名单/不在名单 × 是否直接呼叫），默认值写死在文档里。

配套的是 `acceptMode: off | observe | auto`，默认 `observe`。
**先让它在不影响任何人的情况下跑一周，看它本来会干什么**——这是主动型 agent 唯一靠谱的上线方式。

### 3.6 批处理 = 排队合并，不是时间窗合并

`src/core/evals.ts`：「batches a Thread's queued Triggers into one Eval」。
机制是：一个 Thread 同时只跑一个 Eval，其间到达的 Trigger 排队，合并进**下一批**。

所以它解决的是「忙的时候不重复研判」，**不解决刷屏合并**——
Thread 空闲时连来 5 条，就是 5 次 Eval。对 agent 够用（agent 本来就慢），对聊天机器人不够。

但它指出了一个更优解：**批的边界可以就是串行执行器的边界**，于是不需要定时器、
不需要 `max_wait`、不会有 debounce 饿死、也不会有旁路导致的顺序反转。
Loquat 需要低延迟，所以结论是「边界优先，定时器兜底」（见 `message-batching.md` 的修正）。

## 4. 与前三份文档对账

| 我们的结论 | 它的做法 | 判定 |
|---|---|---|
| 每会话串行执行器（评审建议 7） | Thread 同时只跑一个 Eval | **验证**，且证明串行是批处理的前置 |
| 出站队列 + 有限重试 + 限流（建议 2） | 事务性 outbox + 顺序 + 死信 | **验证并加强**：我提的是内存队列，它是事务性持久化 |
| 消息级去重（建议 9） | 事件级幂等 + 读位点 | **应升级**：去重键提到事件身份 |
| 失败插件退避（P0-2） | 连续 3 次才升 error，其余只记 attempts | **应加强**：健康状态而非日志洪水 |
| 批处理要定时器 + 两个上界 | 边界即串行边界，无定时器 | **部分修正**：边界优先，定时器兜底 |
| 三种链语义（扩展面提案） | 无对应（单一 assessor，无多插件竞争） | 不适用：它的「扩展」是 Source/Channel，不是消息处理链 |
| 过度设计清单 | Rejected concepts 表 | **同构**，可互为印证：把「拒绝什么」写成正式产物 |

## 5. 不要照搬的四件事

1. **领域模型**。Ticket / Mandate / Assignment / Subscription 成立的前提是「授权一项跨天的、
   可能对外发 PR 的工作」。Loquat 做的是「回一条消息」。照搬即过度建模——
   和「自动编织」是同一个错误换了件更体面的衣服，而且更危险，因为它看起来像架构水平。
2. **持久化 outbox**。单机聊天场景，丢一条回复的代价是「没回」，不是「承诺没兑现」。
   内存有界队列 + 可见丢弃 + 降级回复足够；等哪天有对外承诺型动作（定时提醒、定时发帖）再上持久化。
3. **一 Ticket 一 Session 的成本结构**。它敢这么做是因为有 gate 拦在前面，绝大多数事件进不到这一步。
   Loquat 如果把每条消息都当一次 Session，账单会先崩。
4. **prompt-as-configuration**。它把「设置」推进提示词（「一个默认值能覆盖几乎所有团队时，就不要设置项」），
   代价是 `docs/004-prompts.md` 里长出了一整套提示词覆盖体系：`base` 记录、升级时标 **Default updated**、
   diff 展示、按 part 覆盖。**复杂度没有消失，只是从「配置项太多」搬成了「提示词版本漂移」。**
   而且提示词改的是行为，非确定、难测试，所以它必须留一个确定性 gate 兜底。

## 6. 可直接落地的四条

| # | 做什么 | 参考 | 验收 |
|---|---|---|---|
| 1 | 出站改 outbox，表结构抄 `effect`（含 `generation`） | §3.1、§3.3 | 并发改写同一 payload 时，旧尝试不覆盖新行 |
| 2 | 幂等键从 message_id 提到事件 ID，配读位点 | §3.4 | 适配器重连重放不产生重复回复 |
| 3 | 队列满 / 失败分开计数；失败按「连续 N 次」升健康 | §3.2、§3.1 | `/status` 能区分 `busy_dropped` 与 `failed` |
| 4 | chat 插件加 `observe` 模式：照跑触发判断，只记录「本来会回什么」 | §3.5 | 调触发词不必真去群里刷屏 |

第 3 条成本最低（约 40 行），建议先做；第 4 条收益最直观。

## 7. 引用注意

- 它是 research prototype，README 明说可能有 bug、风格不一致；**引用时别当权威**。
- 依赖 DSH 与 Cordis 的 alpha 版本，API 不稳定，代码位置会漂。
- 「文章里讨论的设计」与「原型实现」不是一回事，README 有明确声明。
- 真正可复用的是**概念与规则**（`001-domain.md` 的两张表），不是代码。

## 8. 一条旁证

这个仓库的代码是 coding agent 写的，作者自己承认风格不一致。但它区别于其他 agent 框架的东西
**不在代码里**——在九篇按依赖顺序排列的文档、那张 13 行的拒绝概念表、以及「一个人不是领域概念」
这类判断里。

对应到本仓：AI 重写插件的成本已经很低，稀缺的不是实现，是**决策和它的理由**。
`docs/` 里那些带清单编号的注释、这三份提案里的取舍表，价值高于它们描述的代码。
