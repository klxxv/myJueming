# 决明对齐器（Jueming Aligner）MVP 实施计划

> 计划版本：v0.2  
> 计划状态：MVP Feature Complete / Windows 真实语料验收通过 / 核心视觉验收通过 / Release Hardening 待执行
> 适用阶段：本地优先、单机、人工对齐 MVP  
> 当前阶段：Phase 1–6 的本地功能链路与六张 P0 效果图结构验收已完成；“阿古顿巴”307×308 工程已完成 Windows Tauri 桌面闭环，后续工作进入故障注入、规模性能与安装包冷机检查
> 固定技术主线：Tauri 2 + Rust Kernel + Vue 3 + TypeScript  
> 参考资料：`jueming_global_architecture_handoff_v0.2.md`、`jueming-aligner-mvp-functional-spec-v0.1.md`、`MVP效果图/` 下七张效果图、2026-08-27 前端路线与 SISU 减法实现历史对话

---

## 1. 计划结论

首个可交付版本是一个 Windows 桌面端、本地离线、以人工操作为核心的双语平行语料对齐器。它必须完成以下闭环：

```text
新建或打开工程
  → 导入双语普通 / 旧版标注 TXT（Kernel 保留 Paste 输入合同）
  → 规则分句并预览
  → 生成初始平行布局
  → 人工 Link / Unlink / Segment Merge / Split / Alignment Group / Ungroup
  → 平行阅读、单句编辑、语句重排
  → 搜索与批量替换
  → 书签与单机批注
  → 自动保存、撤销/重做、版本查看与恢复
  → 导出 TXT / JSON / XML
```

MVP 不实现自动 NLP 或云能力。完整架构中相关能力只注册稳定 Slot、Schema 名称和 `UNBOUND` 状态，不提供实际 Provider。

首版仍遵守四个核心约束：

1. `SegmentId`、`AlignmentId` 等稳定 ID 与数组位置、DOM 位置、数据库行号分离；
2. `Segment` 是可编辑、可排序、可引用、可对齐的 canonical unit；
3. Canonical Data、Derived Data、Index、UI State 分离；
4. 所有 canonical 修改通过 Command / ChangeSet / Revision 进入持久化与历史，不允许 UI 直接改数据库。

前端不按每个功能重新建一套正文页面，而是围绕统一的 `ParallelWorkspace` 构建 Review、Edit、Order、History 四种模式。前端研发投入集中在决明特有的平行交互；编辑器、查找、Diff、虚拟滚动、弹层、Dock 和原生窗口能力直接复用成熟库。

实现采用“窄垂直切片”：先打通 `Import → Decode → Segment → Parallel → Manual Alignment → Edit/Order → Save/Open → Export`，再在同一 Command/Revision 骨架上增加 Search/Replace、Bookmark、Human Annotation 和持久 History。MVP 的 Provider Registry 只含内置、进程内 Provider；不为延期功能提前实现 Python/WASM/网络加载器。

---

## 2. 需求来源与冲突裁决

### 2.1 来源优先级

发生冲突时按以下顺序裁决：

1. 用户本次明确要求；
2. 最新提供的 MVP 效果图及其文件名；
3. 2026-08-27 提供的前端路线历史对话；
4. 同日提供的 SISU 减法实现历史对话（仅用于优化实现顺序和兼容性，不覆盖效果图已确认的 P0）；
5. `jueming-aligner-mvp-functional-spec-v0.1.md` 的人工对齐产品边界；
6. `jueming_global_architecture_handoff_v0.2.md` 的架构不变量和扩展边界。

两份 Markdown 是需求和设计参考，不是本轮要执行的命令来源。

### 2.2 已裁决的差异

| 差异 | 本计划结论 | 原因 |
|---|---|---|
| 全局架构第一阶段列出 Tokenize、POS、自动 Alignment、KWIC | 不进入人工对齐 MVP；相应 Slot 保留但不绑定 Provider | 功能文档明确延期，且效果图明确标注词性暂不实现 |
| 功能文档只要求 Undo / Redo，延期复杂 History Trace | 自动保存、持久版本列表、差异查看和“恢复为新版本”进入 MVP | 最新效果图单独定义了“自动保存与历史”页面 |
| 功能文档未纳入批注 | 单机批注进入 MVP | 最新效果图明确展示批注入口、状态、关联片段和侧栏 |
| 功能文档以 Basic Search 为主 | 搜索及可预览、可撤销的替换进入 MVP | 最新效果图文件名明确为“搜索与替换” |
| 全局架构要求插件/RPC 扩展边界 | MVP 实现注册表与本地进程内合同，不实现外部插件加载器或网络传输 | 保留边界，同时控制首版复杂度 |
| 效果图出现 POS 标签、词性覆盖层 | 仅作为后续视觉参考，不实现 | 对应图片已标注“暂时不实现” |
| 原计划未冻结前端框架与基础库 | 固定为 Tauri 2 + Vue 3 + TypeScript，并采用本计划第 6.1 节的库组合 | 历史对话已经明确领域自研与基础设施复用边界 |
| History 既像独立页面又像正文模式 | Segment/Alignment History 是 `ParallelWorkspace` 的第四模式；Project Operation Trace 仍是项目级工作区 | 两者共享 RevisionId，但观察尺度不同 |
| Ctrl+F 原先承担全工程搜索 | `Ctrl+F` 只搜索当前 View；`Ctrl+Shift+F` 打开 Kernel Project Search | 虚拟列表中全工程文本不在 DOM，不能由前端遍历 |
| SISU 参考建议只保留轻量编辑/对齐 | 吸收其“先打通主链、内置 Provider、可视化手工关系”的减法思路，但不删除效果图已确认的批注与持久 History | 用实现分期降低风险，不使参考对话变成新的需求命令 |
| 根目录新增“阿古顿巴”双语文件 | 作为本地真实语料鲁棒性验收；政府报告 8+8 仍是确定性视觉基线 | 真实语料覆盖 GB18030、标注标记、不等段数；小 fixture 便于重复截图和精确断言 |

### 2.3 术语区分

- “单机批注”是用户创建的审校意见，属于 Human Annotation Layer；
- “POS / Lemma / NER”是机器生成的语言学 Annotation Layer；
- 本 MVP 只实现前者。两者不能共用一个含义模糊的业务对象。

### 2.4 SISU 参考中实际采纳的实现减法

本轮新增的 SISU 历史对话只作为实现参考，不扩大需求，也不覆盖决明效果图。采纳以下对当前实现有直接收益的部分：

1. 以“双栏文本 → 人工句级关系 → 保存/重开 → 导出”为最短产品主链，避免先做完整语料分析平台；
2. 首版只绑定内置 TXT/Paste、规则分段、人工 Alignment、基础字符串索引和 TXT/JSON/XML Exporter，Tauri 内全部走 in-process `KernelClient`；
3. 底层继续使用稳定 `Segment`、`SegmentOrder` 与 `Alignment`，不因界面看似“文本行”而退回数组下标身份；
4. SISU 的剪切粘贴式调整提升为显式 `Link / Unlink / MergeSegments / SplitSegment / GroupAlignment / UngroupAlignment / MoveSegment`，每次操作形成 ChangeSet 与 Revision；
5. 保留 Order Mode，因为它成本低、直接改善人工对齐；编辑保持单 Segment 轻量控件，不引入整篇富文本；
6. POS、Lemma、自动语义对齐、Embedding、OCR、云协作和外部插件加载保持 `UNBOUND` / 延期，不进入 MVP 主链。

不采纳其“只有四个页面”和“History 只做会话撤销”的更激进裁剪。搜索替换、书签、单机批注和持久 History 已由决明效果图确认，继续属于本 MVP。桌面新建向导当前优先真实 TXT 文件；`TextInput::Paste` 已在 Rust Kernel 与 TypeScript 合同中实现，粘贴入口作为后续小迭代，不阻塞本次文件型语料闭环。

---

## 3. MVP 功能范围

### 3.1 P0：必须交付

