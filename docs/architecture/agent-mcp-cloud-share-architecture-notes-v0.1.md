# Agent / MCP 介入、云原生与一键分享架构讨论笔记 v0.1

- 状态：Discussion Note / Architecture Candidate
- 日期：2026-09-02
- 范围：架构讨论与后续 ADR 输入，不代表当前 MVP 已实现
- 基线：[全局 Slot Architecture v0.2](../../jueming_global_architecture_handoff_v0.2.md)、[Phase 0 合同](mvp-phase0-contracts-v0.1.md)、[AI Agent / PDF / MCP 架构 v0.1](ai-agent-multimodal-pdf-native-mcp-architecture-v0.1.md)
- 配套 Draw.io（宽松中文布局）：[agent-mcp-cloud-share-intervention-v0.2.drawio](diagrams/agent-mcp-cloud-share-intervention-v0.2.drawio)
- 完整时序图：[agent-mcp-cloud-share-sequences-v0.1.drawio](diagrams/agent-mcp-cloud-share-sequences-v0.1.drawio)
- Serverless Research Compute 延伸：[serverless-research-compute-model-v0.1.md](serverless-research-compute-model-v0.1.md)
- 上一版（固定页面布局）：[agent-mcp-cloud-share-intervention-v0.1.drawio](diagrams/agent-mcp-cloud-share-intervention-v0.1.drawio)

## 0. 本轮结论

1. **Agent 和 MCP 不应“插入 Kernel 内部”。** 它们只在 `Resource Gateway`、`Tool Gateway`、`Operation Scheduler` 和 `Approval / Commit Gateway` 四个受控位点介入。Kernel Core 不理解 prompt、模型会话、MCP session 或远程 tool schema。
2. **Prompt 不能直接成为可执行 Pipeline。** Prompt 先产生 `IntentDraft`，再编译成有 schema、版本、输入输出和权限声明的 `PipelineSpec`；只有通过静态校验和策略检查的 Pipeline 才能交给 Scheduler。
3. **“用自然语言生成正则”属于 Proposal，不属于搜索真值。** 模型输出 `RegexDraft`；确定性 Regex Compiler、复杂度限制、样例测试和搜索预览通过后，才生成可执行 `SearchSpec`。替换仍需 `PatchProposal → ApprovalTicket → Kernel Command`。
4. **数据历史至少分为四本账。** Canonical Revision、Agent Run Ledger、MCP Connection Audit、Operational Telemetry 不能合并。设置变更另有配置审计，不伪装成项目 Revision。
5. **最开始的 Local / Server Compatible Kernel 方向仍然正确；当前实现“概念兼容、部署尚未兼容”。** 领域对象、typed protocol 和 UI façade 已对齐；传输抽象、Chunk/Slice、OpLog、多写者 Revision、远程 Repository 和同步协议还没有落地。
6. **云端不是把当前 `.jm` 或 SQLite 上传后在线编辑。** 云端使用相同 Kernel 语义，但采用 PostgreSQL 元数据、对象存储、追加 Operation Log、远程 Data RPC 和分布式调度；本地与云端通过 Operation / Manifest / Object Ref 同步。
7. **一键分享的正确模型是“不可变 Publication Snapshot + Content-addressed Objects + 可变 Share Channel”。** 分享不暴露工作工程数据库，也不把 Run Ledger、凭据或批注默认打包进去。
8. **设置必须从当前单一 `KernelClient` 概念中继续拆义。** `KernelClient` 只承载工程领域；设备偏好、Agent 控制和云同步分别由 `AppSettingsClient`、`AgentControlClient`、`SyncClient` 承载，再由桌面端组合 façade 统一注入 UI。

## 1. 与最开始 Slot 架构的兼容性审计

### 1.1 原始设计真正要守住的边界

最开始的设计不是“所有东西都做成 Slot”，而是：

```text
Slot = 数据流在哪里允许介入
Operator = 谁 / 如何计算
Schema = 交换什么
Operation RPC = 如何控制
Data RPC = 如何交换大数据
Artifact / Patch = 计算结果如何离开扩展运行时
Kernel = 谁拥有 canonical invariants
```

因此 Agent、MCP、云端和分享能力都应复用这套边界，而不是扩充 `Segment`、`Alignment` 或 `SlotDescriptor` 来承载会话、权限和网络状态。

### 1.2 当前代码与原始设计的对应关系

