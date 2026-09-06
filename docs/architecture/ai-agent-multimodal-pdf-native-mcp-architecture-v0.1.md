# 决明 AI Agent、多模态 PDF 与原生 MCP 架构 v0.1

- 状态：Architecture Candidate，尚未成为 Accepted ADR
- 日期：2026-09-02
- 范围：MVP 之后的本地优先扩展生态
- 实施状态：仅架构规划，不包含代码、依赖选型或生产入口
- 基线：Phase 0 合同 1.0、ADR-009、ADR-010、全局 Slot Architecture v0.2

---

## 0. 结论先行

本轮建议把决明扩展为三个彼此隔离、通过稳定合同协作的运行平面：

1. **Canonical Kernel Plane**
   - 继续唯一拥有 Project、Document、Segment、SegmentOrder、Alignment、HumanAnnotation 和 Revision。
   - 所有正式写入继续经过 Kernel Command、乐观并发校验和 append-only Revision。

2. **Document Capability Plane**
   - 以 Slot、Schema、Operator、Artifact 和 Scheduler 运行 PDF 解析、页面渲染、OCR、版面分析、阅读顺序与分段等可替换数据流水线。
   - 多模态 PDF 是页级派生数据图，不是一个直接吐出 Segment 的不可替换“大插件”。

3. **Agent and MCP Plane**
   - Agent 通过受策略约束的 Tool Gateway 读取 DataView、调用内建或 MCP Tool、生成 Artifact 或 Patch Proposal。
   - MCP 是系统边缘的互操作协议，同时支持“决明作为 MCP Host/Client”和“决明作为 MCP Server”两种角色。
   - MCP Tool 不自动成为 Slot Provider，Slot Operator 也不自动暴露给模型。

核心决策是：

> **Slot 回答数据流水线在哪里允许计算介入；Tool 回答一个受控调用可以做什么；Agent 决定何时组合 Tool；MCP 只负责跨进程或跨产品交换 Tool、Resource、Prompt 和状态。**

这一区分可以保留最开始 Slot 架构的正确部分，同时避免把非确定性 Agent、外部副作用和 Canonical 数据写入混成一个插件接口。

---

## 1. 范围和非目标

### 1.1 本文解决

- 内建或第三方 AI Model Provider 如何接入。
- Agent 如何使用当前选择、当前文档或当前工程的数据。
- 决明如何连接本地 stdio 或远程 Streamable HTTP MCP Server。
- 决明如何向 Codex、Claude、DeepSeek Harness 等 MCP Host 暴露受控能力。
- PDF 如何同时保留原始页面、原生文本、图片、布局、OCR、表格、公式和视觉语义。
- Agent、PDF 与 MCP 结果如何进入 Artifact、Provenance、Patch Proposal 和 Revision。
- 权限、凭据、审计、预算、取消、错误隔离和恢复边界。
- 桌面 App、未来 Headless Host 与远程 Server 如何共享语义合同。

### 1.2 本文不解决

- 不在当前 MVP 中上线 Agent、PDF Parser、OCR 或外部 MCP。
- 不实现自动翻译器、通用桌面自动化平台或云协作平台。
- 不允许模型、MCP Server 或插件直接写 catalog、Chunk、SQLite 或 Revision 文件。
- 不把完整工程默认发送给远程模型。
- 不决定 PDFium、PDF.js、Docling、OCR 引擎或具体模型的最终依赖。
- 不把 MCP 当作 Kernel 内部 ABI，也不让 Kernel Core 依赖某个 MCP SDK。
- 不在本轮接受任何新 ADR；文末只列出需要另行评审的 ADR 候选。

---

## 2. 现有架构基线

### 2.1 必须保留的正确设计

最开始的 Slot 架构已经固定了本轮最重要的基础：

- Segment 是可编辑、可引用、可对齐的 canonical unit。
- SegmentOrder 与 Segment 身份分离。
- Alignment 只引用稳定 SegmentId。
- Canonical、Derived、Index 和 UI State 分离。
- Slot 是稳定介入位置，Operator 是 Provider。
- 未绑定能力以 UNBOUND 存在。
- 插件读取 DataView，输出 Artifact 或 Patch，不获得可变 Corpus。
- Operation RPC 与 Data RPC 分离。
- DataHandle 绑定 Project、Revision、Schema 和 View specification。
- 大数据按 Chunk、Slice、Batch 或 Artifact 传递，控制面只传 ID、Handle 和状态。
- UI 只通过 typed KernelClient 进入 Kernel。

Phase 0 又进一步冻结：

- Command 是唯一 canonical 写入入口。
- 每次成功写入产生完整 Revision。
- stale base revision 不能静默覆盖。
- HumanAnnotation 是 sidecar。
- Tauri MVP 没有外部插件运行时。

因此，新增 Agent、PDF 和 MCP 不需要重写 Core 数据模型；真正需要补齐的是扩展平面和安全合同。

### 2.2 当前文档存在的命名分叉

仓库内至少存在两套 Slot 命名 Profile：

| 能力 | 功能文档示例 | 全局 Handoff 示例 |
|---|---|---|
| PDF | source.pdf | 未单独冻结；PDF 主要进入 source.ocr |
| OCR | source.ocr | source.ocr |
| 规则分句 | segment.rule | content.segment.sentence |
| POS | annotation.pos | token.pos |
| 手工对齐 | alignment.manual | relation.alignment |
| 基础搜索 | analysis.basic_search | index.lexical / analysis.kwic |

Phase 0 把“稳定 Slot 名称和 UNBOUND 状态”设为合同，但没有给出唯一的 namespace catalog。外部 MCP、插件 Manifest 或 Agent Tool 一旦引用这些名字，分叉就会变成长期兼容负担。

所以本架构提出一个前置门：

> **在任何外部 Runtime 上线前，先通过新 ADR 冻结 Slot Namespace 2.0，并为既有 1.0 名称提供明确 alias/deprecation 表。**

本文中的新 Slot 名称只是候选，不等于已接受合同。

### 2.3 现有架构尚缺的对象

最开始的 OperatorDescriptor 适合批处理或数据转换，但不足以表达以下语义：

- 一个工具是否只读、幂等、可重试或具有外部副作用。
- 一个模型调用的预算、数据授权、上下文范围和远程保留政策。
- MCP 连接的协议版本、传输、认证、会话和动态 tool generation。
- Agent run、turn、tool call、approval 和 usage 的审计链。
- PDF 页、页面坐标、原生文本、OCR、布局和视觉描述之间的 provenance。
- 用户批准一个 Patch Proposal 后如何发放不可伪造的一次性提交权。

这些对象应位于 Agent/MCP 或 Document Capability Plane，而不是塞进 Segment 或 SlotDescriptor。

---

## 3. 开源架构参考与取舍

### 3.1 Motrix Turbo

Motrix Turbo 当前架构的优点不在 Electron 本身，而在边界纪律：

