# 决明 Serverless Research Compute 架构讨论稿 v0.1

- 状态：Discussion Note / ADR Candidate
- 日期：2026-09-03
- 范围：长期研究计算、Slot/Pipeline、Agent/MCP、本地与云、PB 语料、可复现分享
- 非目标：不改变当前 MVP 范围，不引入云服务、OCR、外部插件运行时或不可用入口
- 基线：[Phase 0 合同](mvp-phase0-contracts-v0.1.md)、[全局 Slot Architecture v0.2](../../jueming_global_architecture_handoff_v0.2.md)、[AI Agent / PDF / MCP 架构](ai-agent-multimodal-pdf-native-mcp-architecture-v0.1.md)、[Agent / MCP / Cloud / Share 讨论稿](agent-mcp-cloud-share-architecture-notes-v0.1.md)

## 0. 本轮结论

1. **Serverless 的长期价值不是“每个 Slot 一台 Server”，而是把逻辑计算身份与物理执行实例分离。** 决明中的稳定逻辑对象是 Operator Release；某次被平台创建、租赁、冻结或销毁的是 Invocation/Worker，不是 Slot，也不是 Operator 本身。
2. **Slot 是方法学自由度与稳定能力端口，不是函数。** Slot 描述“这里允许什么类型的方法介入”；Operator 描述“谁、以什么版本实现”；Invocation 才描述“这一次在哪里运行”。
3. **Pipeline 必须拆成编辑态、封印态、逻辑计划和物理计划。** 可编辑的 `PipelineDefinition` 不能直接执行；运行和发布使用不可变 `PipelineSnapshot`；Planner 再生成与本地、Serverless、容器、GPU 或分布式集群有关的 `PhysicalExecutionPlan`。
4. **本地与云统一的真正合同是同一个逻辑计划、Schema、Handle、Artifact 和提交语义，不是强制同一个 binary。** Desktop 可以是 `LocalExecutionProfile`，但不应被字面定义成一个云集群。
5. **分布式执行默认按“至少一次派发”设计，只有 canonical commit 提供幂等的提交一次语义。** Worker 不能直接修改工程；它只能发布完整 Artifact 或 PatchProposal，最终仍由带 `command_id` 和 `base_revision_id` 的 Kernel Command 产生 Revision。
6. **PB 语料访问与训练读取共用 DataHandle/LogicalDataPlan/BatchStream，但不能把训练概念塞进 Segment 或 Kernel Command。** 稳定 ID 是语义身份；Partition、Shard、RecordBatch 和本地 dense index 都只是物理表示。
7. **可分享的核心对象升级为 Research Capsule。** 它不是只有图表或 Pipeline，而是 Finding、Evidence、PipelineSnapshot、Slot Binding、数据快照引用、Run Proof、Artifact 和 Provenance 的不可变组合，再由 PublicationSnapshot 和 ShareChannel 对外发布。
8. **Agent 只修改研究画布的 Proposal，Scheduler 才决定执行位置。** Prompt 可以提议 Pipeline Diff、Slot Binding 或运行参数，但模型不能选择绕过隐私、预算、数据驻留与审批策略的后端。

## 1. 纠正三个容易误导实现的类比

### 1.1 “一个 Server 一个函数”只是极端物理实现

可以用它理解按需分配，但不能把它冻结为决明架构：

| 层次 | 稳定性 | 决明对象 | 是否可被平台销毁 |
|---|---|---|---|
| 方法端口 | 长期稳定 | SlotDescriptor | 否 |
| 计算定义 | 版本化稳定 | OperatorDefinition | 否 |
| 可执行发布物 | 不可变 | OperatorRelease | 否 |
| 一次运行 | 短期 | Invocation | 是 |
| 物理载体 | 更短期 | Process / Container / MicroVM / GPU Worker | 是 |

同一个 Operator Release 可以：

- 在桌面进程内运行；
- 在隔离的本地 worker 运行；
- 被 Serverless 平台按请求拉起；
- 作为长时容器 Job 运行；
- 被分片成大量分布式 Task；
- 通过显式适配的 MCP Operator 在远端运行。

这些执行方式不能改变其输入/输出 Schema、参数解释、版本指纹和 provenance 语义。

### 1.2 Slot 不能等同于 Function

Slot 是“哪里允许替换方法”，Function/Operator 是“实际怎么计算”。例如 `genre.judgement` 可以是一个 Slot，而规则、BERT、人工标注、LLM 分类器和 SubPipeline 都可以成为候选 Provider。

