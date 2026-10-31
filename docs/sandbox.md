# 沙箱会话（多会话 + 持久化）

沙箱选项卡原来是「一屏一次性对话」：`bubbles` 只活在组件内存里，切页即丢。
现在每个会话有自己的 chat_id，记录落 `data/sandbox/`，机器人记忆仍落
`data/sessions/`——两者用同一个 chat_id 对齐。

## 为什么两套文件

| 文件 | 谁写 | 内容 | 面向 |
| --- | --- | --- | --- |
| `data/sessions/<mode>_<chat_id>.json` | 插件（`extensions/_lib/agent.py`） | `role/content/tool_calls`，被 `_trim` 裁剪 | 喂给模型 |
| `data/sandbox/<chat_id>.jsonl` | 宿主（`src/webui/sandbox_store.rs`） | 段结构、图片引用、时间戳 | 给 WebUI 渲染 |

上下文文件里有工具调用、群聊 `<uid>` 前缀、没有时间戳也没有图片，不能直接
当聊天记录渲染；反过来把展示记录喂模型又会把 tool 消息弄丢。所以分两套，
但**只用 chat_id 这一根线对齐**：删会话 = 两个文件一起删。

## 会话标识

```
chat_id = <base>.<6 位十六进制>
base    = 10000（私聊）| dev-group（群聊）
```

- 缺省会话保留裸 base（`10000` / `dev-group`），让改造前已有的记忆文件继续可见可用；
- 新会话带后缀，因此 `(message_type, chat_id)` 在插件侧天然是新的键 → 独立记忆；
- 后缀只用 `[a-z0-9]`，避免进 `data/sessions/<mode>_<chat_id>.json` 时与文件名
  分隔符 `_` 冲突（`pybridge::load_sessions` 用 `rpartition("_")` 切）；
- 适配器侧对 chat_id 做白名单校验（`validChatID`），挡住目录穿越。

## 落盘格式

`data/sandbox/index.json`（原子写，tmp + rename）：

```json
{ "version": 1, "sessions": [
  { "chatId": "10000.a1b2c3", "mode": "private", "name": "帮我看下今天的排班",
    "created": 1793341418078, "updated": 1793341420526, "count": 2, "autoNamed": false }
]}
```

`data/sandbox/<chat_id>.jsonl`，一行一条，追加写：

```json
{"v":1,"ts":1793341419188,"dir":"in","id":"7","segments":[{"text":{"text":"你好"}}]}
{"v":1,"ts":1793341420526,"dir":"out","text":"收到，会话正常在线"}
```

- 追加写：崩溃最多丢最后一行；读取时坏行/版本不符直接跳过，不拖垮页面；
- 图片的 dataURL 落盘成 `data/sandbox/media/<fnv 哈希>.<ext>`，记录里只留
  `/api/sandbox/media/xxx`（同一张图会命中同一文件）；
- 会话名：新建时占位「新会话」，首条带文本的消息到来时自动取前 40 字；
  用户改过名（`autoNamed=false`）就不再被覆盖。

## 保留与配置

`config/webui.yaml`（模板见 `config/webui.yaml.example`）：

```yaml
sandbox:
  keep: 500           # 每会话保留条数；超过两倍时才压缩重写
  max_sessions: 50    # 会话上限，超出按最久未用淘汰（连同记忆一起删）
```

## API

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| GET | `/api/sandbox/sessions` | 会话列表（按最近使用倒序） |
| POST | `/api/sandbox/sessions` | 新建，body `{mode}` |
| PATCH | `/api/sandbox/sessions/{chatId}` | 改名，body `{name}` |
| DELETE | `/api/sandbox/sessions/{chatId}` | 删会话（记录 + 机器人记忆） |
| GET | `/api/sandbox/history?session=&limit=` | 历史，正序，`[{dir,id?,segments?/text?,ts}]` |
| POST | `/api/sandbox/send` | 注入，body 增加 `session` 字段 |
| GET | `/api/sandbox/media/{name}` | 会话图片，token 走查询参数兜底 |

`send` 不传 `session` 时落到缺省会话（兼容改造前的旧前端）。

## 记账的两个出口

1. `POST /api/sandbox/send` 成功后记 `in`（带完整段与平台 messageId）；
2. `publish_reply(platform="sandbox")` 记 `out`（适配器不落库，宿主是唯一
   拿得到回复文本的地方）。

`publish_incoming` 对 sandbox **不记账**——它与 send 是同一件事，双写会重复。
消息流 WS 帧新增 `chat_id` 字段（`chat` 仍是不带后缀的展示名），前端据此把
回复归到对应会话。

## 将来把沙箱 tab 迁成插件

现状：沙箱是内置页（`webui/src/tabs.ts` + `views/Sandbox.vue`），因为它需要
WS 订阅与 `inject_event`，而 ext-ui 的 `ctx.api` 只给了 get/post。

留的口子：

- 存储是独立模块 + 独立目录（`data/sandbox/`），迁移时换成插件目录即可，
  REST 契约不用动；
- 会话配置集中在 `config/webui.yaml` 的 `sandbox` 段，可整体搬进插件 config；
- 前端只依赖 `webui/src/api/index.ts` 里的六个函数，插件 ui.js 复用同一组端点。

真要迁，需要先给 ext-ui 补三件东西：WS 帧订阅（`ctx.stream`）、宿主能力白名单
（`ctx.host.injectSandbox(...)`）、插件级数据目录。补齐后沙箱就是第一个试点。