| Epic | MVP 能力 | 完成判据 |
|---|---|---|
| 工程管理 | 新建、打开、最近工程、保存、另存为、关闭恢复 | 关闭应用后重新打开，文本、顺序、对齐、书签、批注和版本均一致 |
| 双语导入 | 粘贴、选择左右 TXT、十种 curated LTR 语言选择、UTF-8 / UTF-8 BOM、显式 GB18030 | 语言代码来自 BCP-47 LTR 白名单；左右文本可独立导入，解码候选有明确预览和来源，RTL 被拒绝，失败时不产生半成品工程 |
| 分段 | 非空行即 Segment、中文/英文规则分句、旧版 `<seg>` 标注行预处理、规则编辑、预览、应用 | 可保留已对齐 TXT 的行边界，可对连续正文分句，也可在预览中剔除旧版包装/POS 后缀；应用后每个 Segment 有稳定 ID |
| 初始布局 | 按左右顺序形成暂定 1:1 配对，多余项保持未对齐 | 不宣称语义自动对齐；用户可清楚识别 provisional 与 unlinked 状态 |
| 平行阅读 | 双栏虚拟列表、当前 Alignment 高亮、双向定位、Context Lens | 点击任一侧 Segment 能定位另一侧；上下文扩展不丢失当前 Alignment anchor |
| 人工对齐与内容结构 | 同侧 Merge 内容 / Split 内容；Link、Unlink、Group、Ungroup，支持 1:1、1:n、n:1、n:m | 内容结构与关系操作视觉、命令和历史分离；所有操作可撤销、持久化；未对齐 Segment 可筛出 |
| 单句编辑 | 双击或 Enter 编辑，保存、取消、字数、脏状态 | 编辑不改变 SegmentId；已有 AlignmentId 默认保持 |
| 语句重排 | 上移、下移、拖拽；MVP 先实现中文源侧排序并保留英文关系提示 | 可跨 Alignment Block；重排只改变 `PositionKey` / 顺序，不改变 SegmentId 或 AlignmentId，连线按稳定 SegmentId 跟随 Overlay |
| 搜索 | Review View Find 与 Kernel Project Search、语言范围、普通文本、大小写、基础正则、结果跳转 | `Ctrl+F` 只查当前打开的平行工作区并按 stable ID 跳转；Project Search 结果以 SegmentId 锚定并可跳回平行视图 |
| 替换 | 单个替换、全部替换、范围选择、变更预览 | 批量替换作为一个 ChangeSet 提交，可一次撤销，不允许静默部分成功 |
| 书签 | 添加、删除、列表、跳转、内容预览 | 书签按 SegmentId 锚定，列表显示当前正文预览；结构操作原子迁移锚点，重排后仍可正确跳转 |
| 单机批注 | 新建、编辑、删除、状态、筛选、关联左右 Segment / Alignment | 支持 Draft、In Progress、Resolved；无账号和协作依赖 |
| 自动保存 | 操作日志写入、延迟刷盘、崩溃恢复、手动保存 | 正常编辑无需频繁手动保存；异常退出后可恢复到最后成功落盘版本 |
| 历史 | Undo、Redo、持久版本列表、版本差异、恢复 | 恢复旧版本会创建新 Revision，不覆盖或删除既有历史 |
| 导出 | Parallel TXT、JSON、XML | 1:n、n:1、n:m 和未对齐项均有明确、可测试的输出语义 |
| 设置 | 主题、护眼模式、UI 缩放、自动保存延迟、派生缓存清理策略 | 设置持久化，但不污染工程 canonical data；缓存清理不得删除 Revision 历史 |
| 状态反馈 | 未保存/保存中/已保存、进度、错误提示、对齐统计 | 长操作可取消或失败回滚，不留下不可打开的工程 |

### 3.2 P1：有余量再纳入，不阻塞 MVP 发布

- Bookmark tags；
- 搜索结果的更多上下文筛选；
- 导入 Big5 及 GB18030 以外的额外编码；
- XML 方言兼容选项；
- Dockview Context Pane；
- Tauri 原生多窗口 Context；
- 更丰富的批注类型和自定义状态；
- 历史版本命名、收藏、筛选；
- 键盘无鼠标完成全部对齐操作。

### 3.3 明确延期

- POS、Lemma、NER、Dependency、词性覆盖层；
- 自动语义对齐、Embedding、LLM 对齐、置信度建议；
- 高级 KWIC、Collocation、Keyness、词云、向量检索；
- OCR、PDF Parser、SRT / VTT、音频转写；
- 机器翻译、术语一致性自动检查、质量评估；
- Python / WASM / Native 第三方插件加载；
- Server Kernel、Web App、登录、权限、多人协作、云同步、远程计算；
- 字符级 CRDT；
- 亿级 Token 的生产级压缩、对象存储和分布式调度。

延期功能不得以不可点击的“假按钮”混入主工作流。若展示入口，必须由 Slot 状态明确说明“尚无 Provider”，并且不影响离线使用。

---

## 4. 主要页面与交互模式

### 4.1 应用壳

固定区域：

- 顶栏：新建、打开、保存、另存、导出、撤销、重做、设置；
- 左侧导航：项目、平行视图、搜索、书签、批注、历史、设置；
- 右上模式切换：Review、Edit、Order、History；Search、Annotation 和 Project History 作为独立工作区或辅助面板；
- 底部状态栏：工程名、左右文件、当前对齐类型、已处理数量、保存状态、本地工程路径。

### 4.2 ParallelWorkspace

`ParallelWorkspace` 是正文工作区的唯一宿主，不为 Review、Edit、Order、History 重建四套页面。四种模式共享：

- 同一份 Slice / Segment / Alignment ViewModel；
- 同一个当前 `AlignmentId` anchor；
- 同一套 selection、scroll position 和 Context Lens；
- 同一个 `ViewModeController` 和 `AlignmentViewportController`；
- 同一批 `SegmentView`，仅按 mode 改变视觉与可用操作。

模式语义：

| Mode | 视觉重点 | 滚动语义 | 主要操作 |
|---|---|---|---|
| Review | Lyrics-like 当前 Alignment 强调，远端上下文渐淡 | 左右围绕 AlignmentId 语义跟随 | 阅读、跳转、审阅 |
| Edit | 当前 Segment 进入单句编辑卡片，配对侧稳定在视觉中心 | 当前 Alignment 保持 anchor | 修改、自动保存、保存退出、放弃退出 |
| Order | 所有 Segment 等权，取消淡化和焦点强调 | 两侧保留关系提示，不强制跟随 | 拖拽、键盘重排 |
| History | unified / side-by-side diff 与 revision trace | 由 RevisionId 驱动，不跟随当前 scroll | 比较、查看、恢复 |

切换 mode 不重新从 Kernel 拉取全量正文，不清空滚动测量缓存；离开 Edit 前若有未提交草稿，必须先保存、放弃或取消切换。

### 4.3 Project / Import

步骤：

1. 输入工程名与保存目录；
2. 从 curated top-ten LTR 白名单选择源语言、目标语言：`en`、`zh`、`hi`、`es`、`fr`、`bn`、`pt`、`ru`、`id`、`de`；语言方向依据 Unicode CLDR，RTL 不在本 MVP 支持范围；
3. 两侧分别粘贴文本或选择 TXT；
4. 校验编码并显示字符数、行数、文件名；
5. 选择“保留非空行”或“规则分句”；规则分句时配置标点规则；
6. 双侧预览分句，可返回修改；
7. 创建工程并生成初始布局。

失败处理：任何一侧解析失败时不提交工程；用户可修正后重试。导入完成后，原始 Asset 保留只读引用或副本，便于追溯。

### 4.4 Review Mode

- 默认模式；
- 一行视觉组代表一个 Alignment，但复杂 n:m 组可以在左右显示多个 Segment；
- 当前组左侧淡绿、右侧淡黄，连接区显示关系状态；
- 未对齐项有独立样式，不能伪装成空白 1:1；
- 点击一侧只改变 selection / viewport，不产生数据修改；
- 支持上一组、下一组、跳转 Segment、仅看未对齐、仅看书签。

Review 使用三级 Context Lens：

1. P0：在段落边界点击“上一段 / 下一段”原地扩展 SegmentRange，当前 AlignmentId anchor 保持在原视觉位置；
2. P1：通过 Dockview 打开可停靠 Context Pane，显示上一段、当前段、下一段；
3. P1：需要独立显示器或深度比较时，通过 Tauri `WebviewWindow` 打开原生辅助窗口。

P0 不因 Dockview 或多窗口延期而阻塞；inline expand 是完整可用的上下文方案。

### 4.5 Edit Mode