如果把 Slot 直接等同于函数，会失去：

- 一个 Slot 多 Provider；
- 方法作者声明固定或可替换部分；
- Provider 不存在时的合法 UNBOUND；
- SubPipeline 递归组合；
- 运行时按隐私、成本、资源和版本解析 Provider；
- 同一方法在本地与云端选择不同物理执行后端。

### 1.3 “同一个 binary”不应成为长期不变量

本地与云端可以在早期复用大量 Rust crate，甚至共享可执行程序，但真正需要冻结的是：

- 相同领域不变量；
- 相同版本化 DTO 和 Schema；
- 相同 PipelineSnapshot；
- 相同 OperatorRelease 解释；
- 相同 DataHandle/Artifact 语义；
- 相同 PatchProposal → Kernel Command → Revision 边界；
- 相同错误、幂等、取消和 provenance 语义。

云端可能拆成多个服务，GPU Operator 可能使用 Python，浏览器 Shell 也不可能嵌入桌面 binary。强制同一 binary 会把部署便利误写成领域合同。

## 2. 统一对象模型

### 2.1 从研究问题到执行实例

| 对象 | 作用 | 可变性 | 身份 / 版本 |
|---|---|---|---|
| ResearchQuestion | 面向研究者的问题与研究目标 | 可编辑 | ResearchQuestionId |
| PipelineDefinition | 研究画布的可编辑方法定义 | 可编辑 | MethodId + MethodRevisionId |
| MethodSlotPolicy | 声明一个方法位点是固定、可替换还是开放 | 随方法修订 | 属于 MethodRevision |
| SlotBindingSet | 当前方法对 Slot 的 Provider 选择 | 随方法修订 | BindingSetId |
| PipelineSnapshot | 运行前封印的完整方法快照 | 不可变 | 内容哈希 / PipelineSnapshotId |
| LogicalExecutionPlan | 由快照编译出的数据与操作 DAG | 不可变、可重建 | plan hash |
| PhysicalExecutionPlan | 带分区、放置、资源和后端选择的任务图 | 单次运行 | RunId 内版本 |
| PipelineRun | 一次研究运行的耐久控制记录 | append-only 状态 | RunId |
| Task | 可重试、可租赁的调度单元 | 单次运行 | TaskId + attempt |
| Invocation | Task 的一次实际执行尝试 | 短期 | InvocationId |
| Artifact | 完整发布的派生结果 | 不可变 | ArtifactId + content hash |
| PatchProposal | 对 canonical 数据的结构化建议 | 不可变 | ProposalId + base Revision |
| ResearchCapsule | 发现、证据、方法、数据与运行证明 | 不可变 | CapsuleId / manifest hash |

### 2.2 Revision 必须分域

`RevisionId` 已被冻结为工程 canonical 历史，不能继续承担所有“版本”含义。建议至少区分：

| 版本域 | 记录什么 | 不记录什么 |
|---|---|---|
| Canonical Revision | Segment、Alignment、Order、HumanAnnotation 等正式工程变更 | Pipeline 画布拖拽、Agent 对话、Worker retry |
| Method Revision | PipelineDefinition、Slot Policy、参数和默认 Binding | 语料正文和对齐关系 |
| Operator Release | 可执行包、Schema、资源声明、代码/模型哈希 | 某次运行状态 |
| Run Ledger | PipelineSnapshot、解析后的 Binding、Task/Invocation、成本和结果引用 | canonical 正文 |
| Publication Snapshot | Capsule manifest、对象引用、访问策略快照 | 工作区当前可变状态 |

推荐：`PipelineDefinition` 采用独立 Method Revision 流；当结果被 Promote 到 canonical 数据时，新 Revision 只引用精确的 `PipelineSnapshotId`、RunId、Artifact hash 和 ProposalId。这样既能追溯方法，又不会因移动画布节点而制造语料 Revision。

## 3. 七个长期平面

| 平面 | 核心职责 | 明确不拥有 |
|---|---|---|
| Research Experience Plane | Research Question、模板、画布、Easy/Advanced 模式、Agent 建议、复现和 Fork | Kernel 事务、Worker 生命周期 |
| Method Plane | PipelineDefinition、Slot Policy、BindingSet、Method Revision、PipelineSnapshot | 物理机器与容器 |
| Governance Plane | Identity、DataGrant、ToolGrant、预算、隐私、许可、审批、组织策略 | canonical 数据内容 |
| Planning & Orchestration Plane | 编译、Schema 检查、Logical/Physical Plan、Task、Lease、Checkpoint、取消和恢复 | Operator 内部算法 |
| Execution Plane | Local Worker、Serverless Function、Container Job、GPU/Distributed Worker、MCP Operator Adapter | canonical commit 权限 |
| Data & State Plane | Canonical Store、Object Store、Artifact Store、DataHandle、BatchStream、索引与缓存 | UI 会话真值 |
| Provenance & Publication Plane | Run Ledger、MCP Audit、Invocation Trace、Capsule、Publication、ShareChannel | 修改既有 Revision |