- 一个 host-neutral product core 同时服务 Electron Desktop 和 Node/Web Server。
- Renderer 只使用统一 command/query/event transport。
- EngineAdapter 隔离 aria2，EngineSupervisor 独占引擎生命周期。
- Wire schema、method、error code 有单一协议真值。
- 插件运行在 QuickJS 沙箱，无 Node API 和默认文件/网络权限。
- Plugin candidate、active worker、permission generation 和 invocation 明确分离。
- 同一插件 VM 使用 FIFO lane，跨插件可并行，重入环在入队前失败。
- DTO 在 Host→Worker 与 Worker→Host 两个信任边界都验证并限制大小。
- 文件提交先形成不可变计划，再使用持久 journal、无覆盖安装、补偿和启动恢复。
- Post-hook 使用稳定 delivery id 和 at-least-once 语义，不伪装成任意外部副作用 exactly-once。
- Electron 和 Server 按同一启动顺序装配相同 Runtime。

决明应吸收：

- Kernel Core 与 Desktop/Headless Shell 分离。
- MCP/Agent 的单一 Tool 和 wire schema 真值。
- 每个 MCP Server 或 Agent Provider 独立 lane、generation 和取消域。
- 先验证 Proposal，再由 Kernel Commit。
- 长任务、外部调用和异步通知必须有稳定 ID、幂等边界和恢复策略。
- 权限变更后旧 capability lease 立即失效。

决明不应照搬：

- 不把 QuickJS 作为默认 MCP 或模型运行时。
- 不把 Agent 变成通用插件生命周期 Hook。
- 不把外部工具副作用写进 canonical Revision。
- 不让插件市场或自动更新提前进入 MVP。

### 3.2 dsh-pet 与 DeepSeek Harness

dsh-pet 的架构价值在于小而清晰：

- HarnessBridge 是唯一知道宿主原始事件名的层。
- 原始事件先归一化为 NormalizedEvent。
- 纯 PetStateResolver、PetStateMachine 和 TaskStateRegistry 不依赖 Harness、窗口或网络。
- Win32 与 X11 被 WindowBackend 隔离。
- 可选能力通过运行时探测，缺失时降级而不是阻止插件加载。
- 注册行为使用可逆 effect，卸载时统一清理监听、命令、路由、状态机和窗口。
- 并发 Agent 状态用确定性优先级折叠，桌宠本身不消耗模型 Token。

DeepSeek Harness 更进一步，把模型、Tool、Agent loop、session log 和 UI 都作为可组合插件，并用 durable session events 与 live extension events 分开。

决明应吸收：

- 所有 Provider/MCP 原始事件先进入唯一适配层，再变为稳定 AgentActivityEvent。
- Agent 状态投影与桌宠、通知、状态栏解耦。
- 可选 Agent/MCP 能力失效时，人工对齐与 PDF 本地阅读仍正常工作。
- Runtime 的注册和卸载必须可逆，不能遗留 listener、child process 或 credential lease。

决明不应照搬：

- Kernel canonical invariant 不能变成可替换插件。
- Revision、Segment 和 Alignment 校验不能由 Agent loop 或 Cordis 风格事件覆盖。
- “Everything is a plugin”不适合拥有强数据不变量的语言数据 Kernel。

### 3.3 Docling、PDF.js 与 PDFium

Docling 的主要启发：

- 格式 Backend 与 Pipeline 分离。
- Pipeline 由 preprocessing、OCR、layout、postprocess、table、assemble 等阶段组成。
- 每次 run 隔离、队列有界、显式 back-pressure 和 deterministic shutdown。
- PDF 最终先形成统一 Document 表示，再导出或 chunk。

PDF.js 的主要启发：

- Core parsing、Display API、Viewer UI 三层分离。
- 解析在 Worker 中运行，UI 不直接碰 PDF 内部对象。
- 文本层、标注层和渲染层彼此分开。

PDFium 的主要启发：

- Parsing、decoding、logical page representation、rendering 和 rasterization 是不同层。
- Embedding 只依赖 public API；内部 C++ 结构不是稳定 ABI。
- 页面坐标变换必须作为正式边界处理。

决明应吸收：

- PdfBackend 只是可替换解析/渲染适配器。
- 页级、阶段化、有界队列和可取消。
- 原生文本、OCR、布局、阅读顺序与视觉语义是并列或依赖 Artifact，不互相覆盖。
- UI 只读取 PageView、Raster/Tile 和 RegionGraph，不接触第三方引擎对象。

决明不应照搬：

- 不把某个库的内部 Document 类型作为项目格式。
- 不把 Markdown 当成 PDF 的唯一真值。
- 不假设 OCR 返回顺序就是人类阅读顺序。
- 不在依赖许可、跨平台打包和恶意文件隔离验证前锁定具体 PDF 引擎。

---

## 4. 目标架构原则

### P1：MCP 是协议边缘，不是 Data Kernel

MCP Adapter 可以调用 KernelClient、Tool Gateway 和 Resource Gateway；Kernel Core 不解析 MCP 消息，不维护 MCP session，也不依赖 MCP SDK。

### P2：Slot、Tool、Agent、MCP 不合并

| 概念 | 回答的问题 | 是否模型可见 | 是否可写 canonical |
|---|---|---:|---:|
| Slot | 数据图哪里允许 Provider 介入 | 否 | 否 |
| Operator | 谁计算某个 Slot 输出 | 否 | 只能输出 Artifact/Patch Proposal |
| Tool | 一个受控调用能做什么 | 可选 | 只能经 Tool Gateway |
| Agent | 如何选择上下文和组合 Tool | 是 | 不能直接写 |
| MCP Adapter | 如何跨产品交换 Resource/Prompt/Tool | 是 | 服从同一 Tool Gateway |

### P3：同一语义核心，多种 Shell

Desktop Tauri、未来 Headless MCP Sidecar 和 Server Kernel 共享：

- Kernel contracts
- Resource URI semantics
- Tool schemas
- Policy decisions
- Patch validation
- Agent activity event semantics

不同 Shell 只提供：

- 窗口和用户交互
- 本地 IPC、stdio 或 HTTP transport
- OS keychain
- 文件选择和平台通知

### P4：原始 PDF 不可变，理解结果可重建

PDF 原始字节作为 SourceAsset 保存或安全引用。解析、渲染、OCR、布局、表格、公式和视觉描述全部是带 provenance 的 Artifact。用户确认导入后，才用 ChangeSet 创建 canonical Segment。

### P5：模型只能提出 Patch

Agent、MCP Tool 和非内建 Operator 永远不能直接 Commit。Proposal 必须：

1. 指定 base revision。
2. 使用 stable SegmentId/AlignmentId。
3. 通过 schema 和 invariant validation。
4. 展示数据来源、变更范围和置信度。
5. 由用户批准。
6. 由 Kernel Command 提交并生成新 Revision。

### P6：本地优先也是故障边界

没有 Model Provider、MCP Server、OCR 或 GPU 时：

- TXT 人工对齐不受影响。
- 文本型 PDF 仍可本地解析和阅读。
- 扫描 PDF 可显示页面，但清楚标记 OCR UNBOUND。
- Agent 和远程入口保持隐藏或显式不可用，不出现假按钮。

---

## 5. 逻辑架构