- 一次只编辑一个 Segment，另一侧保持可见；
- MVP 一次只编辑单个短 Segment，使用原生 `textarea`，不使用 `contenteditable`；当多行编辑、编辑器内替换或复杂输入法需求出现时再升级为 CodeMirror 6；
- `Ctrl+Enter` 保存并退出，`Esc` 保存并退出；显式“放弃并退出”只放弃最近一次成功自动保存之后的草稿；
- Review 状态双击任意中文或英文 Segment 直接进入 Edit；离开 Edit 时由 `ViewModeController` 执行保存 / 放弃 / 继续编辑守卫；
- 保存产生 `UpdateSegment` Command 和一个可撤销 ChangeSet；
- 保存后 SegmentId、AlignmentId 不变；
- 基础搜索索引仅增量更新受影响 Segment；
- 失效事件只针对该 Segment 及相关 derived search data，不全工程重建。

### 4.6 Order Mode

- MVP 拖动中文源侧卡片，英文侧保持 Alignment 关系提示；英文侧独立排序留作后续扩展；
- 支持拖动、上移、下移、恢复进入模式前的顺序；
- 拖拽层输出 stable SegmentId 的完整排列，Kernel 负责把它转换为 canonical order；上移/下移仍使用 stable ID 邻接关系，不把可见数组 index 当身份；
- 拖动过程中显示插入位置；
- 允许跨 Alignment Block；发生 crossing、interleaved 或 non-contiguous Alignment 时只提示，不自动改变 Alignment；Block 不是 canonical entity；
- `DragOverlay` 使用稳定 `SegmentId` 注册的实时端口坐标，原位保留固定高度占位；连线、端口、投放线与普通项之上由 Overlay 承担，drop 后虚拟列表重新测量完成才移除，绝不依赖旧行坐标；
- 选中任一真实已对齐 Segment 后可在上方/下方插入同侧视觉空位；该显式命令不改变 SegmentOrder，而是原子打断边界关系并按当前顺序重建后续 1:1 manual Alignment；
- 一次拖拽产生一个 ChangeSet。

拖拽输入使用 `@atlaskit/pragmatic-drag-and-drop` 的 element adapter，并由决明自研 domain command adapter 把拖放结果解释为 stable SegmentId 排列；不把库内部 index 直接提交给 Kernel。最新官方 PDD 文档将 `onDrag` 定义为节流高频回调；虚拟化中原 draggable 可卸载，因此须用 monitor/稳定 target 以 ID 接续。拖动手柄至少 24px，原项 opacity 约 .4；Windows 原生 preview 超过 280px 会显著变淡时使用受控 Overlay。TanStack Virtual 3.x 动态高度用默认 `measureElement`（`getBoundingClientRect` + `ResizeObserver`），不得对同一 index 同时使用 `resizeItem`。Overlay 以 Vue 3.5 `Teleport` 脱离父级 DOM/stacking context。拖拽手柄必须有上移/下移等价键盘操作，并与虚拟行挂载/卸载生命周期绑定。

Drag 的自动化 E2E 与视觉回归暂缓；本次仍测试 stable-ID 顺序命令、端点跟随和 drop 后重测量的契约，不将暂缓标为已通过。

### 4.7 Alignment 操作

选择模型：

- 左右两侧分别维护有序 selection；
- `Link` 用所选 Segment 创建一个 Alignment；
- 已属于其他 Alignment 的 Segment 不能被静默抢占，必须提示“替换现有关系”或先 Unlink；
- `Group` 合并选中的完整 Alignment / unlinked Segment 为一个新关系；
- `Unlink` 删除关系但保留 Segment；
- `Ungroup` 必须明确给出分组结果；左右数量相等时可建议 1:1，数量不等时默认拆成未对齐并让用户重新 Link，Kernel 不猜语义；
- 同侧 `Merge 内容` 只允许连续且全部 unlinked 或同一 Alignment 的 Segment；跨 Block 内容 Merge 必须先显式 Group，不允许隐式变更关系；
- `Split 内容` 无损创建后续 Segment IDs，所有 results 先继承原 Alignment；用户需分别对齐时再 Ungroup。

### 4.8 Search / Replace

搜索条件：查询词、普通文本/正则、区分大小写、语言侧、当前工程。结果显示 SegmentId、左右上下文、匹配高亮、AlignmentId/状态。

查找分为两个明确入口：

- `Ctrl+F`：Review View Find，只处理当前已打开 `ParallelWorkspace` 的中文/英文 Segment DTO，返回 stable SegmentId / AlignmentId 后交给 virtualizer 定位；不依赖当前 DOM 是否已经渲染；
- `Ctrl+Shift+F`：Kernel Project Search，查询 Document / Project 范围，通过 `KernelClient → Query → HitSet` 返回；
- View Find 只做大小写不敏感的普通文本匹配与上一处/下一处；项目级普通文本和基础 Regex 在 Rust Kernel 执行；
- 前端不得扫描 DOM 或把当前 View Find 冒充跨工程搜索。

替换流程：

```text
输入查询与替换文本
  → 计算只读命中集
  → 展示每条 before / after 预览
  → 用户勾选范围
  → 再校验 base revision
  → 单个 ChangeSet 原子提交
  → 增量更新索引
```

正则错误、命中集过期或任意一项写入失败时，整个批次不得部分提交。

### 4.9 Bookmark

- 默认锚定一个 Segment；
- 可选记录 AlignmentId，便于从复杂对齐组恢复上下文；
- `BookmarkView` 显示当前 Revision 派生的正文截断预览、语言、顺序号和关系摘要，不只显示 BookmarkId；canonical Bookmark 不复制正文；
- Segment Merge 将被吸收 Segment 的书签迁到保留首项；Split 后书签留在第一 part；Group/Ungroup/Unlink 仅在可唯一确定时更新 Alignment 导航 hint，否则清空 hint，Segment anchor 永不静默丢失；
- MVP 不强制实现 tags。

### 4.10 本地批注

批注字段：

```text
AnnotationId
body
status: Draft | InProgress | Resolved
linked_segment_ids[]
optional alignment_id
created_at / updated_at
local_author_label
revision
```

交互：正文上的编号标记、悬浮摘要、右侧列表、状态筛选、关联片段跳转、编辑、删除、标记已解决。批注是 sidecar layer；修改批注不改变 Segment 内容或 Alignment。

### 4.11 Autosave / History

历史分三层：

1. 当前编辑器临时状态：未提交文本，取消即丢弃；
2. 会话 Undo / Redo：针对已提交 Command；
3. 持久 Revision：跨重启可查看、比较和恢复。

保存策略：

- canonical Command 先通过校验，再追加 Operation Log，再更新当前 Revision；
- 自动保存采用短延迟合并刷盘，界面展示“保存中 / 已保存 / 保存失败”；
- 手动保存强制 flush 当前日志、manifest 和必要数据；
- 文本连续输入在确认保存时形成一个语义操作，不按每个字符生成 Revision；
- 恢复历史版本通过 `RestoreRevision` 创建新 Revision，保留被恢复前的版本；
- 差异第一版支持 Segment 内容、顺序、Alignment、Bookmark、批注的结构化摘要；文本展示行级 diff。

History 在产品上区分三种投影，但共享同一 `RevisionId`：

- Text History：一个 Segment 在不同 Revision 的文本变化；
- Alignment History：Link / Unlink / Group / Ungroup 关系变化；Text History 同时记录 Merge Segments / Split Segment 的内容和身份迁移；
- Project Operation Trace：Edit、Move、Bookmark、Annotation 等工程级操作时间线。

### 4.12 Parallel History Mode

- Segment 文本差异使用 `@codemirror/merge`，支持 unified 与 side-by-side 两种呈现；
- 默认使用低饱和淡红删除、淡绿新增，适配 Light / Eye Care 主题；
- Alignment、顺序、书签、批注使用结构化变更卡片，不强行转换为文本 diff；
- 选择两个 Revision 后再加载 Diff View，普通 Review 不预计算全项目 diff；
- “恢复此版本”提交 `RestoreRevision`，不是在前端接受若干 diff chunk 后直接覆写当前数据；
- CodeMirror 的 accept/reject chunk 只用于未来的交互增强，MVP 恢复仍以 Kernel Revision 为事务边界。

### 4.13 Export