Kernel Core 仍位于 Data & State Plane 的 canonical 边界，并通过 Query/Command 合同服务其他平面；它不吸收 Scheduler、Agent loop、MCP session 或云平台生命周期。

## 4. Slot 从扩展端口升级为方法学对象

### 4.1 三个正交维度

现有 `BOUND / UNBOUND / MULTI_PROVIDER / DISABLED_BY_POLICY / INCOMPATIBLE_SCHEMA` 继续作为 Runtime/Capability 投影。研究方法还需要两个独立维度，不能复用同一个状态字段：

| 维度 | 候选状态 | 回答的问题 |
|---|---|---|
| Capability State | 现有 SlotState 枚举 | 当前系统能不能提供该能力 |
| Method Slot Policy | FIXED / REPLACEABLE / OPEN | 方法作者允许别人怎样改变这里 |
| Run Resolution State | RESOLVED / AMBIGUOUS / UNRESOLVED / POLICY_BLOCKED | 这次运行是否已经得到唯一可执行绑定 |

语义建议：

- `FIXED`：方法定义钉住精确 Operator Release；改变它必须产生新 Method Revision 或 Fork。
- `REPLACEABLE`：方法提供默认 Provider，但允许使用者选择兼容实现；每次运行仍封印实际解析结果。
- `OPEN`：作者有意留下方法自由度或未解决问题；在成为可执行 Snapshot 前必须明确绑定、降级或排除该分支。

因此：

- “没有安装 OCR Provider”是 Capability State 的 `UNBOUND`；
- “作者主动让 genre judgement 保持开放”是 Method Slot Policy 的 `OPEN`；
- “本次运行有两个兼容 Provider 未选择”是 Run Resolution 的 `AMBIGUOUS`。

三者不能混为一个 `unbound` 布尔值。

### 4.2 Slot Binding 的解析顺序

一次运行建议按以下优先级解析：

1. PipelineSnapshot 中钉住的 exact release；
2. Project/Method BindingSet；
3. 用户或组织策略允许的默认 Provider；
4. 与 Schema、数据驻留、许可、资源和预算兼容的候选集；
5. 可证明等价的确定性降级路径；
6. 否则保持不可执行，并向 UI 返回缺失原因。

Agent 可以建议第 2 项，但不能覆盖第 1 项或第 3～4 项的强制策略。

### 4.3 SubPipeline 是一等 Operator

SubPipeline 不需要成为第二套执行系统。编译后它应表现为：

- 对外有明确输入/输出 Schema；
- 内部仍是 PipelineDefinition；
- 发布时封印成 PipelineSnapshot；
- 可以实现一个 Slot；
- 内部还可以包含 FIXED、REPLACEABLE 或 OPEN Slot；
- provenance 同时保留外层调用和内层节点。

这样 Pipeline、Operator 和 Slot Provider 可以递归组合，但 Scheduler 最终只面对一个展开或分阶段保留的 LogicalExecutionPlan。

## 5. Pipeline 的四阶段生命线

### 5.1 Authoring：可编辑研究方法

语言学家看到的是 Corpus、Filter、KWIC、Compare、Statistics 等研究步骤。Agent 对画布的修改必须先形成 `PipelinePatchProposal`，包括新增/删除节点、参数变化、Slot Policy 变化、预期数据范围和解释。

用户接受后产生新的 Method Revision；拒绝不会改变方法历史，也不会产生 canonical Revision。

### 5.2 Sealing：生成不可变 PipelineSnapshot

运行、发布或引用前必须封印：

- 精确节点和边；
- 参数与 Schema 版本；
- PromptSnapshot，而不是可变 Prompt 名称；
- Operator Release / MCP tool generation；
- Slot BindingSet；
- 数据快照选择器；
- DataGrant、ToolGrant 和 Policy snapshot hash；
- 随机种子、确定性声明和缓存策略；
- 预算、截止时间与可接受降级策略。

未解析的 Required Slot、未知 Schema、越权数据范围或没有许可证的远程执行都使 sealing 失败，而不是到 Worker 内才猜测。