| 原始设计 | 当前实现证据 | 判断 |
|---|---|---|
| Stable opaque ID | `jueming-core` 已有 Project/Document/Segment/Alignment/Revision 等类型化 ID | 兼容 |
| UI 只通过 typed façade | [`kernel-client.ts`](../../apps/desktop/src/domain/kernel-client.ts) 集中 Tauri `invoke` | 兼容，但 façade 过宽 |
| Kernel / Protocol / Storage 分离 | `jueming-kernel`、`jueming-protocol`、`jueming-storage` 已拆 crate | 兼容 |
| Canonical 写入生成完整 Revision | Kernel mutation 共用 revision advancement；`revisions/` 保存可重开快照 | 兼容单机合同 |
| UI State 与 Canonical Data 分离 | Pinia 与 Tauri Store 保存设备设置，`.jm` 保存工程 | 基本兼容 |
| Local / Remote transport 可替换 | 当前 `createKernelClient()` 直接调用 Tauri `invoke` | 尚未实现 |
| Operation RPC / Data RPC 分离 | 当前只有本地 command DTO；大数据仍由完整 `ProjectSnapshot` 传递 | 尚未实现 |
| Chunk / Slice out-of-core | 当前工程使用完整 JSON snapshot | 尚未实现 |
| Append-only Operation Log | 当前保存 Revision 快照，没有独立可同步 OpLog | 尚未实现 |
| Local / Server compatible storage | `KernelService` 仍接收本地 path 与完整 snapshot | 尚未实现 |
| 多写者 Revision / conflict | 当前 `RevisionId` 按本地顺序递增 | 尚未实现 |
| Slot Runtime | Phase 0 冻结了状态语义，但代码尚无完整 Registry / Scheduler | 合同存在，Runtime 未接通 |

### 1.3 兼容性结论

当前实现没有背离最开始的设计，但它只实现了 **Local Reference Adapter**：

```text
正确且应保留：
Domain Core → Versioned Protocol → KernelService → Storage Adapter
UI → typed façade → Tauri Adapter

下一步要抽离：
Tauri invoke ≠ KernelClient 语义本身
local path ≠ Project identity
full ProjectSnapshot ≠ Data RPC
local increment RevisionId ≠ multi-writer Operation identity
Tauri Store settings ≠ Canonical Kernel API
```

现在仍处于可以低成本补齐 Ports/Adapters 的阶段。如果等 Agent Tool、远程 MCP 和公开 Share API 都引用当前 Tauri 方法名或本地路径，再拆就会形成外部兼容债务。

## 2. Agent / MCP 的介入位点

### 2.1 四个唯一入口

| 入口 | 用途 | 可以访问 | 可以输出 | 明确禁止 |
|---|---|---|---|---|
| Resource Gateway | 只读上下文 | DataView、Slice、Artifact、Schema、Slot 状态 | 分页 Resource、受限 Bundle | 直接返回工程路径、凭据或全部数据库 |
| Tool Gateway | 执行动作 | 内建 Tool、MCP Tool、Operator façade | ToolResult、Artifact、PatchProposal | 绕过策略直接调用外部副作用或 Kernel mutation |
| Operation Scheduler | 跑 Pipeline / Operator | `PipelineSpec`、Slot Binding、DataHandle | Artifact、Run Progress、失败状态 | 把 prompt 字符串当作可执行代码 |
| Approval / Commit Gateway | 提交 canonical 修改 | PatchProposal、base revision、ApprovalTicket | Kernel Command Result、Revision | Agent/MCP 自签审批票据或直接写 Store |

这四个入口之后，MCP 的真实位置是：

```text
Jueming as MCP Host:
Agent → Tool/Resource Gateway → MCP Client Adapter → External MCP Server

Jueming as MCP Server:
External MCP Host → MCP Server Adapter → Tool/Resource Gateway → KernelClient

MCP-backed Operator:
Scheduler → explicitly registered McpOperatorAdapter → Tool Gateway → MCP Client
```

第三条只能通过显式 `OperatorDescriptor` 成立。发现一个 MCP Tool 不等于自动绑定一个 Slot；必须确认输入/输出 Schema、确定性、幂等性、执行位置、数据授权、缓存策略和版本指纹。

### 2.2 Prompt 形成 Pipeline 的正确路径

Prompt 是 authoring interface，不是 runtime ABI。建议流程：

```text
自然语言需求
  → PromptTemplate + 当前 UI Context
  → IntentDraft
  → Plan Compiler
  → typed PipelineSpec
  → Schema / Capability / Policy Validation
  → Dry-run Plan + Cost / Data Preview
  → 用户确认
  → Scheduler
  → QueryStep / OperatorStep / ToolStep / AgentStep
  → Artifact 或 PatchProposal
  → 可选 Approval / CommitStep
```

`PipelineSpec` 至少需要固定：