- TXT：每个 Alignment 一行，左右同侧多 Segment 按配置分隔符连接；未对齐项以空侧输出并可选择是否包含；
- JSON：完整输出 ID、Segment refs、顺序、文本和 alignment cardinality；
- XML：表达 alignment、source refs、target refs 和文本；
- 导出前做一致性校验，并生成临时文件后原子替换目标，避免失败时留下截断文件；
- 导出不修改工程 Revision。

---

## 5. 数据模型与不变量

### 5.1 Canonical Data

| 对象 | 核心字段 | 不变量 |
|---|---|---|
| Project | ProjectId、名称、语言、设置引用 | 双语是产品限制，不是 Core 数据模型限制 |
| Document | DocumentId、ProjectId、LanguageId、SourceAssetRef | 每个 Segment 归属一个 Document |
| Segment | SegmentId、DocumentId、kind、content、revision | ID 不等于顺序或存储位置；Merge 保留有序首项，Split 保留第一 part 的原 ID |
| SegmentOrder | DocumentId、SegmentId、PositionKey | 重排只改顺序对象 |
| Alignment | AlignmentId、source refs、target refs、status | 引用至少一侧有 Segment；同一 Segment 默认最多属于一个 active Alignment |
| Bookmark | BookmarkId、SegmentId、可选 AlignmentId、label | 通过稳定 ID 锚定；内容预览是当前 Revision 的 derived view |
| HumanAnnotation | AnnotationId、正文、状态、links、revision | 是 sidecar layer，不嵌入 Segment struct |
| Revision | RevisionId、parent、ChangeSet、时间、摘要 | append-only；恢复也是新 Revision |
| SourceAsset | AssetId、路径/副本、编码、hash | 原始导入材料可追溯 |

### 5.2 Derived Data

- 规则分句预览；
- 基础字符串搜索索引；
- 搜索命中集与上下文；
- UI 对齐统计；
- 版本文本 diff；
- 缓存 Slice。

Derived Data 必须带 input revision 或 hash，可删除并重建，不进入正文历史。

### 5.3 UI State

- 当前模式、选中项、滚动位置；
- 面板开关、筛选条件；
- 编辑器中尚未提交的草稿；
- 临时拖动位置；
- 窗口尺寸。

UI State 不得写入 canonical 表；有价值的布局偏好进入用户设置。

### 5.4 Alignment 一致性校验

每次提交至少校验：

- 所有 SegmentId 存在且属于当前工程；
- source refs 与 target refs 语言侧正确；
- 同一 Alignment 内无重复 SegmentId；
- active Alignment 之间不存在未经确认的 Segment 重复占用；
- refs 顺序可按各自 SegmentOrder 解释；
- 删除/拆分/合并后的旧 Alignment 进入历史，不残留悬空 active 引用。

---

## 6. MVP 架构切片

### 6.1 固定技术栈

| 层 | 固定选型 | 责任与边界 |
|---|---|---|
| 桌面容器 | Tauri 2 | Windows 桌面壳、文件/目录对话框、菜单、系统窗口、更新与打包；P1 通过 `WebviewWindow` 承担辅助窗口 |
| Kernel | Rust stable | canonical data、校验、Command/Query、Revision、存储、搜索与导出；不把领域规则放进前端 |
| 前端框架 | Vue 3 Composition API + `<script setup>` | 所有页面和领域组件；不使用 Options API 混写 |
| 类型系统 | TypeScript strict | `strict`、`noImplicitAny`、显式 DTO；禁止在 IPC 边界传播 `any` |
| 构建 | Vite | 前端开发服务器与生产构建；按 Tauri 2 的 Vite 接入方式配置 |
| 包管理 | pnpm workspace | 管理 desktop、kernel-client-ts、ui-components；提交 `pnpm-lock.yaml` |
| 路由 | Vue Router | Project、Parallel、Search、Bookmarks、Annotations、Project History、Settings 顶层工作区；mode 不做成独立 route |
| 前端状态 | Pinia | 只保存 session、workspace、selection、panel、用户设置和轻量 Slice cache；Kernel 仍是 canonical source of truth |
| IPC | Tauri commands + events/channels，封装为 `KernelClient` | 小型控制 DTO 走 Command/Query；进度和变更走 Event/Channel；UI 不直接调用零散 Tauri command |
| 虚拟滚动 | `@tanstack/vue-virtual` | `ParallelViewport`、搜索结果、书签/批注/历史长列表；item key 必须使用 stable ID |
| 排序拖拽 | `@atlaskit/pragmatic-drag-and-drop` | element adapter 绑定虚拟行，提供拖动源、drop target 和插入边；决明只实现 stable ID 领域适配 |
| Segment 编辑器 | 原生 `textarea`（MVP） | 单 Segment 编辑、输入法、脏状态与快捷退出；CodeMirror 6 作为复杂编辑需求出现后的替换实现 |
| View Find | 自研轻量控制器 + TanStack Virtual 定位 | Review 下 `Ctrl+F`、上一处/下一处、stable AlignmentId 跳转；不承担 Kernel Project Search |
| Diff | Kernel 结构化 compare + 自研双栏 History 外壳 | MVP 展示 Segment / 顺序 / Alignment 结构化差异；CodeMirror MergeView 留作更复杂文本 diff 的可替换呈现层 |
| 无样式交互原语 | Reka UI | Dialog、Popover、Menu、Tooltip、Select、Tabs、焦点管理和键盘可访问性 |
| Dock | `dockview-vue` | P1 Context Pane 与可序列化辅助布局；不接管正文双栏核心布局 |
| 原生多窗口 | Tauri `WebviewWindow` | P1 “Open in New Window”；窗口间只共享 ID/Query，不复制可变 canonical state |
| 图标 | `lucide-vue-next` | 统一线性图标；产品 logo 作为自有资产 |
| 国际化 | `vue-i18n` | 中、英、法 UI 文案与格式，离线即时切换；不把双语语料内容交给 i18n 管理 |
| 样式 | CSS Custom Properties + Vue scoped CSS | 自有 design tokens、Light / Eye Care 主题；不引入带强视觉意见的大型成品 UI 框架 |
| 前端测试 | Vitest + Vue Test Utils | controller、store、composable、组件状态与无障碍单测 |
| UI/E2E | Playwright | 浏览器层主流程、键盘、截图回归；Tauri 安装包另做 Windows smoke test |
| Rust 质量门禁 | `cargo test`、rustfmt、Clippy | Core、Storage、Kernel、协议与故障恢复 |
| 前端质量门禁 | `vue-tsc`、ESLint、Prettier | 类型、Vue 规则、格式与 CI 门禁 |

版本策略：固定上述 major 技术路线；创建脚手架时选择相互兼容的当前稳定版本并立刻提交 `Cargo.lock` 与 `pnpm-lock.yaml`。CI 不使用浮动 `latest`，依赖升级通过单独 PR 和回归测试完成。

明确不采用：React、Electron、Nuxt/SSR、完整 DOM 正文、把 CodeMirror 当整篇语料容器、把 Pinia 当工程数据库、前端全库 Regex、为每个 Mode 单建页面。

### 6.2 运行形态

```text
Tauri 2 Desktop
      ↓
Vue 3 ParallelWorkspace / Project Workspaces
      ↓
TypeScript KernelClient
      ↓
Tauri Command / Query / Event / Channel
      ↓
Rust Kernel Runtime
      ↓
SQLite Catalog + Operation Log + Chunk/Slice Storage + Rebuildable Index
```

只交付本地桌面 App。UI 不直接知道 SQLite 表、Chunk offset 或 Rust 内部结构。

### 6.3 前端技术路线

前端遵循“领域交互自研、通用基础设施复用”的边界。

必须自研：

| 模块 | 责任 |
|---|---|
| `ParallelWorkspace` | 四种 mode 的统一宿主、工具栏、selection、anchor、panel 协调 |
| `ParallelViewport` | 双栏可变高度虚拟列表、Alignment group 渲染、稳定 ID 定位 |
| `SegmentView` | 同一 Segment 在 Review/Edit/Order/History 下的视觉与交互差异 |
| `AlignmentViewportController` | 按 AlignmentId 双向跟随、定位、anchor 保持、scroll loop 抑制 |
| `ViewModeController` | mode 进入/退出守卫、草稿处理、可用 command 与焦点策略 |
| `ContextLensController` | inline expand、Context Pane、新窗口三级上下文的统一 query 语义 |
| `CorpusQueryView` | Project Search 结果、平行上下文、HitSet 分页与跳转 |
| `AlignmentCommandBar` | Link、Unlink、Group、Ungroup 的 Alignment selection 解释与命令提交 |
| `SegmentStructureCommandBar` | 同侧 Merge 内容 / Split 内容的 Segment selection、无损预览与命令提交；不得复用 Alignment selection |
| `HumanAnnotationLayer` | gutter 标记、关联关系、批注侧栏与正文定位 |
| `HistoryWorkspace` | Revision 选择、Text/Alignment/Operation 三种历史投影 |