### 5.3 Planning：逻辑计划与物理计划分离

LogicalExecutionPlan 只描述：

- 数据依赖；
- 操作依赖；
- 输入/输出 Schema；
- 分区、聚合和顺序语义；
- checkpoint 边界；
- 可缓存性、确定性和副作用分类。

PhysicalExecutionPlan 才决定：

- Local / Remote / Either 的实际放置；
- page、Chunk、Partition 或 Shard 的拆分；
- CPU、GPU、内存、网络和并发；
- Function、Container Job、长驻 Worker 或分布式后端；
- 数据本地性、预取、shuffle 和 reduce；
- retry、lease、checkpoint 与 backpressure。

因此同一个 PipelineSnapshot 在笔记本和云集群上具有相同研究语义，但 Physical Plan 可以完全不同。

### 5.4 Running：Task 与 Invocation 分离

Task 是耐久的调度意图；Invocation 是某次尝试。一个 Task 可能经历：

- 第一次 Invocation 冷启动超时；
- 第二次 Invocation 在另一 Worker 重试；
- 第三次从 checkpoint 继续；
- 最终只有一个完整 Artifact 被原子发布。

Run Ledger 必须记录 attempt 和最终选择，不能假设“一个节点只执行一次”。

## 6. Serverless 执行模型

### 6.1 Durable Control Plane + Ephemeral Execution Plane

Serverless 化的不是整个 Jueming Kernel，而是适合弹性化的 Operator Invocation。以下对象必须耐久存在：

- PipelineRun 状态；
- Task DAG、依赖、Lease 与 checkpoint；
- Policy/Grant snapshot；
- Artifact manifest；
- Run Ledger 与审计；
- ApprovalTicket 状态；
- canonical Revision 和 PublicationSnapshot。

以下对象可以短暂存在：

- Worker process/container/microVM；
- 解码器、模型进程和临时内存索引；
- 某个 Task attempt；
- 可重建的局部缓存和临时批次。

Agent session 也不能只存在于一个 Function 内；它的 durable run state 与瞬时模型调用必须分开。

### 6.2 执行后端不是单选题

| Backend Profile | 适合 | 不适合 |
|---|---|---|
| Local Inline | 很短、纯函数、可信内建操作 | 不可信 PDF、Python、重模型 |
| Local Isolated Worker | PDF 解析、OCR、Python、崩溃隔离 | PB 分布式扫描 |
| Serverless Function | 短时、无状态、事件触发、可并行分片 | 长时 GPU、巨型模型、持续会话、大 shuffle |
| Container Job | 长时 CPU、PDF 批处理、可检查点任务 | 极低延迟细粒度调用 |
| Distributed Task Backend | Corpus 级 scan/filter/shuffle/reduce | 小型交互 Query |
| GPU Job / Service | Embedding、训练、批量推理 | 简单文本过滤 |
| MCP Operator Adapter | 受控复用外部能力 | 未声明 Schema、版本、幂等与数据边界的任意 Tool |

Planner 选择 Profile；PipelineDefinition 不写死云厂商函数名、容器 ID 或队列地址。

### 6.3 Invocation 最小合同

一次 Invocation 至少需要稳定表达：

| 字段组 | 内容 |
|---|---|
| Identity | RunId、TaskId、attempt、InvocationId |
| Code | OperatorReleaseId、package/model hash、tool generation |
| Inputs | DataHandle / ArtifactHandle，不内嵌大语料 |
| Outputs | 预期 Schema、Artifact publish policy、Patch 是否允许 |
| Governance | DataGrant、ToolGrant、tenant、actor、secret references |
| Reliability | idempotency key、deadline、retry class、checkpoint policy |
| Placement | locality、residency、CPU/GPU/memory、network policy |
| Accounting | budget、quota、cost center、trace correlation |

Secret 只以可解析引用进入受信运行环境，不进入 PipelineSnapshot、日志、Artifact 或 Publication。

## 7. 分布式可靠性语义

### 7.1 默认接受至少一次派发

网络超时无法证明远端函数究竟“没有执行”还是“执行后响应丢失”。因此：

- Scheduler 到 Worker 按至少一次派发设计；
- Task/Invocation 使用幂等键和 attempt；
- cacheable Operator 以 ArtifactKey/content hash 去重；
- 有外部副作用的 Tool 默认不自动重试；
- MCP Tool 必须显式声明 side-effect 与 idempotency，Host 仍可提高风险等级；
- canonical 写入只在 Kernel 事务内按 `command_id` 去重。