| 字段 | 作用 |
|---|---|
| `pipeline_id` / `version` | 稳定身份与更新策略 |
| `base_revision` 或输入选择器 | 运行所依据的数据版本 |
| `nodes` / `edges` | DAG 拓扑，不依赖 prompt 的自然语言顺序 |
| `node_kind` | Query / Operator / Tool / Agent / HumanApproval / Commit |
| `input_schema` / `output_schema` | 每一步的可验证合同 |
| `operator_ref` / `tool_ref` / `prompt_ref` | 解析后的稳定引用与 generation |
| `DataGrant` / `ToolGrant` | 数据范围、工具范围和网络范围 |
| `retry_policy` / `timeout` / `budget` | 运行约束 |
| `cache_policy` / `determinism` | 是否可复用及缓存 Key |
| `failure_policy` | stop / skip / fallback-to-explicit-node；禁止隐式 Provider 回退 |
| `output_policy` | Artifact、PatchProposal 或仅回答 |

Prompt、Profile、Tool Schema 和 Provider Binding 在运行开始时都应形成 snapshot hash。后续修改设置不能改变旧 Run 的解释。

### 2.3 Prompt 自动生成正则的路径

建议新增逻辑对象 `RegexDraft`，但不要把它放进 Canonical Core：

```text
用户描述搜索目标
  → Agent / regex tool 生成 RegexDraft
  → Regex Validator
  → Safety Analyzer
  → Sample Test Cases
  → Kernel Query Preview
  → 用户采用
  → SearchSpec
```

`RegexDraft` 需要携带：

- pattern、flags、engine/dialect；
- 自然语言解释和预期匹配；
- 正例、反例和边界样例；
- 生成所用 prompt/model/tool generation；
- 建议的语言侧、范围和大小写设置；
- 最大输入长度、超时或 step limit；
- safety finding，例如 nested quantifier、灾难性回溯风险、lookbehind 支持差异。

关键规则：

1. 模型生成的正则先在样本 Slice 上执行，不能直接扫全工程。
2. Regex engine 必须成为 `SearchSpec` 的显式字段，不能依赖前端 JavaScript 与 Rust regex 方言“碰巧一致”。
3. 优先选择可限制复杂度的引擎；不支持的特性在 Validator 阶段报错，不悄悄改写语义。
4. “采用正则”只更新搜索会话或用户保存的 SearchPreset，不产生 Canonical Revision。
5. 批量替换先产生 Replace Preview / PatchProposal，用户确认后才进入 Kernel Command。

### 2.4 MCP Prompt 如何介入

MCP Prompt 应视为来自外部 Server 的不可信模板：

```text
MCP prompts/list
  → Prompt Registry Candidate
  → provenance + server identity + generation
  → 用户显式选择 / AgentProfile allowlist
  → Prompt Resolver
  → Context Builder
```

禁止行为：

- MCP Prompt 自动写入 system policy；
- Prompt 声称“需要全部工程数据”就扩大 DataGrant；
- Prompt 内文本改变 Tool 风险级别；
- Server 更新 Prompt 后悄悄改变正在运行的 Pipeline；
- 把 Prompt 作为 Slot Provider。

### 2.5 MCP Tool 如何介入 Kernel

MCP Tool 不获得 Kernel 句柄。所有调用都经过：

```text
ToolDescriptor
  → AgentProfile allowlist
  → ToolGrant / DataGrant
  → Tool Gateway validation
  → MCP Client session pinned to generation
  → Tool result validation
  → Artifact / PatchProposal
```

只有以下链路可以进入 Canonical Kernel：

```text
PatchProposal
  + immutable base_revision
  + user-issued one-time ApprovalTicket
  + Kernel invariant validation
  = new Revision
```

即使 Jueming 自己作为 MCP Server 对外暴露 `propose_segment_edits`，该 Tool 也只能创建 Proposal；不要暴露 `update_database`、`write_project_file` 或 `execute_sql` 一类逃逸接口。

## 3. Data / Log 与设置层

### 3.1 四本账与一份配置历史

如果“介入 data log”指 Data 与 Log 两个层面，应明确分开：

| 记录 | 真值范围 | 典型内容 | 保留位置 | 是否同步/分享 |
|---|---|---|---|---|
| Canonical Revision / Operation Log | 工程领域真值 | ChangeSet、稳定对象 ID、作者、父 Revision、冲突 | `.jm/revisions`；云端 OpLog | 工程同步；分享只引用选定 snapshot |
| Agent Run Ledger | Agent 可解释性 | plan、step、tool call、prompt hash、token、DataGrant、结果引用 | 默认设备本地或私有云日志 | 默认不分享，可按用户选择导出 |
| MCP Connection Audit | 外部连接审计 | server identity、transport、tool generation、grant、错误 | 设备或组织审计库 | 不进入工程分享 |
| Operational Telemetry | 系统运行 | latency、queue、crash、resource usage | 本地诊断或匿名遥测 | 不包含正文，受隐私设置控制 |
| Configuration Audit | 设置变化 | scope、key、old/new hash、actor、policy source | 设置服务 / 设备日志 | 仅同步可同步 scope |