~~~mermaid
flowchart TB
    subgraph Presentation["Presentation"]
        Workspace["Parallel Workspace"]
        PdfView["PDF Page View"]
        AgentUI["Agent Console / Approval UI"]
        Status["Agent Status / Optional Pet"]
    end

    subgraph Shells["Application Shells"]
        Desktop["Tauri Desktop Shell"]
        Headless["Headless MCP Sidecar"]
        Server["Future Server Shell"]
    end

    subgraph Edge["Agent and MCP Plane"]
        AgentRuntime["Agent Session Runtime"]
        ModelGateway["Model Provider Gateway"]
        ContextBuilder["Context Builder"]
        ToolGateway["Tool Gateway"]
        ResourceGateway["Resource Gateway"]
        McpClient["MCP Client Manager"]
        McpServer["MCP Server Adapter"]
        Policy["Policy / Consent / Budget"]
        RunLedger["Run Ledger"]
    end

    subgraph Capability["Document Capability Plane"]
        SlotRegistry["Slot Registry"]
        SchemaRegistry["Schema Registry"]
        OperatorRegistry["Operator Registry"]
        Scheduler["Operation Scheduler"]
        PdfRuntime["PDF Document Runtime"]
        ArtifactStore["Artifact / Provenance Store"]
    end

    subgraph Kernel["Canonical Kernel Plane"]
        KernelClient["Typed KernelClient Contract"]
        Command["Command + Validation"]
        Query["Query + DataView"]
        Revision["Revision / Event"]
        Canonical["Canonical Store"]
    end

    Workspace --> Desktop
    PdfView --> Desktop
    AgentUI --> Desktop
    Status --> Desktop

    Desktop --> KernelClient
    Headless --> KernelClient
    Server --> KernelClient

    Desktop --> AgentRuntime
    Headless --> McpServer
    Server --> McpServer

    AgentRuntime --> ContextBuilder
    AgentRuntime --> ModelGateway
    AgentRuntime --> ToolGateway
    AgentRuntime --> Policy
    AgentRuntime --> RunLedger
    McpClient <--> ToolGateway
    McpServer --> ToolGateway
    McpServer --> ResourceGateway

    ContextBuilder --> ResourceGateway
    ResourceGateway --> Query
    ToolGateway --> Query
    ToolGateway --> Command
    ToolGateway --> Scheduler

    Scheduler --> SlotRegistry
    Scheduler --> OperatorRegistry
    OperatorRegistry --> SchemaRegistry
    Scheduler --> PdfRuntime
    PdfRuntime --> ArtifactStore

    KernelClient --> Command
    KernelClient --> Query
    Command --> Canonical
    Command --> Revision
    Query --> Canonical
    Revision --> ArtifactStore
~~~

关键约束：

- ToolGateway 是所有内建 Tool、MCP Tool 和 Agent Action 的唯一调用门。
- ResourceGateway 只提供版本化只读投影。
- Scheduler 不能绕过 Command 提交 canonical 结果。
- MCP Client Manager 不直接注册 Slot Provider。
- MCP Server Adapter 不直接暴露数据库、文件路径或 Rust 类型。

---

## 6. Registry 与描述对象

### 6.1 继续保留的 Registry

| Registry | 责任 |
|---|---|
| Schema Registry | 跨 Runtime 的逻辑 DTO 和兼容范围 |
| Slot Registry | 稳定数据介入点和绑定状态 |
| Operator Registry | Slot Provider、资源需求、确定性、缓存和执行位置 |

### 6.2 新增 Registry

| Registry | 责任 | 不承担 |
|---|---|---|
| Tool Registry | 内建、Operator facade、MCP Tool 的统一调用描述 | 不决定模型何时调用 |
| Model Provider Registry | 模型能力、执行位置、上下文、多模态、成本和健康状态 | 不保存 API key 明文 |
| MCP Connection Registry | Server profile、transport、protocol、capability、tool generation | 不保存完整 session transcript |
| Agent Profile Registry | 模型路由、prompt、允许工具、数据范围、预算、写策略 | 不拥有 canonical data |
| Prompt Registry | 用户可选择的 Agent/MCP prompt 模板 | 不作为隐藏 system policy |
| Resource Template Registry | 稳定 URI template 到 DataView 的映射 | 不枚举全部 Segment |

### 6.3 ToolDescriptor 最小语义

ToolDescriptor 至少需要：

- 稳定 tool id、版本、来源和显示名。
- input/output JSON Schema。
- side-effect class：read_only、proposal_only、canonical_commit、external_mutation。
- idempotency class：safe、idempotent_with_key、non_idempotent。
- required DataGrant、network grant、filesystem grant。
- 是否需要逐次确认。
- timeout、最大输出、可取消、是否允许并行。
- provenance 和 audit policy。
- 当前 generation 和 health state。

MCP 提供的 annotations 只能作为不可信提示，不能覆盖 Host 的 side-effect 和权限分类。

### 6.4 AgentProfile 最小语义

AgentProfile 需要固定：

- model provider 和可接受 fallback。
- prompt 版本与语言。
- tool allowlist，而不是默认加载所有已连接 MCP Tool。
- 默认读取范围：当前选择、当前文档或当前工程。
- 是否允许读 HumanAnnotation。
- 是否允许页面图片、原始 PDF 或仅结构化文本。
- 本地/远程执行位置。
- Token、金额、时间和 tool-call 预算。
- Proposal policy。
- 运行日志保留和脱敏策略。

每次 AgentRun 保存 Profile 快照 hash；后续修改 Profile 不改变旧运行的解释。

---

## 7. 原生 MCP：双向角色

### 7.1 决明作为 MCP Host/Client

用途：

- 内建 Agent 使用外部词典、知识库、搜索、术语库或研究工具。
- 用户显式连接本地 stdio 或远程 Streamable HTTP Server。

结构：

~~~mermaid
flowchart LR
    Agent["Jueming Agent"] --> Gateway["Tool Gateway"]
    Gateway --> Native["Native Jueming Tools"]
    Gateway --> ClientA["MCP Client A"]
    Gateway --> ClientB["MCP Client B"]
    ClientA --> Local["Local stdio Server"]
    ClientB --> Remote["Remote Streamable HTTP Server"]
~~~

规则：

- 一个 MCP Client session 只连接一个 Server。
- 每个 Server 有独立 connection、permission、credential、tool generation 和 failure domain。
- Tool 公共名称使用稳定 server namespace；冲突时由 server id 与 raw tool name 的确定性 hash 消歧。
- list_changed 后先完整拉取、校验，再原子替换 generation；不能出现半套新 Tool。
- Tool schema 变化会影响模型上下文和缓存，所以 AgentRun 固定所用 generation。
- Server 不可用只移除或降级该 Server 的 Tool，不使 Kernel 或其他 Server 失效。
- 非幂等外部写 Tool 不自动重试；断线后进入 unknown_outcome，需要用户核对。

### 7.2 决明作为 MCP Server

用途：

- 外部 Agent 阅读工程、查询对齐、检查 PDF 页面和提出修改建议。
- Headless 工作流在不复制领域逻辑的情况下使用同一 Kernel Contract。

建议形态：

1. **本地第一阶段：stdio Sidecar**
   - 外部 Host 启动 jueming-mcp sidecar。
   - Sidecar 不包含第二套领域实现，只是 MCP Server Adapter。
   - 若 Desktop 已持有工程写锁，Sidecar 通过认证的本地 IPC 连接该 Kernel Host。
   - 若 Desktop 未运行，Sidecar 可启动 Headless Kernel Host 并独占工程锁。