“Exactly once”只承诺到完整 Artifact 发布或 canonical Command 提交边界，不承诺底层函数真的只运行过一次。

### 7.2 Lease、Heartbeat 与 Zombie Result

Task 被 Worker 领取时签发有期限 Lease。Lease 到期可重新调度，但旧 Invocation 可能晚到。Runtime 必须：

- 按 TaskId + attempt + lease generation 验证结果；
- 拒绝过期 Invocation 覆盖新结果；
- 内容相同的 Artifact 可去重复用；
- 内容不同的迟到结果进入诊断或隔离区，不成为 active output；
- cancellation 是协作信号，hard deadline 才是最终边界。

### 7.3 完整结果原子可见

Worker 输出使用两阶段发布语义：

1. 写临时对象或 multipart upload；
2. 校验 size、hash、Schema 和 provenance；
3. 原子提交 ArtifactManifest；
4. 只有 Manifest complete 后下游节点可见；
5. 失败或中断的临时数据由 GC 回收。

这与现有“半 Revision 不可见”保持同一设计哲学，但 Artifact 仍然不是 Revision。

## 8. 数据模型：从 Slice 到 PB BatchStream

### 8.1 不新增互相竞争的 Handle

对话中提出的 `CorpusHandle` 很有价值，但建议把它定义为 `DataHandle` 的专门投影，而不是第二套平行协议：

| Handle | 典型范围 | 用途 |
|---|---|---|
| ProjectDataHandle | Project + Revision + view spec | 编辑、搜索和 Agent 上下文 |
| CorpusHandle | Corpus + Snapshot + schema + partitions | 大语料查询与研究 Pipeline |
| ArtifactHandle | content hash + schema + producer | 下游复用与发布 |
| StreamHandle | plan/run + partition cursor + schema | BatchStream / TrainingStream |

所有 Handle 至少绑定：

- 逻辑对象 ID；
- Revision/Snapshot；
- Schema；
- View/Projection specification；
- DataGrant 与 tenant scope；
- 数据驻留和允许的执行位置；
- expiry/renewal；
- locality hint；
- 内容或 manifest hash。

Handle 是能力引用，不是可猜测的文件路径或永久公开 URL。

### 8.2 语义层与物理层分离

语义层继续使用稳定对象：Corpus、Document、Segment、Alignment、Annotation、Metadata。物理层可以演进为：

- Snapshot；
- Partition；
- Shard / Chunk；
- RecordBatch；
- local dense index；
- mmap / Parquet / Arrow / object blocks；
- distributed shuffle partition。

稳定 SegmentId、AlignmentId 和 DocumentId 不因 repartition、compaction、cache、训练 shuffle 或存储迁移而改变。物理格式不进入 UI、Pipeline 方法语义或 Publication permalink。

### 8.3 LogicalDataPlan 与 Pipeline IR 统一

不建议再创造一套与 Pipeline 平行的 DatasetPlan 语言。Dataset 操作应成为 Pipeline IR 的一组受约束节点：

- Scan / SelectSnapshot；
- Filter / Project；
- LinguisticQuery / KWIC；
- Map / Tokenize / Embed；
- Join / Align；
- Sample / Shuffle / Shard；
- Batch / Window；
- Aggregate / Compare；
- MaterializeArtifact / PublishStream。

研究分析与训练数据准备因此共享 LogicalDataPlan，但 Training Gateway 只消费明确标记的 Stream output，不反向拥有 Corpus 或 Revision。

### 8.4 Control Plane 与 Data Plane

| Plane | 传输内容 | 禁止内容 |
|---|---|---|
| Control Plane | ID、Handle、Schema、Plan、状态、游标、错误、预算 | GB/TB 正文、模型权重 |
| Data Plane | Slice、Batch、对象分片、Artifact、训练批次 | 未授权的完整工程、永久凭据 |

Arrow RecordBatch/Flight、共享内存、对象存储和压缩流都可以是 Data Plane carrier；对外稳定的是 Schema 和 Batch/Stream 语义，而不是某个库的内部类型。

## 9. 本地、云端与 PB 集群的统一

### 9.1 三种 Profile，共享同一逻辑合同

| Profile | Control | Execution | State/Data |
|---|---|---|---|
| Desktop Local | in-process KernelClient + 本地调度器 | inline / isolated worker | `.jm`、本地 Artifact/Cache |
| Remote Single-Node | HTTP/stream transport + Server Kernel | 单机 worker pool | PostgreSQL-like metadata + object store 可选 |
| Distributed Cloud | durable scheduler + queue/event | serverless/container/GPU/distributed workers | metadata service + object storage + distributed index |