所有跨层事件使用相关 ID 串联，而不是复制正文：

```text
trace_id
agent_run_id
pipeline_run_id
step_id
tool_call_id
mcp_connection_id
operation_id
base_revision_id
result_artifact_id
```

Run Ledger 默认只保存内容 hash、Resource URI、长度、模型/工具版本和脱敏摘要。是否保存完整 Prompt/Response 是显式隐私选项，不能因为“便于调试”默认开启。

### 3.2 设置作用域必须拆分

当前 `KernelClient` 暂时包含 `loadAppSettings/saveAppSettings/resetAppSettings`，这是桌面 MVP 的便利实现，不应固化为长期 Kernel 合同。

建议的客户端边界：

```text
DesktopServices
  ├─ KernelClient          工程 Query / Command / Revision
  ├─ AppSettingsClient     设备 UI、输入、缓存与本地隐私
  ├─ AgentControlClient    Profile、Prompt、Run、Approval
  ├─ CapabilityClient      Slot / Operator / Tool / MCP 状态
  └─ SyncClient            账号、云工程、上传与 Share
```

UI 仍不直接访问 Tauri Store、数据库或 HTTP SDK；这些客户端可以在桌面端由一个组合 façade 注入，但协议和存储责任不再混成一个接口。

### 3.3 设置数据模型

| Scope | 示例 | 保存位置 | 同步规则 |
|---|---|---|---|
| Device | theme、窗口、快捷键、本地缓存、local-only mode | Tauri Store | 默认不跨设备 |
| Secure Device | API key、OAuth refresh token、MCP client secret | OS secure storage | 永不进入普通设置 JSON 或工程 |
| Account | 界面语言、可同步偏好、默认分享策略 | Cloud Settings | 用户登录后同步 |
| Project | PipelineDefinition、PromptPreset、导入/发布 Profile | `.jm` project config / cloud project metadata | 随工程同步，可选择随分享发布 |
| Provider Binding | 本机模型路径、GPU、MCP endpoint | device registry | 不随工程强绑定；工程只声明 capability requirement |
| Organization Policy | 禁止外发、允许的 Provider/MCP、保留期 | policy service | 只读投影，优先级最高 |
| Session Override | 当前 Run 的 model、预算、数据范围 | memory + Run snapshot | Run 结束即释放 |

不要用简单的“后写覆盖前写”合并安全设置。有效权限是多层约束的交集：

```text
EffectiveGrant = OrganizationPolicy
               ∩ DevicePrivacyPolicy
               ∩ ProjectPolicy
               ∩ UserConsent
               ∩ AgentProfile
               ∩ CurrentRequestScope
```

### 3.4 UI Setting 如何展示 Agent / MCP

设置 UI 读取的是 `SettingsProjection` 和 `CapabilityProjection`，而不是直接读 Registry 内部对象：

- Provider/MCP 连接：展示 identity、transport、last handshake、tool generation 和 grant；
- Prompt：展示来源、版本、适用范围、是否被 Profile 引用；
- Pipeline：展示验证状态、缺失 Slot/Tool、预计数据范围和成本；
- Regex Draft：位于搜索工作流，不作为全局 Agent 设置；
- Tool 权限：按 read / compute / propose / external side effect 分类；
- 高风险开关不允许纯 toggle 后立即生效，必须显示 scope 与后果；
- `UNBOUND`、`DISABLED_BY_POLICY`、`AUTH_REQUIRED`、`SCHEMA_INCOMPATIBLE`、`STALE_GENERATION` 是不同状态。

## 4. 云原生支持

### 4.1 不是“云化桌面 App”，而是同语义的第二个 Kernel Host

目标形态：

```text
Desktop UI / Web UI / CLI
        ↓
Typed Client Contracts
        ↓
Local Transport | Remote Transport
        ↓
Local KernelHost | Server KernelHost
        ↓
Shared Kernel Core + Validation + Query Semantics
        ↓
Ports
  ProjectRepository
  OperationLog
  Blob/ObjectStore
  ArtifactStore
  EventBus
  Scheduler
  Identity/Policy Context
        ↓
Local Adapters | Cloud Adapters
```

### 4.2 云端分为四个平面

| 平面 | 组件 | 责任 |
|---|---|---|
| Edge / Local | Desktop Kernel、local cache、offline OpLog | 离线工作、低延迟交互、待同步操作 |
| Control Plane | Auth、Tenant、Project Catalog、Policy、Share、Billing/Quota | 身份、授权、元数据和管理 |
| Data Plane | Object Storage、Chunk Service、Artifact Store、range/stream gateway | 大文件、Chunk、PDF、Artifact 的 hash 寻址和流式传输 |
| Compute Plane | Server Kernel、Scheduler、Operator Worker、Agent Worker | 校验、查询、Pipeline、远程计算 |

