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
- 主平行视图采用现有 `AlignedWorkspaceViewport` + `useAlignedBlockLayout`：按 Alignment 计算双侧高度、可视区裁剪与 overscan、稳定 ID 锚定和 ResizeObserver 测量。搜索结果使用 `@tanstack/vue-virtual`；不把全工程 Segment 常驻 DOM。
- 双栏工作区在导入／打开工程或切换文档对时建立 `useWorkspaceIndex` 内存索引；仅接受 Kernel 返回的结构，后续 Revision 按稳定 ID 增加、替换和删除条目。正文 Slice 只更新对应对象的响应式正文／加载状态，不重建对齐投影；正文淘汰同时清空索引对象中的正文。对齐检查复用 ID 索引，可视区用二分定位；逐帧布局诊断需显式开启 `VITE_ALIGNMENT_LAYOUT_DEBUG=true`。缓存可随时重建，不写入 canonical 历史。
- UI 图标使用 Lucide；桌面品牌图标统一从 `assets/brand/jueming-aligner-icon-master-v2.png` 生成。
- 设备设置使用版本化 TypeScript Schema 和 Pinia 单一 Store；Vue 组件不直接读写持久化。桌面端经 typed `KernelClient` 调用 Rust 适配器并写入 Tauri Store，浏览器预览仅使用独立 localStorage fallback；设置数据永远不进入 `.jm`。
- 设置导航由 capability registry 驱动。只有 `available` 能力显示可操作页面；`UNBOUND` 的账号、桌宠、语料库服务、上传、社区、Agent、插件和通知不显示假按钮。动效由同一 `MotionPolicy` 汇总系统辅助功能、全局模式、后台状态和局部开关。

### 主题与视觉

- 当前项目不使用 Tailwind；主题由全局语义 CSS 变量和组件 scoped CSS 组成。
- 下拉选择统一使用可见的原生 `<select>`，保留系统选项菜单、键盘交互和表单语义。macOS WebKit 原生按钮放大后字号/内边距受限且箭头遮字，因此收起态由全局 CSS 仿制 macOS 的圆角按钮与蓝色双箭头，默认最小高度 34 px，搜索与书签工具栏为 38 px；文字为 13 px，箭头有独立的右侧留白。组件只调整布局尺寸，不叠加透明 select 或另建弹出选项列表；背景复用主题变量，兼容护眼模式。强制颜色模式恢复平台绘制。
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

### 2026-09-13 架构审查修复

- 工程会话持有 `.writer.lock` 的 OS 排他锁，进程退出自动释放；不删除锁文件以避免 inode 替换导致双写。读取与安装工程在同一 Host 临界区完成，同一 Host 重开当前工程复用锁。新建不能覆盖已有工程。
- Kernel 落盘另以 `.commit.lock` 串行化，校验磁盘 project/revision 后才发布后继 Revision。相同快照 flush 不重写历史；已提交 Revision 不得被旧快照覆盖，未被 `project.json` 引用的孤儿 Revision 仍可恢复覆盖。
- 原生 canonical 写入统一经 `execute_command(CommandEnvelope)`；裸参数写命令退出 Tauri 注册。弃用的 Alignment 名称仅在 envelope 解析时映射为 Group/Ungroup。`command_receipts` 是向后兼容的可选持久化字段，与 Revision 同一原子快照提交；Undo/Redo/Restore 保留累计回执，重试返回原提交版本，异参复用和过期版本拒绝。
- 工程打开／创建复用 Edit 的保存、放弃、继续编辑守卫；保存失败保留草稿，保存中切换等待完成。编辑会话冻结起始工程和版本，失败重试保留同一 command ID。批注草稿同样冻结版本、保留失败输入；异步内容合并／拆分确认拒绝过期草稿。
- `WorkspaceProject` 只提供结构、文本 hash/长度、summary、历史摘要及 sidecar；canonical `ProjectSnapshot` 留在 Rust。正文通过最多 200 个稳定 ID 的 `load_parallel_slice` 读取，检查工程、版本、文档和范围。前端正文缓存有界，按 hash 保留未变内容并拒收旧响应；可见区域触发加载。
- Agent 同步投影只提供 ProjectIdentity，不复制 canonical 工程；新版本才读取 workspace 结构。当前视图 Find 逐批读取该视图的 Slice，在前端匹配并只保留命中 ID，不创建 Project Search session，也不把全文留在缓存。主平行视图继续使用既有布局实现。

### 2026-09-13 桌宠分件素材准备