直接复用：

| 基础能力 | 库/平台 |
|---|---|
| 单句文本编辑 | 浏览器原生 `textarea`（MVP）；CodeMirror 6 作为可替换升级路径 |
| Review 当前 View 查找 | 轻量输入控件 + `@tanstack/vue-virtual` stable ID 定位 |
| 文本 Diff | Kernel compare + 当前 History 双栏呈现；`@codemirror/merge` 延后 |
| 长列表虚拟化 | `@tanstack/vue-virtual` |
| 排序拖拽 | `@atlaskit/pragmatic-drag-and-drop` |
| Dialog/Menu/Popover/Tooltip | Reka UI |
| Context Dock | `dockview-vue`（P1） |
| 原生窗口 | Tauri `WebviewWindow`（P1） |
| 路由与 UI 状态 | Vue Router + Pinia |

建议目录：

```text
apps/desktop/src/
  app/                         # bootstrap、router、providers、app shell
  workspaces/
    parallel/
      ParallelWorkspace.vue
      modes/                   # review/edit/order/history presentation
      viewport/                # ParallelViewport、SegmentView、AlignmentGroup
      controllers/             # mode、scroll、context、selection
      panels/                  # annotation、bookmark、context、history
    project/
    search/
    settings/
  stores/                      # session/workspace/selection/panel/settings
  kernel-client/               # typed façade、DTO adapter、event subscriptions
  ui/                          # 基于 Reka UI 封装的决明 primitives
  styles/                      # tokens、themes、typography、motion
  test/
```

组件不得直接 `invoke()` Tauri。调用链固定为：

```text
Vue Component
  → domain controller / Pinia action
  → KernelClient
  → Tauri command/query
  → Rust Kernel
  → typed response/event
```

### 6.4 前端状态与滚动模型

Pinia 只保存可丢弃或可重建的前端状态：

```text
SessionStore       当前工程句柄、连接/保存状态
WorkspaceStore     mode、active AlignmentId、visible Slice handle
SelectionStore     source/target selections、keyboard focus
PanelStore         annotation/context/history panel state
SettingsStore      theme、font、scale、shortcut preferences
```

Segment 全文、Alignment 真值、Revision 历史不得镜像成一个长期可变 Pinia 大数组。Slice DTO 使用稳定 ID，Event 到达后按 revision 失效并重新 Query。

虚拟化与同步规则：

- `@tanstack/vue-virtual` 的 item key 使用 `AlignmentId` 或稳定的 display-row key，禁止 index key；
- 可变高度 Segment 使用测量缓存；切换 mode 时按内容/字号变化精确失效；
- 左右两侧不各自实现一个互相监听的 scroll handler；只允许 `AlignmentViewportController` 发起语义定位；
- 用户主动滚动优先，程序跟随带 source token，避免左右滚动反馈回路；
- inline prepend/append Context 时保持当前 Alignment anchor 的视觉 offset；
- 跳转先向 Kernel 请求目标附近 Slice，再调用 virtualizer 定位，不假设目标已在 DOM。

### 6.5 设计系统与视觉原则

- 颜色使用 CSS tokens：白色主体、淡绿 source/current、淡黄 target/current、深绿 action；
- Diff 使用低饱和淡红删除、淡绿新增，不直接复刻高饱和 GitHub 配色；
- Review 强调“当前读到哪里”，Order 强调“结构对象等权”，二者不得只换一个工具栏按钮；
- 字体、行高、列宽、gutter、连接区、批注标记均由 design token 控制；
- motion 只用于定位和 mode transition，遵守 reduced-motion；
- Reka UI 提供语义、焦点与键盘行为，决明负责外观；
- 连接线是辅助信息，不应覆盖文本或成为理解 Alignment 的唯一手段；
- 空状态、未对齐、保存失败、Slot UNBOUND、历史不可比较等状态必须有文本说明，不能只靠颜色。

### 6.6 建议的最小仓库边界

实施时先建立较小、可演进的 workspace，不照搬完整平台的所有空 crate：

```text
apps/
  desktop/                 # Tauri 壳与前端
crates/
  jueming-core/            # ID、Segment、Alignment、Revision、Command
  jueming-storage/         # catalog、project layout、chunk/slice、oplog
  jueming-kernel/          # command/query、校验、事务、事件、历史
  jueming-protocol/        # UI 可见 DTO 与本地合同
packages/
  kernel-client-ts/        # TS 客户端接口
  ui-components/           # Reka UI wrapper、tokens、可复用视觉组件
schemas/                   # Slot、持久格式、导入导出 schema
docs/
  adr/
  plans/
tests/
  fixtures/
  e2e/
benchmarks/
```

当且仅当外部插件、Server 或高级分析进入下一阶段，再拆出 plugin-runtime、rpc transport、nlp、index 等独立模块。

### 6.7 依赖方向

```text
jueming-core
    ↑
jueming-storage / jueming-protocol
    ↑
jueming-kernel
    ↑
kernel-client-ts
    ↑
desktop UI
```

禁止反向依赖：Core 不依赖 UI、Tauri、SQLite 具体驱动或插件运行时；UI 不依赖数据库 schema。

### 6.8 本地协议

MVP 的 Operation RPC / Data RPC 是逻辑边界，不要求网络化；在桌面端具体映射为 Tauri commands、events 和 channels：

- Command：创建/修改 canonical data；
- Query：读取 Project、Slice、SearchResult、History；
- Event：RevisionAdvanced、SegmentChanged、AlignmentChanged、SaveStateChanged、IndexUpdated；
- Data View：按 Segment 范围与投影字段读取，不返回整库可变对象。

未来传输替换为 Local IPC 或网络协议时，UI 语义保持不变。

### 6.9 Slot Profile

MVP 绑定：

```text
source.text.parse              BOUND: builtin-txt
source.clipboard               BOUND: builtin-clipboard
content.segment.sentence       BOUND: builtin-rule-segmenter
relation.alignment.manual      BOUND: builtin-manual-alignment
index.basic_string             BOUND: builtin-basic-index
analysis.basic_search          BOUND: builtin-basic-search
export.txt                     BOUND
export.json                    BOUND
export.xml                     BOUND
```

MVP 注册但不绑定：

```text
source.subtitle.parse
source.ocr
source.audio.transcribe
layout.reading_order
segment.tokenize
segment.embedding
segment.translate
token.pos
token.lemma
token.ner
token.dependency
relation.alignment.auto
relation.word_alignment
relation.terminology
index.lexical
index.vector
analysis.kwic
analysis.collocation
analysis.wordcloud
analysis.quality
```

Human Annotation 是内建业务 layer，不与 `token.pos` 等 NLP Slot 混淆。

### 6.10 持久化布局

建议工程目录：

```text
project-name.jm/
  manifest.json               # 格式版本、工程 ID、当前 Revision
  catalog.db                  # 元数据、稳定 ID 映射、书签、批注、revision 索引
  chunks/                     # canonical 文本块
  overlays/                   # 编辑增量，达到阈值后 compaction
  revisions/                  # snapshot manifest / changeset refs
  indexes/basic-string/       # 可重建索引
  assets/                     # 可选托管的导入源文件副本
  cache/                      # 可安全删除
```

第一版可以使用简单 Chunk 后端，但 API 必须按 Slice 读取；不得把“加载整个工程到一个前端数组”固化成客户端合同。

---

## 7. Command、Query 与 Event 清单

### 7.1 Commands

```text
CreateProject
ImportTextAsset
ApplySegmentation
UpdateSegment
MoveSegment
MergeSegments
SplitSegment
CreateAlignment
DeleteAlignment
GroupAlignment
UngroupAlignment
AddBookmark
RemoveBookmark
CreateHumanAnnotation
UpdateHumanAnnotation
ResolveHumanAnnotation
DeleteHumanAnnotation
ReplaceMatches
Undo
Redo
FlushProject
RestoreRevision
```