### 4.3 Local / Cloud Adapter 映射

| Port | Local Adapter | Cloud Adapter |
|---|---|---|
| ProjectRepository | `.jm` manifest + local catalog/SQLite（演进目标） | PostgreSQL tenant/project metadata |
| OperationLog | local append-only journal | partitioned server OpLog / ordered stream |
| BlobStore | filesystem content-addressed objects | S3-compatible object storage |
| ArtifactStore | `.jm/cache` + local artifact metadata | object storage + artifact index |
| EventBus | in-process / Tauri events | durable queue + WebSocket/SSE projection |
| Scheduler | local bounded executor | queue + lease + worker pool |
| Data RPC | borrowed view / local IPC | HTTP/2 or QUIC range/stream |
| SecretStore | OS keychain | cloud KMS / secret manager |

Kernel Core 不依赖 PostgreSQL、S3、Kafka、Tauri 或某个云厂商 SDK。Server Host 在进入 Core 前注入 tenant、actor、request policy 和 transaction context。

### 4.4 同步协议

禁止同步数据库文件。同步单位是：

```text
ClientOperation
  client_operation_id
  project_id
  actor/device_id
  base_server_revision
  preconditions
  typed ChangeSet
  referenced object hashes
```

服务端流程：

1. 按 `client_operation_id` 幂等去重；
2. 拉取缺失 object hash，而不是重传已有对象；
3. 对 base revision 和对象前置条件做校验；
4. 可交换操作自动 rebase；
5. 同一 Segment 内容修改产生显式冲突；
6. Alignment 结构冲突生成 `AlignmentConflict`，不采用 last-write-wins；
7. 服务端序列化成功操作并签发 Server Revision；
8. 客户端收到映射并更新本地 pending state。

多写者环境中建议拆开：

- `OperationId`：全局唯一、可离线生成；
- `ClientChangeId`：设备本地待同步身份；
- `ServerRevisionSeq`：服务端项目内有序序列；
- `RevisionId`：对外稳定引用，是否继续使用顺序数需单独 ADR。

不要让本地临时 `RevisionId + 1` 与远程最终 Revision 共用同一语义。

### 4.5 远程 Query 与大数据

远程 `KernelClient` 不能继续返回整个 `ProjectSnapshot`：

- UI 首屏只取 ProjectSummary、Visible Slice、Alignment Slice；
- PDF 页面使用 page/tile Artifact 与 range 请求；
- Search/KWIC 返回游标和稳定 anchor；
- Annotation/Alignment 按投影列加载；
- 大批量 Operator 通过 DataHandle 读取 Arrow/binary batches；
- Event 只通知 identity/version，UI 再按需 query 新投影。

这与最开始的 Chunk/Slice 设计完全一致，也是云端成本和亿级 Token 支持的必要条件。

### 4.6 云兼容整改优先级

1. 把 TypeScript `KernelClient` 拆成语义接口与 `TauriKernelTransport` 实现；
2. 从 `KernelService(path, snapshot, ...)` 提取 Repository / Transaction ports；
3. 引入 OperationEnvelope、idempotency key、actor 和 precondition；
4. 引入独立 Operation Log，Revision 快照变为检查点而非唯一同步源；
5. 将 full snapshot query 改成 Slice/Projection；
6. 冻结 Local/Remote 同语义的 Query/Command errors；
7. 最后再引入 HTTP、PostgreSQL、对象存储与账号系统。

## 5. 一键上传与分享

### 5.1 核心模型

不要共享可变工作数据库。使用以下对象：

| 对象 | 语义 |
|---|---|
| WorkingProject | 用户持续编辑的本地/云工程 |
| PublicationPlan | 从指定 Revision 生成的发布预览、隐私检查、大小和缺失项 |
| PublicationManifest | 逻辑对象到 content hash 的不可变清单 |
| PublicationSnapshot | 已完成、不可变、可验证的一次发布 |
| ShareChannel | 可变的分享频道，可把稳定链接指向新的 Snapshot |
| ShareToken / AccessPolicy | 访问方式、权限、过期、撤销和下载/派生策略 |
| ObjectBlob | 以 hash 寻址的 PDF、Chunk、Artifact、缩略图或导出文件 |
| ForkRecord | 从 Snapshot 创建新 Project 的 provenance |

两类链接同时存在：

```text
/s/{share_slug}       稳定 ShareChannel，可由所有者更新 head
/p/{snapshot_id}      不可变 permalink，永远指向同一 PublicationSnapshot
```

### 5.2 元数据数据库建议