2. **后续阶段：Streamable HTTP**
   - 仅在 Server Kernel、身份、OAuth、审计和网络策略完成后开放。
   - 本地 HTTP 默认只绑定 127.0.0.1，校验 Origin，不监听 0.0.0.0。

Headless Sidecar 没有可信交互 UI 时，只发布 read/proposal 能力；commit_approved_proposal 必须保持不可用，不能用命令行配置预先授予永久提交权。

单写者规则：

> 同一个 .jm 工程任意时刻只有一个 Kernel Host 拥有 canonical 写锁。Desktop 与 Sidecar 不得分别打开后再靠文件级合并解决冲突。

### 7.3 MCP 生命周期映射

MCP protocol lifecycle 保持原生语义：

~~~mermaid
stateDiagram-v2
    [*] --> Configured
    Configured --> Connecting
    Connecting --> Negotiating
    Negotiating --> Ready
    Ready --> Degraded
    Degraded --> Connecting
    Ready --> Stopping
    Degraded --> Stopping
    Connecting --> Failed
    Negotiating --> Failed
    Failed --> Connecting
    Stopping --> Stopped
~~~

这些是 MCP connection state，不能复用 Slot 的 BOUND/UNBOUND。

MCP initialize 期间固定：

- protocol version。
- client/server capabilities。
- resources、prompts、tools、logging、roots、sampling、elicitation、tasks 等支持面。
- implementation identity。

第一阶段建议只承诺：

- Client 侧：tools、resources 基础消费，stdio。
- Server 侧：resources、resource templates、prompts、tools、logging、progress、cancel。
- 不声明尚未完整实现的 sampling、elicitation 或 task-augmented capability。

### 7.4 MCP Resources

决明使用自有 URI，不暴露 .jm 绝对路径：

| Resource template | 内容 |
|---|---|
| jueming://project/{project_id} | 工程摘要和 current revision |
| jueming://project/{project_id}/revision/{revision_id}/segment/{segment_id} | 版本固定的 SegmentView |
| jueming://project/{project_id}/revision/{revision_id}/alignment/{alignment_id} | AlignmentView |
| jueming://project/{project_id}/revision/{revision_id}/parallel-slice{?anchor,halo} | 小范围双语 Slice |
| jueming://project/{project_id}/asset/{asset_id}/pdf/manifest | PDF manifest |
| jueming://project/{project_id}/asset/{asset_id}/pdf/page/{page_id}/structure | 页结构和 RegionGraph |
| jueming://project/{project_id}/asset/{asset_id}/pdf/page/{page_id}/image{?scale,region} | 页或区域 Raster |
| jueming://project/{project_id}/artifact/{artifact_id} | 有权限的 Artifact manifest |
| jueming://project/{project_id}/proposal/{proposal_id} | Patch Proposal 预览 |

约束：

- 使用 resource template，不能 list 出亿级 Segment。
- 所有 canonical resource 带 revision。
- 二进制 PDF 可以作为 application/pdf blob resource 提供，但不能假设所有 MCP Client 或模型能直接理解。
- 跨产品的默认多模态表示应是 Page image/resource link + structured page data。
- 大资源返回 ResourceLink 或分页 Handle，不在一次 tools/call 中塞入完整工程。

### 7.5 MCP Prompts

Prompt 是用户控制的显式入口，可候选提供：

- review_alignment
- inspect_pdf_page
- compare_native_text_and_ocr
- propose_ocr_correction
- propose_segment_boundaries
- explain_alignment_issue

Prompt 模板不是安全策略。不可被用户模板覆盖的规则继续在 Host Policy 和 Tool Gateway 中执行。

### 7.6 MCP Tools

第一批 Tool 应按风险分层：

| 层级 | 示例 | 默认策略 |
|---|---|---|
| 只读查询 | get_project_summary、get_segment、search_segments、load_parallel_slice | 工程授权后可调用 |
| PDF 读取 | get_pdf_manifest、inspect_pdf_page、render_pdf_region | 受页范围和图像授权约束 |
| 计算 | run_pdf_stage、validate_alignment、compare_layers | 需要预算与 OperationId |
| 建议 | propose_segment_patch、propose_alignment_patch、propose_ocr_correction | 生成 Proposal，不写 canonical |
| 提交 | commit_approved_proposal | 默认不暴露；仅持一次性 ApprovalTicket |
| 外部副作用 | upload、publish、remote write | 第一阶段禁止 |

提交工具即使存在，也必须同时校验：

- proposal hash。
- base revision。
- one-time ApprovalTicket。
- ticket 与 proposal、工程、用户会话和过期时间绑定。
- ticket 由 UI 审批流程签发，模型或 MCP Server 不能自签。

### 7.7 Roots、Sampling、Elicitation 与 Tasks

**Roots**

- 默认不把 .jm 工程目录作为 filesystem root 暴露给外部 MCP Server。
- 决明工程应通过 Resource Gateway 读取，避免绕过 Kernel。
- 只有用户为通用文件工具单独选择的目录才进入 roots。

**Sampling**

- 第一阶段不允许外部 MCP Server 请求决明模型采样。
- 以后如支持，必须逐次经过模型、Prompt、数据、费用和 Tool 审批。
- Server 不获得完整对话，也不能默认 include all servers context。

**Elicitation**

- 第一阶段不声明。
- 以后 form mode 禁止请求密码、API key、access token 或支付信息。
- 敏感授权只允许 URL mode，并在 UI 显示目标域名。

**Tasks**

- MCP task-augmented request 不直接等同于 Kernel Operation。
- 第一阶段用内部 OperationId、progress 和 cancel 完成稳定闭环。
- 等任务协议的兼容、恢复和权限语义验证后，再由 Adapter 映射。

---

## 8. Agent Runtime

### 8.1 Agent 不是 Kernel Provider

Agent Runtime 位于 Kernel 之外，职责是：

- 接受用户目标。
- 选择有限 DataView。
- 组装文本、图片和结构化上下文。
- 调用 Model Provider。
- 通过 Tool Gateway 使用内建/MCP Tool。
- 产生 Answer、Artifact 或 Patch Proposal。
- 记录运行、预算、授权和错误。

它不承担：

- Segment/Alignment invariant。
- Revision 提交。
- PDF 解析引擎生命周期。
- MCP Server 的信任判断。

### 8.2 AgentRun 状态机

~~~mermaid
stateDiagram-v2
    [*] --> Draft
    Draft --> AwaitingConsent
    AwaitingConsent --> Running
    AwaitingConsent --> Cancelled
    Running --> AwaitingToolApproval
    Running --> AwaitingUser
    AwaitingToolApproval --> Running
    AwaitingToolApproval --> Cancelled
    AwaitingUser --> Running
    Running --> Completed
    Running --> Failed
    Running --> Cancelled
~~~

运行结束不自动产生 Revision。只有用户批准的 Proposal 进入 Command。

### 8.3 Context Builder

Context Builder 默认遵守最小范围：

1. 当前选择。
2. 必要的相邻 Segment 或页面 halo。
3. 用户允许的 Annotation 或 Artifact。
4. 明确的语言、Revision 和来源。

禁止默认：

