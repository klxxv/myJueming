# 决明对齐器决策总览

_更新时间：2026-09-01。本文是仓库级快速入口；正式架构依据仍是 [`docs/adr/`](docs/adr/000-index.md) 与 [Phase 0 合同](docs/architecture/mvp-phase0-contracts-v0.1.md)。_

## 已接受的架构决策

| 主题 | 当前决策 | 依据 |
| --- | --- | --- |
| 桌面技术栈 | Tauri 2 + Rust Kernel + Vue 3 Composition API + TypeScript strict + Vite | [ADR-001](docs/adr/ADR-001-tauri-rust-vue-stack.md) |
| 身份模型 | canonical 对象使用稳定 opaque ID；MVP 使用 UUIDv7，位置不承担身份 | [ADR-002](docs/adr/ADR-002-stable-ids.md) |
| 内容单位 | Segment 是可编辑、可引用、可排序、可对齐的 canonical relation unit | [ADR-003](docs/adr/ADR-003-segment-canonical-unit.md) |
| 顺序模型 | SegmentOrder 独立于 Segment；重排只改变结构，不改变身份和关系 | [ADR-004](docs/adr/ADR-004-segment-order.md) |
| 对齐模型 | Alignment 只引用 Segment，支持 1:1、1:n、n:1、n:m | [ADR-005](docs/adr/ADR-005-alignment-segment-refs.md) |
| 人工批注 | HumanAnnotation 作为 sidecar layer，不污染正文和机器标注 | [ADR-006](docs/adr/ADR-006-human-annotation-sidecar.md) |
| 历史语义 | Revision append-only；Undo、Redo、Restore 都追加新 Revision | [ADR-007](docs/adr/ADR-007-append-only-revision.md) |
| 大文档边界 | Chunk 是物理单位，Slice 是 UI 数据边界，稳定 ID 是语义边界 | [ADR-008](docs/adr/ADR-008-chunk-slice-storage.md) |
| 前后端边界 | Tauri 内实现 in-process Operation/Data contract；前端只依赖 `KernelClient` | [ADR-009](docs/adr/ADR-009-in-process-rpc-contract.md) |
| 延期能力 | Slot 可以处于 `UNBOUND`，不删除合同，也不显示可操作的假功能 | [ADR-010](docs/adr/ADR-010-unbound-slot.md) |
| 工作区模式 | Review、Edit、Order、History 共享领域投影和显式模式状态机 | [ADR-011](docs/adr/ADR-011-parallel-workspace-modes.md) |
| 查找与搜索 | 当前视图 Find 与工程级 Search 分离；虚拟列表不能依赖 DOM 全文查找 | [ADR-012](docs/adr/ADR-012-find-search-separation.md) |

## 当前实施约定

### Rust crate 与模块边界

- 保留现有 `core / protocol / storage / kernel / desktop` 分层，不为文件拆分新增 crate，也不改变领域合同、DTO wire shape 或 `.jm` 格式。
- 各库 `lib.rs` 只声明私有模块并显式重导出现有公共 API；`KernelService` 仍是无状态的统一 façade，功能模块不建立第二份工程状态。Tauri `lib.rs` 仅保留应用装配与命令注册。
- `jueming-core` 按稳定 ID、Project/Document、Segment、Order、Alignment、Revision 和 import 原语拆分；领域校验与相应对象放在一起，解码与文本分段分别维护。
- `jueming-protocol` 按导入、工程、命令、事件、历史、搜索、sidecar、导出和 Slice 组织 DTO。旧的 crate 根路径与兼容别名保持可用；新内部模块显式依赖 `jueming-core` 类型，不依赖根模块的通配导入。
- `jueming-storage` 只处理目录布局、通用 JSON 快照、原子写入和派生缓存，不依赖 Core、Protocol 或 Kernel；历史提交语义由 Kernel 决定。
- `jueming-kernel/commands` 按内容结构、顺序、对齐关系、gap、书签和批注拆分。导入、工程生命周期、搜索替换、导出和历史各自成模块；共享的 `validation`、`projection`、`revision`、`persistence`、`sidecar` 保持单一实现，内部 helper 使用受限可见性。
- 桌面 `commands/` 仅做 typed IPC 适配，`state.rs` 保存当前工程会话；命令名、参数、返回值和成功持久化后才替换会话快照的顺序不变。
- 黑盒回归测试放在各 crate 的 `tests/`，通过原有根路径 API 调用；仅需要私有提交步骤的故障注入测试保留在持久化模块中。新增功能沿职责模块扩展，不再堆回 `lib.rs`，也不通过 `use super::*` 形成隐式生产依赖。