| 表/集合 | 关键字段 | 说明 |
|---|---|---|
| `users` | user_id、status | 账号主体 |
| `organizations` | org_id、policy_ref | 团队与策略边界 |
| `projects` | project_id、owner_scope、head_revision | 工作工程元数据 |
| `project_memberships` | project_id、principal_id、role | 云工程 ACL |
| `operations` | operation_id、project_id、server_seq、actor、payload_ref | 追加 Operation Log |
| `revisions` | revision_id、project_id、parent、manifest_hash | Canonical 检查点 |
| `object_blobs` | content_hash、size、media_type、storage_key、encryption_scope | 对象存储索引；内容不可变 |
| `object_refs` | owner_type、owner_id、content_hash、role | Project/Revision/Publication 对 Blob 的引用 |
| `publication_snapshots` | snapshot_id、source_project、source_revision、manifest_hash、state | 一次不可变发布 |
| `publication_items` | snapshot_id、logical_id、role、content_hash、visibility | 发布清单的可查询展开 |
| `share_channels` | channel_id、owner、slug_hash、head_snapshot、state | 稳定分享入口 |
| `share_access_policies` | policy_id、visibility、download、fork、agent_access | 访问能力 |
| `share_tokens` | token_hash、channel_id、expires_at、revoked_at | 只存 hash，不保存明文 bearer token |
| `upload_sessions` | upload_id、manifest_hash、idempotency_key、state | 上传事务 |
| `upload_parts` | upload_id、content_hash、part、etag、state | 大对象续传 |
| `fork_records` | target_project、source_snapshot、actor | 复用对象时保留来源 |
| `audit_events` | actor、action、target、trace、created_at | 分享、访问、撤销与下载审计 |

`object_blobs` 只保存对象元数据，正文位于对象存储。数据库不能承载大 PDF、页图、Embedding 或完整 Artifact blob。

### 5.3 一键分享流程

```text
用户点击分享
  → 选择 current revision / profile
  → Kernel 生成 PublicationPlan
  → 隐私、许可证、批注、Agent 日志与外部资源检查
  → 用户确认可见性和允许动作
  → 本地构建不可变 PublicationManifest
  → 计算 content hashes
  → Cloud 返回 missing-hash set
  → 只上传缺失对象，支持 multipart/resume
  → Server 校验 hash、大小、媒体类型和病毒扫描
  → 单事务 finalize PublicationSnapshot
  → 创建或更新 ShareChannel head
  → 返回稳定链接与不可变 permalink
```

`finalize` 必须满足：

- 所有 manifest 引用的对象都存在且 hash 匹配；
- `source_revision` 在发布过程中没有被悄悄替换；
- idempotency key 重试不会创建重复 Snapshot 或重复计费；
- AccessPolicy 已通过组织与项目策略；
- 失败的 upload session 不形成半可见分享；
- Share URL 创建成功后再向 UI 报完成。

### 5.4 默认发布 Profile

默认包含：

- 指定 Revision 的 Segment、SegmentOrder、Alignment；
- 必需的 Document/Project metadata；
- 用户明确选择的原始文本或 PDF；
- 阅读所需的缩略图/页图/结构 Artifact；
- manifest、schema version、provenance 摘要；
- viewer 所需的轻量 Index/Summary。

默认排除：

- API key、OAuth token、MCP endpoint secret；
- Agent Run Ledger 的完整 Prompt/Response；
- MCP Connection Audit；
- 本地路径和最近工程记录；
- HumanAnnotation、Bookmark 私密标签和本地作者信息，除非用户显式勾选；
- 可重建但体积巨大的 cache；
- 许可不允许再分发的模型或第三方 Artifact。

### 5.5 分享更新、撤销与删除

- 更新分享：创建新 PublicationSnapshot，再原子移动 ShareChannel head；旧 permalink 不变。
- 撤销分享：立即停用 Channel/Token；不直接删除仍被其他 Snapshot 或 Fork 引用的 Blob。
- 删除发布：写 tombstone，等待保留期和引用扫描后 GC。
- 删除对象：采用 mark-and-sweep 或引用图，不依赖容易失真的单一 refcount。
- Fork：新建 ProjectId，复用已有 content hash，写 `ForkRecord`；不能让两个用户共享同一可变数据库。
- 密钥分享：明文 token 只在创建时展示，服务端保存 hash；下载使用短期签名 URL。

### 5.6 一键分享与 Agent/MCP

分享属于外部副作用 Tool，风险级别高于 project mutation：

```text
Agent 可以：create_publication_plan
Agent 可以：explain_privacy_findings
Agent 可以：propose_share_policy
Agent 不可以：publish_without_user_confirmation
Agent 不可以：read_or_copy_share_token_after_creation
MCP Server 不可以：扩大 PublicationManifest 或 AccessPolicy
```