统一发生在 PipelineSnapshot、OperatorRelease、DataHandle、Artifact、PatchProposal 和错误语义。物理 Repository、队列、数据库和执行引擎可以不同。

### 9.2 数据本地性优先于“任意函数弹性”

PB 语料最昂贵的往往不是函数计算，而是数据移动。Planner 必须优先考虑：

- 让 Task 靠近 Shard/Artifact；
- 先做 projection/filter，再跨网络；
- 模型靠近数据还是数据靠近模型；
- 对象是否已在目标区域缓存；
- 数据许可是否允许离开设备、组织或地域；
- shuffle 和 reduce 是否值得分布式执行；
- 小型交互 Query 是否应留在本地。

因此 Serverless 只是 Execution Plane 的一种弹性实现，不能主导整个数据架构。

### 9.3 高速训练的边界

训练路径建议是：Research Pipeline 产生已封印的 StreamHandle，Training Gateway 消费 BatchStream。训练系统得到：

- 精确 Corpus Snapshot 和过滤计划；
- tokenizer/model preprocessing release；
- seed、sampling、shuffle 和 batch policy；
- partition ownership 与 checkpoint；
- provenance、license 和 DataGrant。

训练系统不获得 Kernel mutable store，也不把梯度、optimizer state 或训练日志写入 canonical Revision。模型产物作为 Artifact/OperatorRelease 重新注册后，才能成为新的 Slot Provider。

## 10. Agent 与 MCP 在新模型中的位置

### 10.1 Agent 修改方法 Proposal，不直接修改执行计划

Prompt 介入路径建议细化为：

1. ResearchQuestion / 当前画布 / DataView 进入 Context Builder；
2. Agent 生成 IntentDraft；
3. Plan Compiler 生成 PipelinePatchProposal；
4. Schema、Slot、Policy、预算和数据范围校验；
5. UI 展示画布差异与理由；
6. 用户接受后创建 Method Revision；
7. 运行前 Sealing 生成 PipelineSnapshot；
8. Planner 独立选择物理后端。

Agent 可以说“该任务适合远程 GPU”，但这只是 placement hint；Policy 与 Planner 拥有最终决定权。

### 10.2 MCP Tool 有两种身份

| 身份 | 使用方式 | 能否绑定 Slot |
|---|---|---|
| Ad-hoc Tool Step | Agent 经 Tool Gateway 临时调用词典、搜索、外部 API | 否 |
| McpOperatorAdapter | 显式声明 Schema、版本、generation、确定性、幂等、资源和数据边界 | 经注册和审核后可以 |

发现一个 MCP Tool 不能自动生成 Provider。每次 PipelineSnapshot 还必须封印实际 tool generation；Server schema 更新不能悄悄改变旧方法。

### 10.3 Jueming 作为 MCP Server

对外建议暴露：

- revision-bound Resources；
- Research Capsule / Artifact manifests；
- 只读 Query Tools；
- propose_pipeline_patch / propose_canonical_patch；
- get_proposal_status / run_status。

不暴露：

- 数据库和文件路径；
- execute_sql / write_project_file；
- 可自签 ApprovalTicket 的接口；
- 绕过 DataGrant 的完整 PDF/语料下载；
- 直接指定物理 Worker、云账号或 secret 的接口。

## 11. 多模态 PDF 的 Serverless 化

PDF 继续遵守 Immutable SourceAsset + Page Artifact Graph。适合弹性执行的单位是 page range 或 region：

| 阶段 | 适合的 Task 粒度 | 输出 |
|---|---|---|
| Probe | 单文件 | PDF manifest、页数、加密/损坏状态 |
| Render | 页或页区间 | PageRaster Artifact |
| Native Text | 页 | NativeTextSpan Artifact |
| OCR | 页/区域 | OcrRegion Artifact |
| Layout / Reading Order | 页或相邻页窗口 | Layout/Order Artifact |
| Table / Formula / Vision | 区域 | 结构化或多模态 Artifact |
| Segment Builder | 文档区间 | ImportDraft / PatchProposal |

每个 Task 的 ArtifactKey 包含 asset hash、page/region、OperatorRelease、参数和输入 hash。远程处理 PDF 页面必须由 DataGrant 和许可共同允许；OCR 未安装时继续表现为合法 UNBOUND，不因存在云后端就自动上传。