- 扫描全工程。
- 读取所有 HumanAnnotation。
- 发送完整 PDF。
- 把本地路径、凭据、日志或其他 MCP Server 内容拼进 Prompt。

上下文对象使用 ResourceLink/DataHandle；只有 Provider Adapter 在调用前物化必要内容。

### 8.4 Model Provider 能力协商

ProviderDescriptor 至少描述：

- text、image、audio、native_pdf 输入。
- tool call 和 structured output 支持。
- 最大上下文、最大图片数量、分辨率和文件大小。
- local 或 remote。
- 是否保留输入、训练使用和地域策略。
- 价格、速率限制和健康状态。
- 取消、流式输出和重试语义。

路由规则：

- 默认优先发送页面区域图片和结构化文本，而不是完整 PDF。
- native_pdf 只在用户显式批准完整资产范围时使用。
- 不支持图像的模型退化为结构化文本路径。
- Provider health 变化不改变已开始 Run 的审计身份。

### 8.5 Run Ledger

Run Ledger 记录：

- run、turn、tool call、approval、model route、usage 和终态。
- tool generation、AgentProfile hash 和 DataGrant。
- 发送的数据类别与范围，不默认保存全部正文副本。
- Proposal 与 Artifact 引用。

它不是 canonical Revision：

- 默认保存在设备级安全运行记录中。
- 已提交 Proposal 在 Revision 中只留下必要 provenance 摘要。
- 若未来允许把完整 Run 附加到工程，应使用独立 Artifact/Run 目录和 format minor upgrade，不能塞进 revisions 正文。

---

## 9. 多模态 PDF 数据架构

### 9.1 PDF 不是一种文本导入格式

PDF 同时包含：

- 原始对象和页面树。
- 字体、字形、原生文本位置。
- 位图、矢量图形、颜色和透明度。
- CropBox、MediaBox、旋转和坐标变换。
- 注释、链接、表单、书签和结构树。
- 扫描页面。
- 表格、公式、图表、图片和复杂阅读顺序。

如果 source.pdf 直接输出 Segment，以下信息会被不可逆丢失：

- 文本来自原生对象还是 OCR。
- 页面和 bbox。
- 多栏关系与阅读顺序。
- 表格单元格和图片语义。
- 对某一页重新 OCR 或更换 Layout Provider 的能力。

因此 PDF 必须先形成 Page Artifact Graph。

### 9.2 页级流水线

~~~mermaid
flowchart TB
    Asset["Immutable PDF Asset"]
    Probe["PDF Probe / Manifest"]
    Primitive["Native Page Primitive"]
    Raster["Page Raster / Tiles"]
    OCR["OCR Layer"]
    Layout["Layout Region Graph"]
    Order["Reading Order Graph"]
    Table["Table Structure"]
    Formula["Formula Layer"]
    Vision["Figure / Visual Semantic Layer"]
    Structure["Document Structure"]
    Draft["Import Draft"]
    Proposal["Segment Creation Proposal"]
    Canonical["Canonical Segments"]

    Asset --> Probe
    Probe --> Primitive
    Probe --> Raster
    Primitive --> Layout
    Raster --> OCR
    Raster --> Layout
    Layout --> Order
    Layout --> Table
    Layout --> Formula
    Layout --> Vision
    Primitive --> Structure
    OCR --> Structure
    Order --> Structure
    Table --> Structure
    Formula --> Structure
    Vision --> Structure
    Structure --> Draft
    Draft --> Proposal
    Proposal --> Canonical
~~~

文本型 PDF 可以跳过 OCR；扫描 PDF 在 source.ocr 为 UNBOUND 时停在 Raster，并显示可行动的能力说明。

### 9.3 PDF Artifact 模型

| Artifact | 最小内容 | 可重建 |
|---|---|---:|
| PdfDocumentManifest | asset hash、页数、加密状态、页面尺寸/旋转、sanitized metadata、backend identity | 是 |
| PdfPagePrimitive | text span/glyph、image ref、link、annotation、structure hint、坐标 | 是 |
| PageRaster | page/region、scale、color、transform、content hash 或 tile manifest | 是 |
| OcrLayer | word/line、bbox、confidence、language、provider | 是 |
| LayoutRegionGraph | region、type、bbox/polygon、adjacency、confidence | 是 |
| ReadingOrderGraph | directed edges、分栏/跨页关系、provider | 是 |
| TableStructure | table/cell graph、row/column span、bbox | 是 |
| FormulaLayer | source region、LaTeX/MathML candidate、confidence | 是 |
| VisualSemanticLayer | figure/chart description、labels、source crop、confidence | 是 |
| DocumentStructure | heading/paragraph/list/table/figure hierarchy | 是 |
| ImportDraft | 候选 Segment、来源 region、顺序和差异 | 是 |

这些 Artifact 不获得 SegmentId。只有用户确认 Promote 后，Kernel 才分配稳定 SegmentId。

### 9.4 页面身份与坐标

候选合同：

- PageId 由 AssetId 与不可变 page index 派生或稳定映射。
- page_number 仅用于 1-based 用户显示，不作为引用身份。
- 跨边界使用 PageSpace/1：
  - 原点为应用旋转后的可见 CropBox 左上角。
  - 单位为 PDF point。
  - 每页携带 width、height 和从 PDF user space 到 PageSpace 的 affine transform。
- ML 可额外使用 0–1 normalized bbox，但它不是唯一真值。
- UI、OCR、Layout 和截图工具必须通过同一 transform 做 round-trip 测试。

这样可以避免 PDFium、PDF.js 或其他 Backend 的坐标约定泄漏到 Schema。

### 9.5 PDF Backend 边界

PdfBackend 只提供：

- probe document。
- random/sequential page access capability。
- native text/primitive extraction。
- page/region render。
- password/encryption status。
- cancellation 和资源统计。

它不提供：

- canonical Segment 创建。
- OCR、Layout 或 Reading Order 的唯一实现。
- UI 组件。
- Model Provider。

Backend 生命周期由 PdfRuntime Supervisor 独占，类似 Motrix 的 EngineSupervisor。UI 或 Agent 不能直接持有第三方 document pointer。

### 9.6 Worker 隔离与资源限制

PDF 是不可信二进制输入。解析/渲染建议运行在独立 worker process，而不仅是 UI thread：

- worker crash 不终止 Kernel。
- 限制页数、文件大小、对象/stream 解压、像素、字体、内存、CPU 和 wall time。
- 默认不执行 PDF JavaScript、Launch action、外部网络内容或嵌入式脚本。
- 外部链接仅作为 sanitized metadata 返回。
- 密码只驻留会话安全内存，不写日志或普通项目 metadata。
- 每页输出通过 schema 和大小限制验证。
- Artifact 完成后按 content hash 原子发布，半页结果不可见。

### 9.7 并发、背压和缓存

- 调度单位是 page range 或 region，不是整个 PDF。
- Pipeline stage 使用有界队列。
- 视口页和用户选择优先于后台全书解析。
- OCR、Layout、Vision 可按页并行，但同一 Provider 遵守资源并发限制。
- PageRaster 按 scale 和 region 缓存。
- ArtifactKey 包含 asset hash、page、operator version、params 和 input hash。
- 用户修改 canonical Segment 不使原始 PDF Artifact 失效；重新解析 SourceAsset 也不能覆盖人工编辑。