发布成功产生 Share Audit Event，但不应为了“分享了一个链接”修改 Canonical Segment/Alignment。若工程内需要记录发布引用，保存 sidecar metadata，并明确它不是正文历史。

## 6. 建议新增的合同对象

这些对象先进入 architecture schema，不直接加入 `jueming-core`：

| 对象 | 所属平面 |
|---|---|
| PromptTemplate / PromptSnapshot | Agent Control Plane |
| IntentDraft / RegexDraft | Agent Artifact |
| PipelineSpec / PipelineRun | Operation Runtime |
| SearchSpec / SearchPreset | Query / UI Workflow |
| ToolDescriptor / ToolGeneration | Tool Gateway |
| AgentProfile / DataGrant / ToolGrant | Policy Plane |
| ApprovalTicket / PatchProposal | Commit Gateway |
| AgentRunLedgerEntry | Agent Audit |
| McpConnectionProfile / McpAuditEvent | MCP Adapter |
| PublicationPlan / Manifest / Snapshot | Share Control Plane |
| ShareChannel / AccessPolicy / UploadSession | Cloud Control Plane |

`jueming-core` 继续只拥有跨所有 Shell 都成立的工程领域事实和不变量。

## 7. ADR 候选与开放问题

### 7.1 建议冻结为 ADR

1. Agent/MCP 只通过四 Gateway 介入，Kernel Core 不依赖 MCP SDK。
2. Prompt 必须编译为 typed `PipelineSpec`，不可直接执行。
3. Regex 生成采用 Draft → Validate → Preview → Adopt。
4. Canonical Revision、Run Ledger、MCP Audit、Telemetry 分离。
5. `KernelClient` 与 `AppSettingsClient` / `AgentControlClient` / `SyncClient` 拆义。
6. Local / Server 共享 Kernel semantics，通过 Ports/Adapters 替换存储和传输。
7. 云同步复制 Operation、Manifest 与 content-addressed object，不复制数据库文件。
8. 分享采用 immutable PublicationSnapshot；ShareChannel 只保存可变 head。
9. 分享默认排除批注、Run Ledger、凭据和本地路径。
10. MCP Tool 只有显式适配后才能成为 Slot Operator。

### 7.2 仍需验证

1. `RevisionId` 在云端使用服务端序列、ULID，还是“稳定 ID + 独立 seq”。
2. PipelineDefinition 是 project canonical config、project sidecar，还是独立配置 Revision。
3. PromptTemplate 默认按 account、project 还是 organization scope 保存。
4. Regex engine 是否统一为 Rust-compatible dialect，或允许多 dialect 但显式标注。
5. PublicationSnapshot 是否包含原始 PDF 的默认值和许可检查规则。
6. ShareChannel 更新是否默认保留旧版本可见，还是仅所有者可见。
7. 公共分享是否允许 Agent/MCP 访问；建议默认关闭，单独授权。
8. Server Kernel 的首个最小实现是单体 PostgreSQL + S3，还是先做本机 remote-loopback transport。

## 8. 推荐实施顺序

### Phase A：先冻结合同

- Slot Namespace 2.0；
- PipelineSpec、ToolDescriptor、RegexDraft、SearchSpec；
- 四本账与 correlation ID；
- Settings scope 与客户端边界；
- PublicationManifest 与 Share state machine。

### Phase B：本地只读 Agent / MCP

- Resource Gateway；
- Prompt Registry；
- regex draft/validate/preview；
- Agent Run Ledger；
- 只读 MCP Resources/Tools。

### Phase C：Pipeline 与受控写入

- Plan Compiler / Scheduler；
- Artifact / PatchProposal；
- ApprovalTicket；
- Kernel commit 与 stale-base 处理。

### Phase D：远程兼容 Kernel

- transport-neutral clients；
- Repository/ObjectStore/EventBus ports；
- loopback Remote Transport；
- Operation Log 与同步 conformance tests；
- PostgreSQL / object storage adapters。

### Phase E：上传与分享

- PublicationPlan 与隐私预览；
- CAS dedupe 和 upload sessions；
- immutable snapshot / stable channel；
- revoke、fork、GC、审计；
- 最后才开放 Agent 提议分享。

## 9. Draw.io 与 MCP 预备交互

本轮只保存可编辑 `.drawio` 源文件，不再生成 PNG。v0.2 使用中文标签、无固定页面边界、无泳道外框的宽松连线布局；图仍采用官方未压缩 `mxGraphModel` / `mxfile` 结构，并保留稳定 page/cell ID。

### 9.1 页面与稳定 ID