## 12. Research Capsule 与科学交流

### 12.1 Capsule 最小组成

| 部分 | 内容 |
|---|---|
| Finding | 人类可读结论、状态、作者与限定条件 |
| EvidenceSet | 表格、统计量、效应量、代表例、图表 Artifact |
| MethodSnapshot | PipelineSnapshot、Method Revision、Slot Policy |
| BindingProof | 实际 Operator Release、模型、Prompt/MCP generation |
| DataSnapshotRefs | Corpus/Project Revision、选择器、哈希、许可 |
| RunProof | RunId、环境/后端类别、seed、预算、成功/失败摘要 |
| ProvenanceGraph | derived_from、reproduces、replicates、forks、challenges |
| Access & License | 可见性、下载、fork、Agent access、过期和许可 |

### 12.2 三种复现语义

| 模式 | 语义 | 允许变化 |
|---|---|---|
| Exact Replay | 使用封印的 Snapshot、Release、参数、seed 与数据引用 | 仅物理 Worker 可变化 |
| Compatible Reproduction | 允许明确兼容的 Provider/后端，但记录新的解析结果 | 版本范围内变化 |
| Exploratory Fork | 允许修改 Pipeline、Slot、数据或参数 | 产生新的 Method/Capsule，并保留 derived_from |

产品不能把“在另一台机器上成功运行”自动宣称为科学复现。系统只证明执行身份、输入、方法和输出链；研究结论是否成立仍由研究者解释。

### 12.3 与现有分享模型兼容

Research Capsule 作为 PublicationManifest 中的一类根对象：

- PublicationSnapshot 继续不可变；
- ShareChannel 继续只保存可变 head；
- CAS 继续复用相同 Artifact、PipelineSnapshot 和数据对象；
- Fork 创建新的 ProjectId/MethodId，但复用有权限的内容哈希；
- 永久链接固定 Snapshot；稳定链接跟随 Channel；
- 撤销先禁用访问，GC 延后回收无引用对象。

## 13. 面向语言学家的产品模型

### 13.1 渐进展开

| 层级 | 用户看到 | 系统内部 |
|---|---|---|
| Easy | 研究问题、数据、常用研究模板 | 创建初始 PipelineDefinition |
| Canvas | Corpus、KWIC、Filter、Compare、Statistics | Pipeline IR 与 Schema 检查 |
| Method | “固定”“可替换”“待选择” | MethodSlotPolicy + BindingSet |
| Advanced | 输入输出、参数、版本、成本、隐私、执行位置 | PipelineSnapshot/Plan diagnostics |
| Code Projection | Python/Rust/DSL 风格表示 | Pipeline IR 的可逆投影，不是第二份真值 |

普通用户不需要看到 Slot、Schema、Invocation 和 Lease。高级用户可以逐层展开，但所有入口仍编辑同一个 PipelineDefinition。

### 13.2 Agent 的交互语义

Agent 修改画布时必须提供：

- 变更前后节点差异；
- 为什么该步骤能回答研究问题；
- 新增的数据需求和 Slot；
- 是否会发送数据到远端；
- 预计资源、成本和时间等级；
- 是否产生 canonical Patch；
- 接受、部分接受、撤销和查看依据。

不要让用户直接面对“部署函数”“选择容器”或“配置队列”；这些属于 Advanced 的诊断与策略，而不是研究方法本身。

## 14. 账本与关联 ID

至少维持以下分账：

| 账本 | 真值 |
|---|---|
| Canonical Revision / OpLog | 工程领域变更 |
| Method Revision | 研究方法定义变化 |
| Agent Run Ledger | Prompt、模型、Tool、计划与 Proposal |
| MCP Audit | 连接、generation、Resource/Tool 调用与结果状态 |
| Scheduler / Invocation Trace | Task、attempt、lease、worker、retry、checkpoint |
| Publication Audit | 创建、访问、下载、更新、fork、revoke |
| Telemetry | 性能、错误率和容量；可按政策采样/清理 |

它们通过 ProjectId、MethodId、PipelineSnapshotId、RunId、TaskId、InvocationId、ProposalId、RevisionId、ArtifactId 和 PublicationSnapshotId 关联，但不能合并成一张无边界的“万能事件表”。

## 15. 与已冻结 MVP 合同的兼容性