### 9.8 Import Draft 到 Canonical 的提升

流程：

1. 导入不可变 PDF Asset。
2. Pipeline 生成 ImportDraft。
3. 用户预览页面、文本来源、顺序和低置信区域。
4. 用户选择 Promote 范围和分段规则。
5. Runtime 生成 create Segment/Order 的 Patch Proposal。
6. Kernel 校验后以一个原子 ChangeSet 创建 Segment。
7. Revision 保存 Asset 与 source anchor 的 provenance。

重新 OCR 或更换 Layout Provider：

- 产生新的 Draft/Proposal。
- 对已经人工编辑的 Segment 做差异预览。
- 不自动覆盖 canonical content。

### 9.9 多模态 Agent Bundle

MultimodalPageBundle 只为一次 Run 组装，不成为第二份 canonical 文档：

- 页面或区域 Raster。
- native text 与 OCR 差异。
- Layout/ReadingOrder/Table/Formula/Visual layers。
- PageId、bbox、language、ArtifactId 和 confidence。
- 前后页或相邻 region 的有限 halo。

模型输出必须锚定：

- PageId。
- source region 或 SegmentId。
- input Artifact hash。
- base revision。

这样模型结论可以被审阅、复算和判定过期。

---

## 10. 候选 Slot Catalog

以下只是 Namespace 2.0 讨论稿：

| Stage | Slot candidate | 输入 | 输出 |
|---|---|---|---|
| L0→L1 | source.pdf.inspect | PdfAsset | PdfDocumentManifest |
| L0→L1 | source.pdf.page.extract | PdfPageHandle | PdfPagePrimitive |
| L0→L1 | source.pdf.page.render | PdfPageHandle | PageRaster |
| L1→L1 | source.ocr | PageRaster / ImageRegion | OcrLayer |
| L1→L1 | layout.region.detect | Primitive + Raster | LayoutRegionGraph |
| L1→L1 | layout.reading_order | LayoutRegionGraph + text | ReadingOrderGraph |
| L1→L1 | structure.table | Region + Primitive/Raster | TableStructure |
| L1→L1 | structure.formula | Region + Raster | FormulaLayer |
| L1→L1 | vision.figure.describe | FigureRegion + Raster | VisualSemanticLayer |
| L1→L2 | content.structure.infer | Page layers | DocumentStructure |
| L1→L2 | content.segment.sentence | DocumentStructure/TextContent | SegmentDraft |
| L2→L5 | relation.alignment.suggest | Segment + optional Layers | AlignmentProposal |

规则：

- source.pdf 既有名称暂作为 legacy capability alias，不在本文直接删除或重定义。
- source.ocr 保留原语义，但输入收窄到明确的 Image/PageRaster Schema；PDF 解码不再隐含在 OCR Provider 内。
- layout.reading_order 保持独立，避免 OCR 与阅读顺序耦合。
- vision.* 只产生派生语义，不写正文。
- Tool 名称独立，例如 inspect_pdf_page，不复用 Slot 名称。

---

## 11. 权限与安全模型

### 11.1 DataGrant

每次 AgentRun 或 MCP connection 都有显式 DataGrant：

- 工程和 revision。
- current_selection、document、project 或指定 page range。
- 可读对象种类。
- 是否可读 HumanAnnotation。
- 是否可发送 Raster、native PDF 或正文到远程。
- 过期时间和撤销 generation。

远程完整工程、完整 PDF 或 HumanAnnotation 默认关闭。

### 11.2 ToolGrant

ToolGrant 独立于 DataGrant：

- read-only tool。
- compute tool。
- proposal tool。
- external network tool。
- canonical commit tool。

读取权限不能隐式升级为写入权限。

### 11.3 本地 stdio MCP 的真实风险

配置 stdio command 等价于允许启动本地程序，因此：

- 安装/启用前显示 executable、args、cwd、env 名称和来源。
- 环境变量使用 allowlist，不把所有进程环境透传。
- 凭据通过 OS keychain/credential broker 注入，不显示完整值。
- child process 有独立 stdout protocol、stderr log、timeout 和 shutdown。
- 未信任 Server 不获得 .jm 根目录或任意 shell。

### 11.4 远程 Streamable HTTP

- 使用 HTTPS。
- OAuth 按 MCP 当前规范采用 PKCE、Protected Resource Metadata 和 resource audience。
- access token 只存 OS secure storage。
- 禁止 token passthrough；MCP Server 访问上游服务使用独立 token。
- 本地 Server 校验 Origin、防 DNS rebinding，只绑定 loopback。

### 11.5 Prompt injection 与跨 Server 隔离

- MCP Resource、Prompt、Tool description 和 Tool result 都是不可信内容。
- 不能提升为 Host system policy。
- 一个 Server 不能读取完整对话或另一个 Server 的内容。
- Context Builder 明确标注来源和信任级别。
- Tool result 在传给模型前做 schema、大小、MIME 和资源链接校验。
- 调用链携带 CallChainId，拒绝 Jueming→MCP→Jueming 的无界重入环。

### 11.6 远程副作用与重试

- read_only 可按策略重试。
- idempotent_with_key 必须携带稳定 ToolCallId。
- non_idempotent 断线后不自动重放。
- unknown_outcome 必须展示给用户并提供核对入口。
- MCP progress 不能无限延长请求；始终存在 hard deadline。

---

## 12. Patch Proposal 与审批

PatchProposal 至少包含：

- proposal id 和 content hash。
- producer、model/operator/server version。
- base revision。
- typed operations。
- affected stable ids。
- supporting Artifact/Resource。
- explanation、confidence 和风险分类。
- created_at、expires_at。

审批流程：

~~~mermaid
sequenceDiagram
    participant A as Agent or MCP
    participant T as Tool Gateway
    participant U as User Approval UI
    participant K as Kernel

    A->>T: propose patch
    T->>K: validate against base revision
    K-->>T: valid proposal preview
    T-->>U: show diff, provenance, risk
    U->>T: approve
    T->>T: issue one-time ApprovalTicket
    T->>K: dispatch canonical Command
    K->>K: revalidate revision and invariants
    K-->>T: committed Revision or stale/conflict
    T-->>A: result
~~~

ApprovalTicket：

- 单次使用。
- 短期有效。
- 绑定 project、proposal hash、base revision、user session。
- 不包含可复用的广泛写权限。
- stale revision 后失效，不能自动 rebase 并提交。

---

## 13. 事件、状态投影与桌宠

所有 Provider/MCP 原始事件先由 Adapter 归一化：

| Normalized event | 含义 |
|---|---|
| agent.run.started | Run 开始 |
| agent.model.thinking | 模型生成中 |
| agent.tool.started | Tool 调用中 |
| agent.tool.progress | 有界进度 |
| agent.approval.required | 等待用户批准 |
| agent.user_input.required | 等待用户输入 |
| agent.run.completed | 正常结束 |
| agent.run.failed | 失败 |
| agent.run.cancelled | 取消 |
| mcp.connection.changed | 连接状态变化 |
| pdf.operation.progress | 页级 Pipeline 进度 |

UI、通知和未来桌宠只消费 AgentStatusProjection：