| Page ID | 页面 | 后续修改范围 |
|---|---|---|
| `page-agent-mcp-intervention` | Agent / MCP 介入位点 | Prompt、Pipeline、Regex、Gateway、日志、设置 |
| `page-cloud-kernel` | Local / Cloud Compatible Kernel | transport、ports、local/cloud adapters、sync |
| `page-share-publication` | Upload / Share 数据模型 | publication、CAS、upload、channel、fork |
| `page-drawio-mcp-preflight` | Draw.io MCP 交互准备 | official server、stable IDs、更新流程 |

每页包含 `layer-background-*`、`layer-main-*` 和 `layer-notes-*` 三类稳定 layer cell。核心节点使用语义 ID，例如 `agent-plan-compiler`、`cloud-project-repository`、`share-publication-snapshot`，后续 Agent 不应根据画布坐标猜对象。

### 9.2 首选 MCP 路径

优先采用官方 [jgraph/drawio-mcp](https://github.com/jgraph/drawio-mcp)：

- 本地 Tool Server：`npx -y @drawio/mcp`，使用 `open_drawio_xml` 打开原生 XML；
- 支持 MCP Apps 的 Host：使用 `create_diagram`；
- Draw.io Desktop 直接编辑仓库中的 `.drawio` 文件；MCP Tool Server 与 Desktop 是两条互补路径，不把 Tool Server 误当成 Desktop 自动化接口；
- 需要自托管编辑器时，将 `DRAWIO_BASE_URL` 指向自托管 draw.io；
- 连接线需要整理时只使用一次 `routing: "libavoid"`，不要再叠加 ELK；
- 当前工程不把 Draw.io MCP 作为生产依赖，也不把它接入 Jueming Kernel。

官方 Draw.io 文档说明，`.drawio` 是可验证的 XML；官方仓库提供 `mxfile.xsd` 和统一 XML reference。后续交互前先做：

1. 读取 `.drawio` 全文与当前 hash；
2. 指定 page ID 和 cell ID，不用模糊页面序号；
3. 只修改用户点名的 page/cell；
4. 保留未知 cell、metadata、layer 和 page；
5. 生成新文件后做 XML/XSD validation；
6. 对比 page/cell ID 集合，防止整页重建导致链接失效；
7. 由用户在 Draw.io 中确认并保存。

### 9.3 当前预备状态

- 已创建多页原生 `.drawio`；
- 已使用稳定 page/cell/layer ID；
- 已使用未压缩 XML，方便 diff 与 Agent 修改；
- 已新增 v0.2 宽松中文布局：`page=0`、横向间距加大、泳道边框隐藏、节点与字号放大；
- 不包含凭据、远程 endpoint 或运行时配置；
- 已在当前 Codex 主机的全局配置中注册并启用官方 `@drawio/mcp` STDIO Server；重启 Codex Host 后由新会话加载工具；
- 本地 Draw.io Desktop 已安装，负责人工复核、微调与保存；
- 尚未把 Draw.io 嵌入决明 UI；
- 尚未为 Draw.io 图创建 canonical Revision；它只是架构文档资产。

### 9.4 完整时序图范围

新增的多页时序图使用无限画布和中文宽松布局，按四条完整链路拆页，避免把所有参与者压缩在同一页面：

1. Agent 主动调用 MCP：会话、设置投影、Pipeline 编译、只读数据、正则预览、外部工具、PatchProposal、审批与 Revision；
2. Jueming 作为 MCP Server：连接鉴权、Resource 读取、受控 Tool Proposal、本地审批、逃逸接口拒绝；
3. 本地优先与云原生同步：本地 Revision、OperationLog、推送排序、对象引用、拉取远端操作、结构化冲突恢复；
4. 上传与一键分享：PublicationPlan、隐私确认、CAS 缺失哈希协商、不可变 Snapshot、稳定 Channel、访问、分叉、撤销与 GC。

## 10. 参考资料

- [draw.io：AI diagram generation 与 XSD validation](https://www.drawio.com/docs/reference/diagram-generation/)
- [draw.io：Embed mode / postMessage protocol](https://www.drawio.com/docs/reference/embed-mode/)
- [Official jgraph/drawio-mcp](https://github.com/jgraph/drawio-mcp)
- [Official draw.io XML reference](https://github.com/jgraph/drawio-mcp/blob/main/shared/xml-reference.md)
- [MCP Tools specification 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25/server/tools)

## 11. 本轮建议

下一轮先评审并冻结以下三张“非视觉”表，而不是继续扩图：

1. `PipelineSpec` 节点类型与写入边界；
2. Settings scope / precedence / sync matrix；
3. PublicationManifest 与 Share 数据表字段。

这三处一旦冻结，Agent、MCP、云 Kernel 和分享 API 就能共享同一组稳定合同，避免分别长出四套互不兼容的接口。