- `anime-dev` 分支新增独立的 SVG puppet v1 资源包：橘猫 31 片、金毛 29 片。`master.svg` 保存美术源，`rig.json` 保存关节父子关系、局部原点和独立绘制顺序；切片及部件总览由 `scripts/companion/export-parts.py` 导出。
- `motions.json` 将局部动作片段与场景位移分开，提供走路、跳上猫窝、睡觉和去花园的有限动作原型。纯采样器不拥有计时器；独立开发入口 `/companion-lab.html` 提供关节检查、切片下载和手动播放，减少动态、后台及失焦时停止播放。
- 此阶段是素材和动作验证，生产 Companion 仍使用原有静态 SVG / Web Animations；没有引入 Pixi 或 Spine。资源格式、开发与后续接入约定见 `docs/design/companion/svg-puppet-pack-v1.md`。
- 按用户选定的圆脸眯眼橘猫参考图，增加 Image Gen 插画帧通道：猫/金毛各 16 帧、独立 atlas 和单次 GIF 预览；页面用纯帧采样器及有限 rAF 控制暂停/终点，不内嵌自动循环 GIF。SVG 绑定保留为独立检查通道，尚未自动混用两套 renderer。内部预览通过 CLI 覆盖为 `127.0.0.1:1422`，保留 Tauri 默认 1420 与 HMR 1421。详细素材来源、动作与限制见 `docs/design/companion/sprite-hybrid-v1.md`。

## 明确延期

MVP 不实现 POS、Lemma、NER、自动语义对齐、OCR、云协作和外部插件加载。新增这些能力前，应先确认 Slot/Artifact 合同、资源与安全边界，并新增 ADR，而不是直接把 Provider 逻辑嵌入 Vue 组件或现有 Segment schema。

## 决策变更流程

1. 先说明要改变的现有不变量及原因。
2. 新增 ADR，记录 Context、Decision、Consequences 和迁移方案。
3. 更新 `docs/adr/000-index.md`、Phase 0 合同版本及本总览。
4. 再修改 Rust Core、KernelClient、前端交互和持久化实现。


### 2026-09-13 多译本工程、自研 Panel 和导入向导

- 领域变更见 ADR-018：显式共享原文／多译本的工程 2.0；旧双文档 1.0／1.1 可继续读取。Alignment 占用按文档对隔离；原文 Split／Merge 覆盖所有译本关系；每份资源记录独立 ImportProfile。
- WorkspacePanelStrip 是自研展示容器，使用已有 Atlaskit 做列拖拽，支持键盘调序、指针／键盘调宽，按工程 ID 在设备 localStorage 保存布局。不引入 Dockview，不把 Panel ID 写进 canonical 数据。
- comparison-projection 组合现有双列 block 投影；两文档路径直接使用原算法。多文档中重叠 n:m 形成共享展示带，但保留每条独立 Alignment 和绑定按钮；AlignedDocumentList 仅渲染可视区。固定高度的绑定栏保证正文起点一致，测量取各列最大值撑齐，宽度变化仅清除该列测量。交叠 n:m 组内按每条真实关系的起点建立高度约束，以最长路径分配 Segment 位置；无法同时满足的交叉约束保留文本顺序并标示交叉绑定。单个巨大 n:m 组内部同样按 Segment 位置裁剪正文与 DOM，滚动锚点跟随稳定 SegmentId。
- 多列支持分别高亮／解除真实绑定、跳至所选文档对使用原有编辑工具、全译本有界视图查找。文档对切换复用未保存草稿保护；搜索和替换增加可选 document_ids，以区分同语种译本。导出按译本分组，保留所有未绑定文本。既有译法研究界面继续以首份译本为目标，其候选、证据指纹及确认检查均按该文档隔离，不把其他列候选混入首份译本名下。
- 新建向导顺序为工程信息、原文、译本 1…N、确认；每次增加译本新增独立页面。SISU 展示文案改为 seg 分段；真实分段仍沿用物理非空行加清洗。预览提供实际边界、清理前后文本与清洗原因，每页 20 段。请求／会话代数拦截迟到结果；Kernel 在任何落盘前校验所有输入和预览 SHA。
- 验证入口：tests/comparison/run-contracts.mjs、tests/comparison/browser-regression.mjs、tests/import-wizard/browser-regression.mjs 与 crates/jueming-kernel/tests/comparison.rs；浏览器回归使用实际 Vue/App 加确定性 Host 桥，真实落盘／解码／历史由 Rust 测试覆盖。