| 已冻结合同 | 本稿处理 | 是否要求当前实现修改 |
|---|---|---|
| Stable opaque ID | 新对象继续使用独立稳定 ID；物理分区不替代语义 ID | 否 |
| Segment 是 canonical relation unit | Pipeline/Corpus 计算不改变 Segment 语义 | 否 |
| Append-only Revision | 所有 Promote/Commit 仍产生完整 Revision | 否 |
| Command 唯一写入口 | Worker/Agent/MCP 只能返回 Artifact/PatchProposal | 否 |
| Chunk/Slice 边界 | 扩展为 Partition/Shard/Batch 仍不成为语义身份 | 否 |
| KernelClient façade | 长期增加独立 Method/Agent/Sync/Publication client，不让 UI 直连存储 | 当前不改 |
| UNBOUND Slot | 继续合法；另增 Method Slot Policy，避免重载状态 | 当前不改 |
| MVP 不加载外部插件 | 本稿只定义未来合同和分期 | 否 |

本稿不能直接修改 Phase 0 的 `RevisionId`、Command、Query、SlotState 或 `.jm` 格式。任何落地都必须先新增 ADR 和合同版本，并证明旧工程 round-trip 与人工对齐主链不受影响。

## 16. 建议冻结的 ADR 候选

按依赖顺序建议评审：

1. Slot、OperatorRelease、Task、Invocation 四类身份分离。
2. Method Revision 与 Canonical Revision 分域。
3. PipelineDefinition → PipelineSnapshot → LogicalPlan → PhysicalPlan 四阶段。
4. MethodSlotPolicy 与 Runtime SlotState 正交。
5. DataHandle 家族与能力引用语义；CorpusHandle 是其专门投影。
6. Worker 至少一次派发、Artifact 原子发布、Kernel Command 幂等提交。
7. Local/Cloud 共享逻辑合同，不要求同一部署 topology 或 binary。
8. Research Capsule、三种复现模式和 Provenance Graph。
9. MCP Tool 成为 Operator 的显式适配与 generation sealing。
10. Training Gateway 只消费 StreamHandle，模型产物以 Artifact/OperatorRelease 回流。

## 17. 推荐验证顺序

### Phase 0：继续完成当前人工对齐 MVP

- 不显示 Agent、Pipeline、Cloud、OCR、Training 假入口；
- 继续验证稳定 ID、Revision、Command、Slice 和 `.jm` 恢复；
- 只在文档中保留未来合同。

### Phase 1：本地方法模型，不引入外部 Runtime

- PipelineDefinition / Method Revision / PipelineSnapshot 的纯合同原型；
- 使用内建、确定性、只读 Query 节点验证画布与 IR 双向；
- 一个 REPLACEABLE Slot 和一个 OPEN Slot 的状态测试；
- 本地 Planner 只生成 Local Physical Plan。

### Phase 2：Artifact 与隔离 Worker

- ArtifactKey、临时结果、完整 Manifest 原子发布；
- 本地隔离 worker、超时、取消、崩溃与重试；
- 先用 PDF page 或纯文本统计证明，不接云端。

### Phase 3：只读 Agent / MCP

- Agent 只提 PipelinePatchProposal；
- MCP Resource 与 ad-hoc Tool Step；
- Run Ledger、MCP Audit、DataGrant；
- 不允许 canonical Patch 自动提交。

### Phase 4：远程回环与单节点 Server

- transport-neutral client；
- 同一 PipelineSnapshot 在 local 与 loopback remote 运行；
- DataHandle、Artifact 和错误 conformance；
- 证明无需同一 binary 仍保持语义一致。

### Phase 5：云端弹性与分享

- durable Task/Lease/Invocation；
- object storage、metadata transaction、CAS；
- PublicationSnapshot、Research Capsule、ShareChannel；
- 再评估 Function、Container Job 和分布式后端。

### Phase 6：PB Corpus 与 Training Gateway

- Corpus Snapshot / Partition / Shard；
- LogicalDataPlan 与 BatchStream；
- data locality、shuffle、checkpoint、GPU feeding；
- exact/compatible/exploratory reproduction 测试。

## 18. 下一轮需要定稿的三张表

在继续扩图或选云厂商之前，优先冻结：

1. **Pipeline / Method 对象表**：Definition、Method Revision、Snapshot、Binding、Run 的字段与身份；
2. **Operator / Invocation 合同表**：Release、Task、Lease、attempt、Artifact publish、retry 与 side-effect；
3. **Research Capsule Manifest 表**：Finding、Evidence、Method、Data、RunProof、Provenance、License、Access。

这三张表确定后，Slot UX、Agent plan proposal、Serverless 调度、PB 数据流和一键分享才会真正共享同一套架构语言。