- 多 Run 按 WAITING_FOR_USER > FAILED > TOOL > THINKING > COMPLETED > IDLE 折叠。
- 状态机确定性运行，不调用模型。
- Provider 特有 event name 不进入组件。
- 订阅必须可释放，Agent/MCP Runtime 关闭后不遗留动画或 listener。

---

## 14. 持久化边界

### 14.1 .jm 内

继续属于 canonical：

- assets 中用户选择保留的原始 PDF。
- Segment、Order、Alignment、HumanAnnotation。
- revisions 中由批准 Proposal 产生的 ChangeSet。
- 必要的 source provenance reference。

可重建：

- PageRaster、OCR、Layout、Table、Formula、Vision、ImportDraft。
- 缓存与模型中间结果。

新增 Artifact Store 或 PDF cache 目录需要独立 format minor ADR；在此之前不能擅自把文件写进 revisions。

### 14.2 设备安全存储

- Provider API key。
- MCP OAuth token。
- 本地 Server trust/consent。
- Agent 默认 Profile 和预算。
- 默认 Run Ledger 与脱敏日志。

这些内容不进入 .jm。

### 14.3 Provenance

一个已提交 Agent Proposal 至少在 Revision summary 中保留：

- producer kind。
- provider/operator identity 和版本。
- proposal hash。
- input revision。
- supporting Artifact hash。
- 用户批准事实。

不要求把完整 Prompt、reasoning 或远程响应写入永久工程历史。

---

## 15. 启动、关闭与恢复

### 15.1 Desktop 启动

1. 打开本机设置和 secure credential references。
2. 打开/迁移 Kernel store。
3. 恢复未完成 canonical transaction。
4. 初始化 Schema、Slot、Operator 和 Tool Registry。
5. 初始化 PDF Worker Supervisor，但按需启动 worker。
6. 初始化 Agent Runtime 和 Run Ledger。
7. 按 Profile 懒连接 MCP Server。
8. 所有真实能力就绪后再发布对应 UI capability。

Agent、MCP 或 PDF Runtime 失败不能阻止纯人工 MVP 启动。

### 15.2 Headless MCP Sidecar 启动

1. 初始化 MCP stdio transport。
2. 协商协议但暂不发布越权 capability。
3. 连接现有本地 Kernel Host，或取得工程独占锁后启动 Headless Host。
4. 构建 Resource/Tool generation。
5. 发布 initialized。

### 15.3 关闭

1. 停止接收新的 AgentRun、ToolCall 和 PDF Operation。
2. 取消可安全取消的模型与只读 Tool。
3. 对 non_idempotent 外部调用记录 unknown outcome。
4. 使 ApprovalTicket 和 capability lease 失效。
5. 关闭 MCP session、stdio child 和 HTTP stream。
6. 终止 PDF worker。
7. flush Kernel 和 Run Ledger。

### 15.4 恢复

- PDF page Artifact 按 ArtifactKey 检查后可断点复用。
- 半完成 Artifact 不发布。
- 未提交 Proposal 可以恢复为 Draft，但必须重新检查 base revision。
- 不自动重放模型调用或 non_idempotent Tool。
- 已签发未使用的 ApprovalTicket 在重启后默认失效。

---

## 16. 第一条架构验证链

建议用一条窄垂直链证明所有边界，而不是一次实现完整 Agent 平台：

~~~text
本地打开一份双语或扫描 PDF
  → probe manifest
  → 只解析当前页
  → 显示 page raster + native text
  → OCR Slot 缺失时明确 UNBOUND
  → 若 OCR 可用，生成独立 OcrLayer
  → 用户选中一个 region
  → Agent 只读取该 region 的 MultimodalPageBundle
  → Agent 提出 OCR/分段/对齐 Patch Proposal
  → UI 显示来源、差异和 base revision
  → 用户批准
  → Kernel Command 创建新 Revision
  → stale revision 路径验证拒绝
~~~

并行增加一个最小 MCP 证明：

- 决明作为 MCP Server 暴露 get_project_summary、inspect_pdf_page 和 propose_patch。
- 决明作为 MCP Client 连接一个只读本地 stdio Server。
- 两侧均不能绕过 Tool Gateway。

---

## 17. 分期建议

### Phase A：合同与威胁模型

- 冻结 Slot Namespace 2.0 与 legacy alias。
- 冻结 PageSpace、PdfManifest、PagePrimitive、RegionGraph、PatchProposal。
- 冻结 ToolDescriptor、DataGrant、ApprovalTicket、AgentActivityEvent。
- 完成恶意 PDF、stdio MCP、远程 OAuth、Prompt injection 威胁模型。
- 决定 .jm format minor 中 Artifact 的位置，但不实现。

### Phase B：本地确定性 PDF

- PdfBackend spike 与许可证/打包评审。
- 页级 probe、native text、render、viewport priority 和 worker isolation。
- PDF UI 与 Kernel 只通过 PageView/Artifact。
- OCR 继续可 UNBOUND。

### Phase C：Tool Gateway 与只读 MCP

- 内建只读 Tool。
- MCP Client stdio、capability negotiation、tool generation、cancel、audit。
- MCP Server stdio sidecar，只暴露 resources 和只读 tools。
- 暂不接模型写入。

### Phase D：Agent Read/Analyze

- Model Provider Registry、AgentProfile、Context Builder、预算和 Run Ledger。
- 文本与 page image 输入。
- 只输出 Answer/Artifact。
- Agent 状态投影与可选桌宠/通知。

### Phase E：Proposal/Approval/Commit

- Patch Proposal、Diff、ApprovalTicket。
- stale revision、conflict、撤销和 provenance。
- 禁止直接 canonical write。

### Phase F：OCR、Layout 与视觉结构

- source.ocr、layout.region.detect、layout.reading_order。
- 表格、公式、图片描述按独立 Slot 增量接入。
- 扫描 PDF 和复杂多栏 fixture。

### Phase G：远程 MCP 和高级能力

- Streamable HTTP、OAuth、secure token、remote data consent。
- 评审后再考虑 sampling、elicitation 和 MCP Tasks。
- Server Kernel 之前不开放公网 MCP endpoint。

---

## 18. 架构验收矩阵

| 场景 | 必须结果 |
|---|---|
| 无 Agent Provider | 人工对齐、TXT、已有 .jm 正常 |
| 无 OCR Provider | 扫描 PDF 可显示，OCR 明确 UNBOUND，不伪造文本 |
| 文本型 PDF | 可用 native text，不强制跑 OCR |
| 恶意/损坏 PDF | worker 失败被隔离，不损坏工程 |
| 1000+ 页 PDF | 首屏按页加载，内存有界，不等待全书 |
| 多栏 PDF | ReadingOrder 独立可替换，不信任原生对象顺序 |
| Tool list_changed | 新 generation 原子替换，不出现半注册 |
| 一个 MCP Server 崩溃 | 其他 Server、Agent 和 Kernel 不受影响 |
| 非幂等 Tool 断线 | 标记 unknown_outcome，不自动重放 |
| 远程完整 PDF | 必须单独确认资产范围和 Provider |
| Agent 读批注 | 默认拒绝，只有显式 DataGrant 才允许 |
| Agent 修改正文 | 只生成 Proposal；无 ApprovalTicket 不提交 |
| base revision 过期 | Commit 拒绝并展示重新比较 |
| Desktop 与 Sidecar 同开工程 | 共享 Kernel Host 或单方取得写锁，不双写 |
| 重启 | 半成品 Artifact 不可见；Ticket 失效；canonical 历史完整 |
| PDF 坐标 | Backend↔PageSpace↔UI round-trip 在容差内 |
| 入口可见性 | Runtime、权限、错误恢复未齐备前保持隐藏 |