### 工程与持久化

- `.jm` v0.1 使用原子 JSON snapshot 加逐 Revision 文件，由 `jueming-storage` 隔离格式细节。
- 正式 `revisions/` 不参与缓存清理；`.jm/cache/` 必须可重建。
- 自动保存间隔是 UI 设置，但每次落盘仍必须经过完整 Kernel ChangeSet/Revision 事务。
- 浏览器 fixture 仅用于 UI 预览；Tauri 没有打开真实工程时保持只读，并引导用户打开或新建 `.jm`。
- 最近打开的工程路径保存在本机设置中，启动时尝试恢复；失败时回到项目页而不是伪造可写工程。

### 前端与交互

- `ParallelWorkspace` 统一双栏虚拟列表、查找定位、选择、编辑和排序；模式差异由控制器表达。
- Segment 选择和 Alignment 选择是两套状态：前者服务 Link，后者服务 Unlink/Merge/Split；跳转锚点不制造操作选择。
- 编辑退出、模式切换和成功的 Alignment 操作必须清理暂态选择，避免旧单元格残留高亮。
- 拖拽使用 `@atlaskit/pragmatic-drag-and-drop`，只从中文侧显式手柄启动；虚拟行卸载时注销监听。
- 长列表使用 `@tanstack/vue-virtual`，不把全工程 Segment 常驻 DOM。
- UI 图标使用 Lucide；桌面品牌图标统一从 `assets/brand/jueming-aligner-icon-master-v2.png` 生成。
- 设备设置使用版本化 TypeScript Schema 和 Pinia 单一 Store；Vue 组件不直接读写持久化。桌面端经 typed `KernelClient` 调用 Rust 适配器并写入 Tauri Store，浏览器预览仅使用独立 localStorage fallback；设置数据永远不进入 `.jm`。
- 设置导航由 capability registry 驱动。只有 `available` 能力显示可操作页面；`UNBOUND` 的账号、桌宠、语料库服务、上传、社区、Agent、插件和通知不显示假按钮。动效由同一 `MotionPolicy` 汇总系统辅助功能、全局模式、后台状态和局部开关。

### 主题与视觉

- 当前项目不使用 Tailwind；主题由全局语义 CSS 变量和组件 scoped CSS 组成。
- 字体优先系统 UI 字体；字号、行高和字重由 `styles.css` 的 `--jm-font-*` / `--jm-line-height-*` token 管理，采用 Apple HIG macOS 的 13/16（Body）、12/15（Callout）、11/14（Subheadline）、15/20（Title 3）、17/22（Title 2）层级，在 WebView 中使用 CSS px 而非 CSS pt。UI 使用 400/500/600，17 px 页面与弹窗标题保持 Regular；有意强调的 15 px 区域标题使用 Semibold。
- 中文辅助信息至少使用 11 px，不再使用 9 px 小字；多行说明保留适合中文的行距。双语正文独立使用 16/15.5 px 和现有阅读缩放，不随 UI 密度缩成 13 px；编号、键盘提示复用系统等宽字体栈。排版调整不改变控件点击尺寸、模式状态机或虚拟列表机制。
- 明亮与护眼模式共享 `paper/surface/input/hover/status` 表面色合同。新增组件不得硬编码 `#fff` 作为中性表面，应复用 `apps/desktop/src/styles.css` 中的变量。
- 护眼模式覆盖框架、工具栏、列表、卡片、输入区、弹窗、批注、搜索和历史 Diff，同时保留但柔化成功、警告和错误语义色。
- Review 跳转高亮是短时、可取消动画；系统减少动态效果或用户触控滚动时立即终止。