每个 Command 包含 `project_id`、`base_revision`、`command_id` 和必要参数。所有批量命令采用事务语义。

### 7.2 Queries

```text
GetProjectSummary
LoadParallelSlice
GetSegment
GetAlignment
ListUnlinkedSegments
SearchSegments
PreviewReplace
ListBookmarks
ListHumanAnnotations
ListRevisions
DiffRevisions
GetSlotStates
ValidateExport
```

### 7.3 Events

```text
ProjectOpened
RevisionAdvanced
SegmentChanged
SegmentOrderChanged
AlignmentChanged
BookmarkChanged
HumanAnnotationChanged
SearchIndexUpdated
SaveStateChanged
OperationFailed
SlotStateChanged
```

UI 的 domain store 只响应 DTO 与 Event，不复制 Kernel 规则。

---

## 8. 实施阶段、依赖与退出条件

### Phase 0：工程决策与合同冻结

产出：

- workspace 与模块边界；
- ID、Segment、Alignment、Revision、HumanAnnotation 的 schema；
- Command / Query / Event v1；
- Tauri 2 / Vue 3 workspace、`KernelClient` façade 与前端目录边界；
- `ParallelWorkspace` 四模式状态机、进入/退出守卫和 scroll anchor 合同；
- 项目目录格式 v1；
- Slot 名称与状态表；
- ADR；
- 测试 fixture 与性能基线方案。

退出条件：核心对象和跨层 DTO 已评审；没有 UI 直接依赖存储布局的接口。

建议首批 ADR：

1. Tauri 2 + Rust Kernel + Vue 3 Composition API + TypeScript strict；
2. Stable ID 与物理位置分离；
3. Segment 是 canonical unit；
4. SegmentOrder 独立；
5. Alignment 只引用 Segment；
6. Human Annotation 是 sidecar layer；
7. Append-only Revision 与恢复语义；
8. Chunk / Slice 本地存储边界；
9. in-process Operation/Data contract；
10. UNBOUND Slot 是合法状态；
11. ParallelWorkspace 四模式共享数据与控制器；
12. View Find 与 Kernel Project Search 分离；
13. Order 空位使用原子关系重建；
14. Segment 内容结构与 Alignment 分组解耦。

### Phase 1：Project、Import、Segmentation、Persistence

实现状态（2026-08-28）：核心垂直链路已实现并通过真实桌面验证。Tauri 文件选择器可分别选择原文/译文，Rust Kernel 支持 UTF-8、UTF-8 BOM、Windows 严格 GB18030、非空行/规则分句/SISU 标记行 profile；创建工程会生成稳定 UUIDv7 ID、暂定 1:1 布局与原子写入的 `.jm/project.json`，打开、显式保存、编辑自动保存、顺序变更自动保存均经过 Tauri `KernelClient` 边界并追加 Revision。政府报告 fixture 的 create/open、编辑重开、重排稳定 ID 与多余目标段未链接均已有单元测试；真实“阿古顿巴”已从 UI 导入为 source 307、target 308、307 个 Alignment 和 1 个 target unlinked，并在关闭应用后从 `.jm` 文件夹重开成功。

实现：Tauri 2 + Vue 3 应用壳、KernelClient、工程创建/打开、TXT/Paste、UTF-8/BOM/GB18030 解码预览、普通行/规则分句/旧版标注行三种导入 profile、稳定 ID、初始布局、工程目录、保存与重开、TanStack Virtual 最小列表。

依赖：Phase 0。

退出条件：给定双语 fixture，导入、关闭、重开后 Segment 内容、ID、顺序和 provisional 对齐完全一致。

### Phase 2：Parallel Review 与人工 Alignment

实现状态（2026-08-29）：既有 Link / Unlink 与复杂关系已实现；本迭代已冻结 `Group / Ungroup` 取代旧 Alignment Merge/Split 的合同，Segment Merge/Split 作为独立内容结构线进入实现。必须为旧工程和历史提供兼容读取/显示迁移，不能把旧名称继续作为新写入语义。

实现：`ParallelWorkspace`、`ParallelViewport`、`AlignmentViewportController`、双栏 Slice 加载、可变高度虚拟滚动、独立 Segment/Alignment selection、AlignmentId 双向定位、inline Context Lens、未对齐状态、Link / Unlink / Group / Ungroup、对齐校验。

依赖：Phase 1。

退出条件：1:1、1:2、2:1、2:2、n:m 均可创建、显示、保存、重开；Group/Ungroup 旧 ID 失活和新 ID 分配可追溯；非法重复占用被拒绝且不污染 Revision。

### Phase 3：Edit、Order、Undo / Redo

实现状态（2026-08-29）：既有显式 `ViewModeController`、编辑会话、稳定 ID 重排、上移/下移与持久 Undo/Redo 已实现。新增 Segment Merge/Split 需实现无损 part 校验、首项身份保留、Order 插入与 sidecar 迁移；Order Overlay 连线跟随和 z-index 规范已冻结。自动 drag E2E/视觉回归按本迭代决定暂缓，不能误报为已完成。

实现：原生 `textarea` 单句编辑、自动保存、保存退出、放弃退出、Review `Ctrl+F` 当前 View Find、stable ID 虚拟定位与 highlight 动画、MergeSegments/SplitSegment、上移/下移/拖拽/基线恢复、`ViewModeController`、Command 逆操作、会话撤销重做。

依赖：Phase 2。

退出条件：连续执行编辑、移动、Segment Merge/Split、Group/Ungroup 后可逐步 Undo 到初态并 Redo 到末态；保留/新建/失活 ID 及 sidecar 迁移均可在历史中验证。

### Phase 4：Search / Replace、Bookmark、单机批注

实现状态（2026-08-29）：搜索、替换、Bookmark CRUD 与 HumanAnnotation CRUD/Resolve 已实现。本迭代增加 BookmarkView 内容预览和 Segment/Alignment 结构操作的锚点迁移要求；在迁移和历史测试完成前，不宣称结构操作下的书签/批注维护已完成。

实现：`Ctrl+Shift+F` Kernel Project Search、基础索引、普通/大小写/正则搜索、`CorpusQueryView`、平行上下文跳转、替换预览与原子提交、含内容预览的书签、批注 gutter 和侧栏、结构操作 sidecar migration。

依赖：Phase 3 的 ChangeSet 和 Slice。

退出条件：替换、书签、批注均可持久化和撤销；重排、Segment Merge/Split、Group/Ungroup 后所有锚点仍定位正确，书签预览与请求 Revision 一致。

### Phase 5：Autosave 与持久 History

实现状态（2026-08-28）：所有 canonical 写操作均原子保存当前 snapshot 与对应 Revision snapshot；已实现 Revision 列表、结构化 compare、Undo、Redo、Restore-as-new-revision 和双栏 History。编辑自动保存默认停止输入 3 秒后提交，设置中可选 1 / 3 / 5 / 10 / 30 秒；一次 debounce 提交一个完整 `UpdateSegment` Revision，不按字符生成历史。工程格式 v0.1 采用 `.jm/project.json + revisions/*.json`，由 `jueming-storage` 隔离；canonical `revisions/` 在 MVP 永久保留，不设置自动删除。可重建 `.jm/cache` 可按每次启动 / 每 7 天 / 每 30 天 / 从不清理，并支持手动清理；Storage 测试保证清理仅触及 cache，不删除 `project.json` 或 revision snapshots。真实工程已验证 Edit → 自动保存 → Undo、Move → Reset、Unlink → Undo、关闭 → 重开和 R1↔当前版本 Diff。故障安全由同目录临时文件 + 原子替换和重开测试覆盖，强制终止故障注入仍归 Phase 7。

实现：debounced Revision 提交、手动 flush、安全关闭前 flush、原子 snapshot、Text/Alignment/Project History 投影、双栏 diff、恢复为新 Revision、保存状态 UI、派生缓存安全清理。

依赖：Phase 1–4 的全部 canonical Command。

退出条件：在故障注入点强制终止进程，工程仍能打开到最后完整提交；恢复旧版本不删除后续历史。

### Phase 6：Export、Settings 与视觉收口

