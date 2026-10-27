# 提案：跨适配器统一消息格式（IR）

状态：已实施（pluginapi v2 全链路落地；与实现有出入处以代码为准——
EventEnvelope 增加了 self_id 字段，napcat 发送侧对 text 段做 CQ 码拆解兼容旧插件。

后续演进（v2 之上）：
- 多适配器共存：SendMessage/SendAction 增加 platform 路由字段，核心按平台
  管理子进程与断流重启循环；新增 InjectEvent 供沙箱注入本地事件
- WebUI 扩展机制：tab 由宿主注册（/api/extensions/ui），沙箱选项卡是
  第一个消费者；输入经 /api/sandbox/send 注入
- 逃生舱动作路由缺省“哪里来哪里去”：python 能力桥读 worker 线程登记的
  消息来源平台，非消息上下文缺省 napcat，可显式传 platform 覆盖）
参考：pi 的做法——内部统一 IR（`Message` + 判别符内容块），转换收敛在 provider 边界（`packages/ai/src/providers/*`），核心零平台知识。

## 1. 现状与问题

消息链路：NapCat → 适配器 → Go 核心 → Rust 宿主 → 插件。

当前 `Event.payload` 是 OneBot 11 裸 JSON，一路穿透到宿主，协议知识全部长在宿主：

- `src/event.rs` 硬解 post_type、CQ 码/数组双格式、at 判定、图片提取
- `business.rs` 的 `MsgCtx` 用 OneBot 味字段（`message_type: private/group`、`chat_id: i64`）
- `main.rs` 发送时手拼 `send_private_msg` / `send_group_msg` params
- Python 插件直呼 OneBot action（`send_action("delete_msg")`），ctx 也是 OneBot 字段

后果：

1. 接第二个平台时宿主必须改版。适配器生态失去意义，宿主成为瓶颈。
2. `chat_id: i64` 出不了 QQ 系，Discord/微信等字符串 ID 平台无法表达。
3. 每加一种发送能力（图片、引用、@）都要动宿主。

## 2. 目标与非目标

目标：

- 宿主与插件零平台知识，协议知识收敛到各适配器
- 接新平台只写适配器，宿主零改动
- 发送走结构化段列表，不再手拼 JSON

非目标：

- 不追求覆盖全平台动作面（踢人、禁言等长尾走逃生舱，见 §4.3）
- 不做有损平台特性的无损表达（`raw` 兜底，信息不丢）

## 3. 设计原则

1. **IR 最小公共集**：两个以上主流平台都有的概念才进 IR；单平台特性走 extension 段。
2. **raw 逃生舱**：信封保留平台原始 JSON，任何 IR 丢的信息都能取回。
3. **ID 全 String**：跨平台 ID 类型不可假设。
4. **proto 真类型**：适配器↔核心用 proto message（字段编号演进），不再往 bytes 里塞约定 JSON。

## 4. proto 草案

### 4.1 服务与事件信封

```proto
syntax = "proto3";

package pluginapi.v2;

import "google/protobuf/struct.proto";

// 适配器插件对核心暴露的服务。
service Adapter {
  // 适配器把平台事件翻译成统一信封，逐条推给核心
  rpc StreamEvents(StreamEventsRequest) returns (stream EventEnvelope);
  // 统一发送：段列表 → 平台消息。核心与插件的主要发送路径
  rpc SendMessage(SendMessageRequest) returns (SendMessageResponse);
  // 动作逃生舱：透传平台自有 action（撤回、踢人、禁言等长尾能力）
  rpc SendAction(SendActionRequest) returns (SendActionResponse);
}

message StreamEventsRequest {}

enum EventKind {
  EVENT_KIND_UNSPECIFIED = 0;
  EVENT_KIND_MESSAGE = 1;   // 普通消息
  EVENT_KIND_NOTICE = 2;    // 系统通知（撤回、入群、戳一戳…）
  EVENT_KIND_REQUEST = 3;   // 请求（加好友、加群…）
  EVENT_KIND_META = 4;      // 心跳、生命周期
}

message EventEnvelope {
  string platform = 1;        // "napcat" / "telegram" / …
  EventKind kind = 2;
  int64 time_ms = 3;          // 平台事件时间，Unix 毫秒；平台缺失时用适配器接收时间
  UnifiedMessage message = 4; // kind == MESSAGE 时必填
  bytes raw = 5;              // 平台原始 JSON，逃生舱
}
```

### 4.2 统一消息

```proto
message UnifiedMessage {
  string id = 1;              // 平台消息 ID（撤回/回复引用用）
  ChatRef chat = 2;
  Sender sender = 3;
  repeated Segment segments = 4;
  bool mentions_bot = 5;      // 是否 at 了机器人；平台不支持 at 时为 false。
                              // 由适配器计算（只有它知道自己的 bot ID）
}

message ChatRef {
  oneof kind {
    string private_id = 1;    // 私聊对方用户 ID
    string group_id = 2;      // 群 ID
    string channel_id = 3;    // 频道 ID（预留，暂不使用）
  }
}

message Sender {
  string id = 1;
  string nickname = 2;        // 平台不提供时为空
}
```