### 跨平台与性能

- Windows 生成 x64 NSIS/MSI；macOS 生成包含 Apple Silicon 与 Intel 的 Universal `.app/.dmg`。
- macOS 使用 Command 快捷键布局和可配置触控板优化，输入框保留系统原生撤销。
- 不引入空闲轮询、无限动画、常驻 GPU 合成提示或不受限后台任务。
- Apple Silicon 温升结论必须由真机 Activity Monitor/Instruments 压力测试确认，CI 只能验证原生构建和功能。

### CI 与发布

- `main` push 和 pull request 运行 TypeScript build、Rust format、Clippy `-D warnings` 与全 workspace tests。
- 手动触发或 `v*` tag 才执行 Windows/macOS 安装包矩阵，产物保存为 workflow artifacts。
- Actions 使用固定提交 SHA，默认权限为 `contents: read`；当前流程不自动创建 GitHub Release。
- 正式 macOS 分发仍需 Developer ID 签名与 notarization；当前测试包只使用 ad-hoc 签名。

### 2026-09-06 Agent / MCP / Pipeline 扩展

- 当前用户已明确授权本轮 Agent、Pipeline 和小花园实施；ADR-015 与 ADR-016 记录新增领域边界。原有稳定 ID、canonical Revision、sidecar 和离线工程合同保持生效。
- 桌面命令、MCP 和内置 runtime 共用 `Arc<LocalAppHost>`；不允许多个进程分别持有工程写入副本。外部修改生成持久化提案，只有可信本机界面可批准。
- MCP 采用官方 Rust SDK `rmcp` 的 stdio sidecar，桥接显式开启、会话级鉴权的 loopback HTTP；AG-UI SSE 投递应用事件。没有读取或伪造 Codex 等外部工具的完整聊天记录。
- 内置 runtime 使用独立 Rust provider/tool-loop port、SQLite 会话与系统凭据存储，支持显式配置的本机或 HTTPS OpenAI-compatible 服务。未配置不运行；可选聊天初始化失败不应阻断基础工程。没有把 Rig、CopilotKit 或外部插件运行时标记为已集成。
- Pipeline 用 Vue Flow 展示，Rust `jueming-pipeline` 执行闭合的原文、规范化、中文 Jieba 分词和 artifact 算子。方法版本与派生结果位于 `.jm/extensions/pipeline`，不修改 canonical 原文或冒充 canonical Revision。
- 全局侧栏位于路由页之外，助手和批注草稿按工程隔离；搜索、工程、设置、历史和 Pipeline 可保持侧栏。Pipeline 与 History 同级，设置位于 Pipeline 下方。
- 小花园默认静态，生动模式通过懒加载 Web Animations renderer 实现有限反馈。静态、隐藏、减少动态、后台和编辑安静模式可释放渲染资源，普通 DOM 审核按钮独立工作。`CompanionRenderer` 为未来 Spine/Pixi 资源适配保留接口，当前未安装 Spine。
- Agent 和桌宠设置页现已接入真实能力；任意外部插件运行、云协作、自动语义对齐等仍保持未绑定。声明式扩展 registry 不等于已运行插件。

## 明确延期

MVP 不实现 POS、Lemma、NER、自动语义对齐、OCR、云协作和外部插件加载。新增这些能力前，应先确认 Slot/Artifact 合同、资源与安全边界，并新增 ADR，而不是直接把 Provider 逻辑嵌入 Vue 组件或现有 Segment schema。

## 决策变更流程

1. 先说明要改变的现有不变量及原因。
2. 新增 ADR，记录 Context、Decision、Consequences 和迁移方案。
3. 更新 `docs/adr/000-index.md`、Phase 0 合同版本及本总览。
4. 再修改 Rust Core、KernelClient、前端交互和持久化实现。