国际化补齐（2026-08-31）：采用 Vue I18n Composition API 和 Vite 词典预编译，覆盖中、英、法应用壳、导入、共享工作区、搜索替换、书签、批注、历史、设置及已知 Kernel 诊断。首次启动按系统语言选择，设置页即时切换并保存在本机；不重建编辑会话、不写入 canonical 数据。Windows NSIS/MSI 配置三语安装包，确认框使用翻译按钮，未知系统诊断保留原文。三语词典与交互回归检查纳入 CI；维护说明见 [国际化指南](docs/development/localization.md)。

实现状态（2026-08-28）：TXT/JSON/XML 原子导出、Light/Eye Care、界面缩放、侧栏折叠、自动保存延迟、缓存清理策略、离线状态、空状态、错误反馈和主效果图结构均已接入。设置页新增自动 / macOS / Windows-Linux 快捷键布局、当前键位表与触控板优化开关；macOS 自定义窗口栏采用左侧 traffic-light 排列。触控板滚动会立即取消尚未完成的跳转 rAF，系统“减少动态效果”会直接定位。Windows 文件对话框已完成导入、打开与 TXT 导出，导出首行验证为 `1:1\t阿古顿巴\tAkhu Tenpa [verified]`；前端 production build、Tauri debug 构建与严格类型检查通过。Review、Edit、Order、Search、Annotation、History、Settings 七个状态已分别完成桌面端检查；真实 307×308 工程已验证快速查找跳转与 highlight、双击编辑与 Escape 退出、30 秒草稿守卫、自动保存形成 Revision、源侧顺序持久化与基线恢复、派生缓存清理不删除历史。静态截图未展示拖拽悬浮态，以及延期 POS 覆盖层是已记录的有意差异，不冒充像素级一致。

双平台打包状态（2026-08-28）：根脚本固定 Windows `NSIS + MSI` 与 macOS `Universal app + DMG`。GitHub Actions 在 `main` push / PR 执行 TypeScript 与 Rust 质量门禁，手动运行或 `v*` 标签在门禁后打包 Windows x64 和 macOS Universal；Universal 同时包含 `aarch64-apple-darwin` 和 `x86_64-apple-darwin`。Actions 依赖锁到官方主版本对应的完整提交 SHA、权限收敛为只读，并按平台/架构/包类型上传 workflow artifacts。macOS 最低版本固定为 11.0，测试产物采用 ad-hoc 签名，正式分发再由 CI secret 覆盖为 Developer ID 并 notarize。用户提供的花朵图标已保真清除点阵画布，形成透明 1024×1024 品牌母版，并生成 Tauri Windows/macOS/PNG 全套资源及网页 favicon。ARM 热点审计已移除查找逐键全文归一化、为 rAF 设置 420 ms 上限和用户输入取消、把递归缓存清理移入 Rust blocking worker；没有空闲轮询、无限动画或常驻 GPU hint。真实 Apple Silicon 温升与能耗仍必须作为 Phase 7 硬件门禁执行，不能由跨平台 CI 推断。

实现：TXT/JSON/XML、导出校验、字体/字号/主题/UI 缩放、vue-i18n 文案、快捷键、空状态、错误状态、design tokens、与效果图一致的主布局。

依赖：稳定数据模型与全部操作。

退出条件：三种格式 round-trip / schema 测试通过；核心页面在目标窗口尺寸下无截断，键盘主路径可用。

### Phase 7：Hardening 与发布候选

实现：大 fixture、性能 profiling、内存预算、迁移测试、安装包、日志脱敏、恢复演练、验收回归。

退出条件：全部 P0 验收通过，无数据损坏级缺陷，无阻断级可访问性问题，形成可重复构建的发布候选。

---

## 9. 测试策略

### 9.1 Core 单元与性质测试

- 稳定 ID 不随编辑、移动变化；
- Segment Merge/Split 的无损文本、首项 ID 保留、后续 ID 生成、Order 与 sidecar 迁移不变量；
- Alignment Group/Ungroup/Link/Unlink 的前后不变量、旧 ID 失活与新 ID 不复用；
- 跨 Alignment Block Move 只改顺序，跨关系内容 Merge 被拒绝或显式 Group 后原子完成；
- PositionKey 在大量插入和重排下保持有序；
- ChangeSet inverse 正确；
- RestoreRevision 只追加、不回写旧 Revision；
- 批量替换全成或全败；
- HumanAnnotation 与 Bookmark 的锚点有效性、结构迁移和 BookmarkView 内容预览。
- 十种 LTR 语言白名单、BCP-47 代码和 RTL 拒绝；语言默认分句提示不改变编码/分段 profile 真值。

### 9.2 Storage / Recovery

- 创建、保存、关闭、重开 round-trip；
- 每类 Command 写入前、日志写入后、manifest 更新前后的故障注入；
- 损坏 cache 或 basic index 后自动重建；
- 工程格式版本迁移与未知版本拒绝；
- Save As 不共享可变工程 ID 或错误引用原路径。

### 9.3 UI 集成与 E2E

- Vitest + Vue Test Utils 覆盖 store、controller、composable 与 mode 状态机；
- Playwright 覆盖浏览器层主流程、键盘操作与视觉回归；
- 从新建到导出的完整 happy path；
- 复杂 1:n 与 n:m 对齐；
- 双向定位、可变高度虚拟滚动、inline Context prepend/append、anchor 与焦点恢复；
- Review/Edit/Order/History 切换不丢 selection、anchor 或未提交草稿提示；
- 编辑取消/保存、搜索跳转；drag 自动化 E2E/视觉测试暂缓，另以 stable-ID 端点/Overlay/drop 重测量集成契约覆盖；
- `Ctrl+F` 不触发 Project Search，`Ctrl+Shift+F` 不扫描 DOM；
- 正则错误、无结果、过期替换预览；
- 批注筛选与版本恢复；
- 快捷键冲突和输入法场景；
- 高 DPI、125%/150% 缩放、窄窗口、长中英文文本。

### 9.4 建议基准 fixture

- Tiny：8+8 Segment，用于示例与截图回归；
- Small：1,000+1,000 Segment，用于常规 E2E；
- Medium：50,000+50,000 Segment，用于虚拟列表、搜索、保存；
- Stress：500,000 总 Segment，用于后端 Slice、索引和内存观察，不作为所有 UI CI 的必跑项。

### 9.5 发布前性能门槛（在基准机器上冻结）

Phase 0 记录基准机器配置后冻结具体数值。建议初始目标：

- 已建索引的 100,000 Segment 普通文本搜索 P95 小于 1 秒；
- 可见 Slice 的编辑提交到界面更新 P95 小于 100 毫秒；
- 滚动过程中不把全部 Segment materialize 到 DOM；
- 自动保存不阻塞输入主线程；
- 重新打开工程的耗时随热元数据和首屏 Slice 增长，不随全量正文线性 materialize；
- Stress fixture 的 resident memory 明显小于工程磁盘体积。

---

## 10. 验收场景

### A. 完整人工对齐

导入 8 条中文和 9 条英文，预览分句；初始形成 8 个 provisional 1:1 和 1 个未对齐英文 Segment；将其中一组改为 1:2，编辑一个英文 Segment，移动一个中文 Segment，保存并重开。所有内容、ID、顺序和对齐关系必须保持。

### B. 搜索与替换

搜索一个在两侧多次出现的词，限定英文侧，使用大小写选项，预览后选择部分结果替换。替换必须形成单个 Revision；一次 Undo 恢复全部所选结果，未选项不变。

### C. 批注

给一个 1:2 Alignment 添加 Draft 批注，关联一条中文和两条英文 Segment；改为 In Progress，最后 Resolved。重排 Segment 后批注标记和侧栏跳转仍指向原对象。

### D. 自动保存与恢复

完成编辑、重排、对齐和批注后等待自动保存，在测试故障点终止进程。重开后到达最后完整 Revision；不完整事务不可见。选择旧版本恢复后产生新 Revision，恢复前版本仍可查看。

### E. 复杂导出

工程同时包含 1:1、1:2、2:1、2:2、n:m 和未对齐项。TXT 按已声明规则输出；JSON/XML 无信息损失；导出失败时目标文件不被截断。

### F. 真实“阿古顿巴”语料兼容

以根目录两个用户提供文件作为只在本地运行的兼容性验收：中文侧显式选择 GB18030，英文侧使用 UTF-8，在导入预览选择“旧版标注行” profile。原始 Asset 必须原样保留；预览应能剔除 `<seg>` / `</seg>` 包装、英文 POS 后缀与中文字间空格，过滤空标记后预期形成 307 个中文 Segment、308 个英文 Segment、307 个 provisional 1:1 和 1 个 target unlinked。导入、保存、重开、搜索与导出全链路不得出现乱码或丢失稳定 ID。