### 4.3 消息段

```proto
// 收录原则：≥2 个主流平台都有的概念才进 IR；
// 单平台特性走 extension。新增段时先插到空闲编号。
message Segment {
  oneof body {
    TextSeg text = 1;
    AtSeg at = 2;
    ReplySeg reply = 3;
    MediaSeg image = 4;
    FaceSeg face = 5;
    MediaSeg voice = 6;
    MediaSeg video = 7;
    MediaSeg file = 8;
    ExtensionSeg extension = 15; // 编号留大，插入新段不挤压
  }
}

message TextSeg {
  string text = 1;
}

message AtSeg {
  string target = 1;   // 用户 ID；"all" 表示全体（支持的平台）
}

message ReplySeg {
  string message_id = 1; // 被回复的消息 ID
}

message FaceSeg {
  string face_id = 1;    // 平台表情 ID，跨平台不保证一致
}

// 媒体统一形态：来源三选一
message MediaSeg {
  oneof source {
    string url = 1;      // 收/发：可直接拉取的 URL
    string file_id = 2;  // 收：平台文件 ID（URL 会过期的平台靠它）
    bytes inline = 3;    // 发：内联二进制（大文件不建议）
  }
  string filename = 4;   // file 段显示名；其他媒体可选
}

message ExtensionSeg {
  string namespace = 1;  // 平台名或扩展名，如 "napcat"
  string type = 2;       // 平台内段类型
  google.protobuf.Struct data = 3; // 平台原始字段
}
```

### 4.4 发送

```proto
message SendMessageRequest {
  ChatRef chat = 1;
  repeated Segment segments = 2;
}

message SendMessageResponse {
  string message_id = 1; // 新消息的平台 ID（撤回用）；平台不回时为空
}

// 逃生舱保持 v1 形态：action 名 + 平台自有 schema 的 JSON
message SendActionRequest {
  string action = 1;
  bytes params = 2;
  int32 timeout_ms = 3;
}

message SendActionResponse {
  bytes data = 1;
  string status = 2;
  int32 retcode = 3;
}
```

### 4.5 FFI 边界

适配器↔核心走 proto message（类型安全）。核心→Rust 宿主的 `onEvent` 回调仍是 C ABI 字节：Go 把 `EventEnvelope` marshal 成 JSON 传过去，Rust 用 serde 解析。C ABI 上 JSON 字节最好排查，暂不引入 protobuf 运行时。

## 5. 迁移步骤

适配器与核心必须同版本发布，靠握手版本号兜底。WIP 期没有第三方适配器，直接切 v2，不做双版本共存。

1. **pluginapi v2**：用 §4 替换 `adapter.proto`，重新生成 pb；`plugin.go` 的 `ProtocolVersion` 升 2，cookie 改 `loquat-adapter-v2`。
2. **napcat 适配器**：写 OneBot ↔ IR 转换器。事件侧：post_type/message_type 判 kind；CQ 码与数组双格式解析成 segments；at 判定填 `mentions_bot`；ID 转 String。发送侧：ChatRef 选 `send_private_msg`/`send_group_msg`；段列表翻成 OneBot 数组格式消息段。`event.rs` 的测试用例（`&#44;` 转义、`qq=all` 不算 at、`qq=1001≠100`）整体移植成 Go 单测。
3. **Go 核心**：`events` 队列类型换 `*EventEnvelope`；加 `SendMessage` RPC 转发；`onEvent` 对外签名不变（仍 `func([]byte)`），内容换成 envelope JSON。
4. **Rust 宿主**：`event.rs` 改解 envelope JSON；`Incoming` 改型（chat 变枚举、segments 进结构体，text/images 变派生值）；`business.rs` 的 `MsgCtx` 与 Python dict 转换同步适配；`ffi.rs` 加 `LoquatSendMessage` 绑定；`main.rs` 回复路径改调 SendMessage，SendAction 保留。
5. **Python 插件**：`ctx` 字段适配（`chat_id` 变 string、新增 `segments`）；`_lib` 提供统一的 `send(chat, segments)` 能力桥；`send_action` 桥保留，`recall`/`poke`/`react` 等长尾动作继续走逃生舱。排查 `chat`/`qa`/`status` 三个插件。
6. **webui/console**：`publish_reply` 的 chat 展示（`群 {id}`）适配 ChatRef；消息流若展示入站消息同样适配。
7. **清账**：删 `event.rs` 已迁移的解析与测试；更新 `docs/mvp.md`、`docs/agent.md`；checkList 记录。

PR 划分：步骤 1–3 一个 PR（proto + 适配器 + 核心，须一起跑通），步骤 4–6 一个 PR（宿主 + 插件 + webui），步骤 7 收尾。

## 6. 风险

- **一次性迁移面大**：适配器、核心、宿主、插件、webui 都动。缓解：按上面 PR 划分，每步有测试兜底；CQ 码解析测试整体移植，不重写。
- **IR 设计不当返工**：原则 §3 兜底——拿不准的概念先不进 IR，走 extension；IR 加段比删段容易。
- **插件生态断裂**：当前只有内置插件，`send_action` 桥不删，破坏面可控。