### 2026-09-13 导入自动编码识别

- 接入 `chardetng 1.0.0` 与 `encoding_rs 0.8.41`。BOM 优先、UTF-8 校验、统计检测依次执行；ISO-2022-JP 转义文本由检测器处理。GB18030 改为纯 Rust 跨平台严格解码，无效字节不替换。
- 新建向导每份文本默认独立自动识别，预览显示具体编码和识别依据；保留手动编码选择。手动切换、重选文件和关闭向导均沿用已有请求世代保护，过期结果不能覆盖当前结果。
- 自动选项只存在于预览请求。创建直接复制成功预览返回的 profile 并携带 SHA-256，持久化 SourceAsset 与主文档 profile 均为具体编码。新增编码合同见 ADR-019。

### 2026-09-13 新建向导删除译本入口

- 每个可删除译本的步骤旁始终显示独立删除按钮，无需先完成前置步骤或进入该译本页面；至少保留一份译本，创建工程或选择文件期间禁用删除。
- 删除当前页时返回前一份文本，删除其他页保留当前页与预览；默认译本名称跟随剩余编号，已命名译本与其他导入设置保持原值。删除会作废该译本的待返回预览并恢复页内焦点。

### 2026-09-13 批注侧栏直角外观

- 批注侧栏外框改为直角，取消左上、左下的 16px 圆角；下拉框外框与箭头底色恢复原有圆角。

### 2026-09-15 · Tailwind 与三主题实施约定

- 按当前用户要求接入 Tailwind CSS 3.4.19 + PostCSS；保留 macOS 11 的既有支持范围，不采用要求 Safari 16.4+ 的 Tailwind 4。禁用 Preflight，避免重置原生表单与阅读排版。
- `apps/desktop/tailwind.config.ts` 将 `bg-raised`、`text-ink-900`、`border-line` 等 utilities 映射到现有语义变量。设置页面开始使用模板 utilities，其余组件通过 scoped `@apply` 迁移共享颜色；动态坐标、虚拟化布局、动画与领域状态选择器保留专用 CSS。
- `data-theme="light|eye|dark"` 仍由设备设置驱动；Tailwind 的 `dark:` selector 复用该属性，不维护额外 `.dark` 状态。新增颜色优先接入语义调色板，三个主题不在组件中分别硬编码。
- 暗色采用中性深灰基础面、较亮浮层与低饱和绿强调；文字强调色与实心按钮填充色分离，增强对比度有独立暗色值。保留品牌与宠物插画原色。
- 参考 Apple HIG [Dark Mode](https://developer.apple.com/design/human-interface-guidelines/dark-mode)、[Color](https://developer.apple.com/design/human-interface-guidelines/color)，及 Tailwind [浏览器兼容性](https://tailwindcss.com/docs/compatibility)。这里是 WebView 的自有语义色实现，不是 AppKit 动态系统色 API。

- 主题增加 `system` 偏好：持久化用户选择，根 `data-theme` 仅写解析后的 light/dark/eye。使用 `matchMedia` change 监听系统明暗并在销毁时移除；系统变化只更新外观，不覆盖持久化偏好。现有默认值及手动主题保持不变。

### 2026-09-15 · 跟随系统主题修复

- 原生主题缓存不能覆盖后续 `prefers-color-scheme` change 事件；两条通知路径都更新当前系统明暗，手动主题保持独立。移除临时 localhost 主题探针请求。
- 设备外观设置增加 `systemLightTheme: light | eye`，仅在跟随系统时参与解析：系统深色使用 dark，系统浅色使用用户指定的 light/eye。旧设置缺省 light，保持原有行为；不修改工程格式。
- 设置页提供“暗黑 ↔ 明亮”和“暗黑 ↔ 护眼”，切换立即生效并沿用现有设置持久化。

### 2026-09-15 · 阅读进度与合并验证

- 页脚按当前文档对的 Segment 显示原文／译文已对齐比例；导航游标表示滚动比例，拖动释放或键盘操作驱动共享视口。进度显示不写入 canonical 数据。
- 两列高度测量在单个动画帧中批量发布，忽略小于 0.5px 的测量抖动；尺寸重置、组件卸载取消待处理测量。锚点在布局更新后校正，关闭浏览器原生滚动锚定以避免重复补偿。
- 合并验证复用既有五语言 Kernel 文案表；内存渲染测试显式选择语言，跨层浏览器夹具跳过首次引导，避免系统语言及引导项目替换测试工程造成误判。