---

## 19. 主要风险与控制

| 风险 | 控制 |
|---|---|
| Slot 与 Tool 再次混用 | 独立 Registry、Descriptor 和 ADR |
| 外部 MCP Tool schema 膨胀模型上下文 | AgentProfile allowlist、generation snapshot、按需 tool set |
| PDF Parser CVE 或资源炸弹 | 独立进程、资源上限、禁脚本、超时和更新策略 |
| 模型覆盖人工编辑 | Proposal + base revision + 用户批准 |
| OCR/Layout 更换导致来源漂移 | Artifact hash、PageId、region anchor、diff |
| stdio Server 获得过大本机权限 | 启动即授权、env allowlist、无默认 roots |
| 远程数据泄漏 | DataGrant、范围预览、传输审计、默认当前选择 |
| Tool 自动重试产生重复外部副作用 | idempotency class、稳定 ToolCallId、unknown_outcome |
| Desktop 与 Headless 双写 .jm | 独占工程锁和单 Kernel Host |
| 运行日志变成第二份正文库 | 最小审计、默认设备存储、Revision 只留 provenance |
| UI 硬编码 Provider 名 | 所有状态来自 Registry/Event |
| MCP 规范演进 | Adapter 隔离、protocol negotiation、conformance suite |

---

## 20. 需要新增的 ADR 候选

在实施前至少评审：

1. Slot Namespace 2.0 与 1.0 alias/deprecation。
2. Slot、Tool、Agent 和 MCP Adapter 分离。
3. Jueming 同时作为 MCP Host/Client 与 MCP Server。
4. Desktop/Headless 单 Kernel Host 与工程写锁。
5. Agent 只输出 Artifact/Patch Proposal。
6. ApprovalTicket 是 canonical commit 的一次性授权。
7. PDF Immutable Asset + Page Artifact Graph。
8. PageSpace/1 坐标合同。
9. 不可信 PDF 在独立 worker process 运行。
10. Model/MCP credential 只进入 OS secure storage。
11. Agent Run Ledger 不属于 canonical Revision。
12. 远程 MCP OAuth 与 DataGrant。

在这些 ADR Accepted 前，不应：

- 将 MCP 类型加入 jueming-core。
- 将 Agent/Plugin 入口显示在生产设置页。
- 在 .jm 中创建未经版本化的新目录。
- 对外发布 Slot 或 Tool namespace。
- 选定并打包 PDF/AI 重型依赖。

---

## 21. 仍需验证的开放决策

### PDF 引擎

需要用同一 fixture 矩阵比较：

- PDFium native。
- PDF.js worker。
- 其他可接受许可的引擎。

比较维度：

- Windows x64、macOS Universal 打包。
- native text bbox、CJK、字体、旋转、裁剪、图片和标注。
- 渲染一致性和坐标 round-trip。
- 恶意文件隔离和更新成本。
- 二进制体积、内存、首屏延迟。
- 开源/商业许可。

在 spike 前不把引擎名字写入稳定 Schema。

### Agent Runtime 进程位置

候选：

- Rust/Tauri Host 内的受控 Runtime。
- 独立本地 Agent worker。
- 远程 Agent service。

稳定合同应先固定 ToolGateway、ResourceGateway、RunLedger 和 Policy；执行位置可以后换。

### Inbound MCP 的第一宿主

需验证：

- Sidecar 连接现有 Desktop Kernel Host 的本地 IPC。
- Sidecar 独立启动 Headless Kernel Host。
- 两种模式下工程锁、凭据、UI 审批和关闭恢复是否一致。

---

## 22. 参考资料

检索日期：2026-09-02。仅将这些资料视为架构参考，不把其中的产品范围自动加入决明。

- [决明全局 Architecture Handoff v0.2](../../jueming_global_architecture_handoff_v0.2.md)
- [决明 MVP Phase 0 合同](mvp-phase0-contracts-v0.1.md)
- [ADR-009 本地 in-process Operation/Data contract](../adr/ADR-009-in-process-rpc-contract.md)
- [ADR-010 UNBOUND Slot](../adr/ADR-010-unbound-slot.md)
- [决明设置系统完整设计 v0.2](../design/jueming-settings-system-design-v0.2.md)
- [Motrix README 与四层架构](https://github.com/agalwood/Motrix)
- [Motrix 贡献指南：Host-neutral core 与统一 transport](https://github.com/agalwood/Motrix/blob/main/CONTRIBUTING.md)
- [Motrix Plugin Hook Runtime Specification](https://github.com/agalwood/Motrix/blob/main/docs/plugin-hook-runtime.md)
- [dsh-pet 架构](https://github.com/ysyyhhh/dsh-pet)
- [dsh-pet HarnessBridge](https://github.com/ysyyhhh/dsh-pet/blob/a17346b9c41efb4220f387216008928bba652e0e/src/integration/HarnessBridge.ts)
- [DeepSeek Harness Architecture](https://github.com/deepseek-ai/deepseek-harness/blob/master/docs/architecture.md)
- [DeepSeek Harness MCP Client Bridge](https://github.com/deepseek-ai/deepseek-harness/blob/master/packages/mcp/mcp-client/README.md)
- [MCP Architecture 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25/architecture)
- [MCP Lifecycle 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle)
- [MCP Transports 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports)
- [MCP Authorization 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25/basic/authorization)
- [MCP Resources 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25/server/resources)
- [MCP Tools 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25/server/tools)
- [MCP Sampling 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25/client/sampling)
- [MCP Elicitation 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25/client/elicitation)
- [Docling Architecture](https://docling-project.github.io/docling/concepts/architecture/)
- [Docling Standard PDF Pipeline](https://github.com/docling-project/docling/blob/main/docling/pipeline/standard_pdf_pipeline.py)
- [PDF.js Architecture](https://github.com/mozilla/pdf.js/blob/master/AGENTS.md)
- [PDFium Core Architecture](https://pdfium.googlesource.com/pdfium/+/refs/heads/main/core/README.md)
- [PDFium Public Embedding Boundary](https://pdfium.googlesource.com/pdfium/+/refs/heads/main/public/README.md)

---

## 23. Handoff

本架构的实施顺序不是“先接一个聊天框”，而是：

> **先冻结 Tool/Data/Approval/PDF Page 合同，再建立本地确定性 PDF 与只读 MCP，最后让 Agent 在同一受控边界内读取、计算、提出 Proposal。**

只要守住以下四条，后续更换模型、PDF 引擎、OCR Provider、MCP SDK 或 Desktop Shell 都不会破坏工程：

1. Canonical 只由 Kernel Command 修改。
2. Slot、Tool、Agent、MCP 分层。
3. PDF 原始资产不可变，理解结果都是带 provenance 的页级 Artifact。
4. 远程或非确定性能力默认最小权限、可审计、可取消、可降级。