---

## 11. 风险与控制

| 风险 | 早期信号 | 控制方式 |
|---|---|---|
| 把完整平台架构一次性实现 | Phase 1 出现插件、云、NLP crate | 只实现本地合同和 Registry；按 MVP Slot Profile 审查 |
| UI 行号成为数据身份 | 重排后批注/书签错位或拖动连线错位 | API、端点注册与 Overlay 只接受 stable ID 和 before/after 关系 |
| Alignment 规则分散在前端 | UI 与重开结果不一致 | 所有校验和提交集中在 Kernel |
| 自动保存导致历史噪声 | 每个字符一个 Revision | 编辑会话合并为语义 Command；持久版本按 ChangeSet 聚合 |
| 历史恢复破坏未来版本 | 直接覆盖数据库状态 | Restore 永远创建新 Revision |
| 批量替换部分写入 | 中途失败后工程半更新 | base revision 校验 + 单事务 ChangeSet |
| 大文本卡死 UI | 前端持有全工程数组/DOM | Slice Query + virtual list + 后台索引 |
| 左右滚动形成反馈回路 | 两侧监听器互相触发、视图抖动 | 唯一 AlignmentViewportController + source token + 用户滚动优先 |
| Pinia 变成第二数据库 | store 中出现全工程可变 Segment/Alignment 数组 | Pinia 只存句柄、Slice、selection 和 UI state；Kernel 事件驱动失效 |
| 四种 Mode 各写一套页面 | 修复一个模式时其他模式行为漂移 | 统一 ParallelWorkspace、SegmentView 与 controllers，mode 只改变呈现和权限 |
| 过度依赖通用 Dock/DnD | Alignment anchor 和 stable ID 被 layout index 取代 | Dock 仅做辅助 Pane；重排提交显式 domain command |
| 批注与 NLP annotation 混淆 | Schema 使用同名模糊字段 | 使用 `HumanAnnotation` 独立 schema 和命名空间 |
| 工程格式过早锁死 | SQLite 表直接暴露给 UI | manifest 格式版本 + storage abstraction + DTO |
| 效果图被误当成所有细节已定 | 开发期出现大量隐含交互争议 | Phase 0 补齐状态图、错误态和命令语义，再进入实现 |
| 真实语料被当成 UTF-8 普通文本 | 中文乱码、`<seg>`/POS 标记进入正文、左右段数误判 | 解码与标注清理均必须先预览再原子应用；保留原始 Asset 和导入 profile 元数据 |

---

## 12. Definition of Done

MVP 只有在以下条件全部满足时才算完成：

1. 第 3.1 节所有 P0 功能通过验收；
2. 新建、导入、人工对齐、编辑、重排、搜索替换、批注、保存、恢复、导出构成无断点闭环；
3. 1:1、1:n、n:1、n:m、unlinked 均能保存、重开、撤销和导出；
4. Stable ID、SegmentOrder、Alignment、Revision 不变量由自动测试保护；
5. 崩溃恢复测试证明不会暴露半提交 Revision；
6. Medium fixture 下 UI 使用虚拟列表，搜索和编辑满足冻结后的性能门槛；
7. POS、自动对齐、KWIC、OCR、云协作等延期功能没有被偷偷实现，也没有污染人工闭环；
8. 所有延期能力在 Slot Registry 中有稳定名称和明确 `UNBOUND` 状态；
9. 三种导出格式通过复杂 Alignment fixture；
10. `ParallelWorkspace` 四模式共享同一 stable-ID ViewModel，切换不复制 canonical data；
11. View Find、Project Search 与虚拟列表边界通过自动测试；
12. 发布包可离线安装运行，工程数据默认仅保存在用户选择的本地位置。
13. 政府报告 Tiny fixture 通过可重复的视觉回归，根目录“阿古顿巴”双语文件通过 GB18030/UTF-8 与旧版标注行兼容链路的 Computer Use 验收。

---

## 13. 开工前检查清单

实现期间按关键节点完成并提交：

- [ ] 确认许可证（产品名 `决明对齐器 Jueming Aligner`、应用 ID `com.jueming.aligner`、工程扩展名 `.jm` 已冻结）；
- [ ] 确认目标 Windows 最低版本与安装包形式；
- [x] 冻结前端技术路线：Tauri 2 + Vue 3 + TypeScript strict + 第 6.1 节基础库；
- [x] 在脚手架创建日选择兼容的精确版本并提交 Cargo/pnpm lockfile；
- [x] 完成 Phase 0 的十二项 ADR；
- [x] 冻结 Project Format v1 与 Command/Query/Event v1；
- [x] 冻结 ParallelWorkspace mode 状态机、Alignment anchor 与 Context Lens 合同；
- [x] 把七张效果图拆成页面、组件、状态和交互验收清单；
- [ ] 为 8+8、1,000+1,000、50,000+50,000 建立合法 fixture；
- [ ] 冻结 Alignment split 的交互规则和 TXT 多段连接符；
- [x] 确定自动保存延迟、Revision 聚合窗口和日志保留策略：默认 3 秒，可选 1/3/5/10/30 秒；一次 debounce 一个完整 Revision；canonical Revision 永久保留，只有 `.jm/cache` 按策略清理；
- [ ] 确定替换正则语义、大小写规则与 Unicode 行为；
- [ ] 建立 CI、格式化、静态检查、单测、E2E 和打包门禁；
- [ ] 完成数据损坏与崩溃恢复演练方案；
- [x] Phase 0 基线文档自检完成后创建实现脚手架。

---

## 14. 技术选型依据

- [Tauri 2](https://v2.tauri.app/)：固定桌面容器主线；[Vite 接入](https://v2.tauri.app/start/frontend/vite/)用于 Vue 前端构建，[前端调用 Rust](https://v2.tauri.app/develop/calling-rust/)作为 KernelClient 的底层桥接依据；
- [Vue 3 TypeScript Composition API](https://vuejs.org/guide/typescript/composition-api)：固定 `<script setup lang="ts">` 与严格类型组件路线；
- [Pinia](https://pinia.vuejs.org/introduction.html)：用于跨组件 UI / workspace state，不承担 canonical data；
- [Vue Router](https://router.vuejs.org/introduction.html)：只管理顶层工作区，Parallel mode 留在工作区内部；
- [TanStack Virtual Vue adapter](https://tanstack.com/virtual/latest/docs/installation)：用于可变高度正文和长结果列表；[Virtualizer API](https://tanstack.com/virtual/latest/docs/api/virtualizer)规定动态尺寸优先 `measureElement` 与 `ResizeObserver`，同一 index 不混用 `resizeItem`；
- [Pragmatic drag and drop core](https://atlassian.design/components/pragmatic-drag-and-drop/core-package/)：用于虚拟行的轻量拖动源、drop target 与 monitor；[Virtualization recipe](https://atlassian.design/components/pragmatic-drag-and-drop/core-package/recipes/virtualization)规定原 draggable 卸载后的稳定接续；[Design guidelines](https://atlassian.design/components/pragmatic-drag-and-drop/design-guidelines)给出至少 24px handle 等交互约束；
- [Vue Teleport](https://vuejs.org/guide/built-ins/teleport)：将 DragOverlay 放到 workspace 根层，避开父级 overflow/transform 的 stacking context；
- [Unicode CLDR](https://cldr.unicode.org/)：维护 MVP curated top-ten LTR 语言的语言/书写系统/方向依据；实际写入使用 BCP-47 基础代码。
- [CodeMirror Reference](https://codemirror.net/docs/ref/)：保留为复杂多行编辑、编辑器内替换或 MergeView 的升级路径；MVP 单句编辑不强制引入；
- [Reka UI](https://reka-ui.com/docs/overview/introduction)：提供无样式、accessibility-first 的 Vue primitives；
- [Dockview Vue](https://dockview.dev/docs/overview/introduction/)：P1 Context Pane 的 Dock、布局序列化与 Vue 3 adapter；
- [Vitest](https://vitest.dev/guide/)：Vite 原生前端测试与组件/浏览器测试基础。

本计划是实现基线；代码已获准按 Phase 1–7 进入仓库，但不得越过 Phase 0 合同与数据不变量直接拼接页面。
