# 决明对齐器 MVP 视觉规范

> 文档版本：v0.1  
> 状态：视觉实现基线 / 截图验收基线  
> 适用范围：Tauri 2 Windows 单机 MVP  
> 依据：`MVP效果图/` 七张 PNG、`jueming-aligner-mvp-implementation-plan-v0.2.md`、`jueming-aligner-mvp-functional-spec-v0.1.md` 及前端路线历史对话  
> 说明：本文件是对静态示意图的实现约束，不将示意图中未纳入 MVP 的功能误列为已实现功能。

## 1. 视觉目标与边界

决明对齐器的视觉目标是“安静、可扫描、关系清楚的双语工作台”：白色工作底、深绿操作色、淡绿原文/当前态、淡黄译文/当前态，所有重要关系以稳定的 `AlignmentId` 锚点表达。界面应让用户同时感知三件事：当前正在看的对齐组、两侧文本的对应关系、当前工程是否已保存。

实现只建立一套 `ParallelWorkspace`，由 `Review`、`Edit`、`Order`、`History` 四种模式改变呈现和操作权限。搜索、批注和工程历史可以作为路由工作区或辅助面板，但不复制另一套正文数据模型。正文、滚动、选择和上下文都以稳定 ID 为锚，不能以 DOM 行号或数组下标作为身份。

MVP 视觉范围：

- 双语导入后的双栏平行视图；
- 当前对齐组、高亮、连接区及未对齐状态；
- 单句编辑卡片；
- 句序调整工具栏、拖拽态和键盘等价操作；
- 工程级搜索与替换结果、预览区；
- 单机批注的正文标记、悬浮卡和右侧侧栏；
- 自动保存、版本列表、结构摘要和文本 Diff；
- 新建/打开/保存/导出/撤销/重做/设置等壳层状态。

明确延期：POS/lemma/NER/依存句法、自动语义对齐、高级 KWIC/OCR/字幕/音频、云协作及插件运行时。`暂时不实现-词性EOS.png` 只作为未来语言学覆盖层的视觉参考，不能作为 MVP 截图验收目标，也不得在主路径放置伪可用按钮。

## 2. 证据与参考画布

### 2.1 逐图证据清单

下表中的文件均已用原始分辨率实际查看。七张图尺寸均为 1448×1086 像素，属于同一桌面窗口基线。

| ID | 文件 | 视觉状态 | MVP 用途 |
|---|---|---|---|
| V1 | [平行阅读.png](../../MVP效果图/平行阅读.png) | Review，000104 当前对齐组，书签在 000103 | 主正文、默认模式、当前锚点验收 |
| V2 | [平行修改.png](../../MVP效果图/平行修改.png) | Edit，000104 双侧编辑卡片 | 编辑态、保存/取消、字数和拼写入口验收 |
| V3 | [语句重排.png](../../MVP效果图/语句重排.png) | Order，拖拽 04 句并显示插入槽 | 句序工具、拖拽反馈、关系降权验收 |
| V4 | [搜索与替换.png](../../MVP效果图/搜索与替换.png) | Search，查询“疫情”、结果表和下方预览 | 查询/替换、命中高亮、回跳验收 |
| V5 | [自动保存与历史.png](../../MVP效果图/自动保存与历史.png) | History，R117→R128 side-by-side Diff | 版本时间线、差异、恢复验收 |
| V6 | [单机批注.png](../../MVP效果图/单机批注.png) | Annotation，右侧批注 Rail，正文悬浮卡 | Human Annotation MVP 验收 |
| V7 | [暂时不实现-词性EOS.png](../../MVP效果图/暂时不实现-词性EOS.png) | POS token overlay，含 Surface/Lemma/Provider | 延期视觉参考，不进入 MVP 验收 |

### 2.2 参考窗口坐标

以下为从七张图观察得到的实现起点，允许因系统边框与 DPI 有 1–3 px 误差；验收时以比例和锚点关系优先。

| 区域 | 参考坐标/尺寸 | 占 1448×1086 比例 | 约束 |
|---|---:|---:|---|
| 系统标题栏 | y=0–49，h≈50 | 4.6% 高度 | 原生窗口标题和最小化/最大化/关闭 |
| 应用 TopBar | y=50–117，h≈68 | 6.3% 高度 | 命令、模式、在线/离线或保存状态 |
| 左侧导航 | x=0–167，w≈168 | 11.6% 宽度 | 固定宽度，当前项淡绿底 |
| 正文工作区顶部 | y≈118–177，h≈60 | 5.5% 高度 | 双栏标题/模式专属工具栏 |
| 正文主体 | y≈177–1009，h≈832 | 76.6% 高度 | 虚拟列表或工作区内容 |
| 底部状态栏 | y≈1009–1085，h≈76 | 7.0% 高度 | 工程、文件、统计、进度、路径 |
| 关系连接区 | 常规视图 w≈64，随主体居中 | 4–6% 主体宽度 | 图标/线条仅辅助，不能遮文本 |
| Annotation Rail | V6 x≈1093–1448，w≈355 | 24.5% 总宽 | 打开时压缩正文，不覆盖正文 |
| History 时间线 | V5 x≈168–555，w≈387 | 26.7% 总宽 | 左栏固定，右侧显示所选 Revision |

V1/V2/V6 的正文双栏在 Annotation Rail 关闭时占用 x≈168–1448；V6 打开 Rail 后正文区域缩至 x≈168–1093。正文两侧宽度可以随窗口变化，但关系连接区保持 56–72 px 的可读范围。V3 Order 在主体内部保留约 14 px 外边距，以圆角边框包住整张句列表。

### 2.3 设计基线

- 画布基线：1448×1086，Windows 桌面端，默认 100% UI 缩放。
- 逻辑 CSS 像素与截图像素在 100% DPI 时 1:1；125%/150% 下按 CSS 逻辑尺寸缩放，不通过位图放大 UI。
- 圆角以 6–8 px 为主；按钮、输入框和卡片不使用大面积阴影，主层级靠边框、留白和浅色背景区分。
- 线条是 #E6EBE6 一类的低对比边界；只有当前态、动作和危险状态使用较强颜色。
- 正文优先阅读密度，正文行高不能因标签、批注标记或连接线发生跳动。

## 3. Design tokens

### 3.1 颜色语义

颜色名称是语义 token，不允许在页面组件中散落硬编码。以下 hex 是从截图视觉观察形成的近似实现基线；实现后以截图回归校准，但必须保留语义关系。

| Token | 参考值 | 用途 |
|---|---|---|
| `--jm-bg` | `#FFFFFF` | 主窗口、正文底色 |
| `--jm-surface` | `#FBFCFB` | 工具栏、输入区、卡片 |
| `--jm-border` | `#E5EAE5` | 分隔线、表格线、输入边框 |
| `--jm-border-strong` | `#C9D5C9` | 当前控件、选中卡片边界 |
| `--jm-text` | `#2B2F2B` | 主标题、正文、按钮文字 |
| `--jm-text-muted` | `#6D746E` | 次要说明、时间、计数 |
| `--jm-text-faint` | `#A0A7A1` | 未激活 ID、禁用态、远端上下文 |
| `--jm-green-700` | `#2D7834` | 主操作、当前 anchor、导航选中 |
| `--jm-green-600` | `#438D46` | hover/连线/进度 |
| `--jm-green-100` | `#EAF5E4` | source/current 背景、导航当前底 |
| `--jm-green-050` | `#F4FAF1` | source 淡层、轻量提示 |
| `--jm-yellow-700` | `#B77A12` | target/current 边界或 ID |
| `--jm-yellow-500` | `#E9B94F` | 书签、目标态强调 |
| `--jm-yellow-100` | `#FFF4D8` | target/current 背景、替换预览 |
| `--jm-yellow-050` | `#FFFAEE` | target 淡层 |
| `--jm-violet-600` | `#8C62C7` | 批注编号、Draft 标签、关联提示 |
| `--jm-violet-100` | `#F2EAFE` | Draft 批注底 |
| `--jm-blue-600` | `#4A78B8` | In Progress/信息性标记，不能代替文字状态 |
| `--jm-red-600` | `#B85C5C` | 删除、错误、危险操作文字 |
| `--jm-red-100` | `#FCEAEA` | Diff 删除背景、错误背景 |
| `--jm-diff-add` | `#EEF8EA` | Diff 新增背景 |
| `--jm-focus` | `#2D7834` | 键盘 focus ring，至少 2 px |

Light 主题为默认主题。Eye Care 主题只调整底色、正文和边界对比，不改变 source/target/action 的语义色；所有状态还必须有文字、图标或形状辅助，不能只靠颜色区分。

### 3.2 字体与文字层级

2026-08-31 按当前用户要求采用 Apple HIG macOS 排版层级；本节替代原 Windows 示意图中的字号建议。所有尺寸为 WebView 的 CSS px，不能写成 CSS pt。来源为 [Apple Typography](https://developer.apple.com/design/human-interface-guidelines/typography)；微信资料范围与单位解释见 [macOS 排版研究](wechat-macos-design-language-and-typography-v0.1.md)。

字体和字号由 `apps/desktop/src/styles.css` 的共享 token 管理。系统字体栈：

```css
font-family: system-ui, -apple-system, BlinkMacSystemFont,
  "PingFang SC", "Segoe UI", "Microsoft YaHei UI", sans-serif;
```

| 层级 | 字号/行高参考 | 字重 | 用途 |
|---|---:|---:|---|
| App title / Workspace heading | 15 / 20 px | 600 | 品牌标题、双栏栏目、批注面板标题；Title 3 强调样式 |
| Page / Dialog title | 17 / 22 px | 400 | 设置、书签、历史、导入与操作弹窗；Title 2 |
| TopBar/Nav / Button label | 13 / 16 px | 400；当前导航 600 | 命令、导航、模式名称；Body |
| Secondary / Meta | 12 / 15 px | 400 | 副文字、摘要、底栏、字数；Callout |
| Auxiliary | 11 / 14 px | 400；徽标可 500/600 | 时间、附属标签；Subheadline，不使用 9 px 小字 |
| Segment ID | 12 / 15 px | 400 | 可见编号，使用系统等宽字体；不是 canonical ID 生成规则 |
| Chinese body | 16 px / 1.62 | 400 | Review/Order 原文，乘以用户阅读缩放 |
| English body | 15.5 px / 1.62 | 400 | Review/Order 译文，乘以用户阅读缩放 |
| Edit body | 16 px / 1.52 | 400 | 当前 textarea 编辑正文，保留阅读缩放 |
| Annotation title | 13 / 16 px | 600 | 批注卡片内部标题 |

常用字重限定为 Regular 400、Medium 500、Semibold 600；Bold 700 作为明确强调的保留 token，不再散落 570/620/650/670/750 等数值。主字号用于界面操作，阅读正文独立管理；多行中文说明可保留 1.4–1.65 行距，不能把短控件的行高强加给所有段落。

中文正文和英文正文可使用不同字号以达到视觉高度接近，但同一 Segment 的首行和底部边界必须稳定。字号与字体变化后由既有 ResizeObserver 重新测量虚拟行。正文中不要使用全大写作为状态唯一表达；英文模式名称可以保留截图中的 Title Case。macOS 的 SF / 中文回退实际观感仍需在 Mac 上验证，Windows 预览不能替代。

本次验证（2026-08-31）：前端 typecheck、build 通过；在 Windows 浏览器预览中检查审阅排序、编辑、历史空态、批注侧栏、设置、搜索空态、书签空态和新建弹窗，覆盖明亮与护眼主题。阅读缩放 130% 时中英文正文分别为 20.8 / 20.15 px，工具栏仍为 13 px；编辑框在 100% 时为 16 px / 400。未进行 macOS 实机字体渲染验收，也未以浏览器演示数据替代本地工程功能验证。

### 3.3 间距、尺寸、边界

基础间距采用 4 px 倍数：`4, 8, 12, 16, 20, 24, 32, 40, 48`。建议 token：

| Token | 值 | 用途 |
|---|---:|---|
| `--jm-space-1` | 4 px | 图标与文字、徽标内边距 |
| `--jm-space-2` | 8 px | 紧凑控件、表格单元 |
| `--jm-space-3` | 12 px | 行内元素、卡片局部 |
| `--jm-space-4` | 16 px | 导航项、正文水平 padding |
| `--jm-space-5` | 20 px | 工具栏控件间隔 |
| `--jm-space-6` | 24 px | 卡片/面板内边距 |
| `--jm-space-8` | 32 px | 工作区分组、底栏分隔 |
| `--jm-row-min` | 96 px | 普通平行行最小高度 |
| `--jm-control-h` | 36–40 px | 常规按钮、输入框 |
| `--jm-radius-sm` | 4 px | 输入框、徽标 |
| `--jm-radius-md` | 8 px | 卡片、当前段落容器 |
| `--jm-focus-ring` | 2 px | 键盘焦点外环 |

边框默认 1 px。当前 source/target 卡片使用 1 px 语义边框，编辑态外层可以使用 1–2 px；不要使用粗重实线把两个语言侧割裂。正文横向分隔线应贯穿两侧和连接区，当前行可以改变底色而不改变高度。

## 4. 应用壳与通用布局

### 4.1 TopBar

左侧从 x≈28 开始依次为新建、打开、保存、导出；随后以竖分隔线分组撤销、重做和设置。图标为线性图标，常规 20–22 px，文字采用第 3.2 节的 13 px / 16 px、Regular 400。按钮在 hover/focus 时只加浅色底和 focus ring，不改变布局。保存中状态应在按钮或底栏同时以文字显示，禁用重做使用低对比灰色。

右侧为 ModeSwitcher，参考 V1–V6 右上角约 180–210 px 宽的描边 pill：左侧图标、中文模式名、英文副标题、下拉 chevron。Review/Annotation/Order/Edit/History/Search 使用各自语义 icon，但模式切换后正文宿主仍为同一 `ParallelWorkspace`。V6 另在右侧显示“本地模式·离线”，该状态必须明确且不应伪装为云连接。

### 4.2 SideNav

宽约 168 px，顶部距 TopBar 约 26–32 px。每个导航项高约 52–60 px、水平 padding 24 px，图标 22–24 px，文字与图标间 12 px。V1–V2–V3 的“平行视图”选中；V4 为搜索；V5 为历史；V6 为批注模式并在导航中保持醒目。选中项底色 `--jm-green-100`，左/右不使用强色条；图标和文字使用 `--jm-green-700`。底部“收起”位于一条淡分隔线下，折叠后必须仍保留可访问的 tooltip 和键盘入口。

### 4.3 FooterStatusBar

底部固定、上边框 1 px；按截图从左至右显示工程名、双侧文件名、对齐状态（如 1:1）、已处理数量、进度条/百分比、项目路径。各块使用 12 px / 15 px 文本，分隔符为竖线或足够的空白；路径过长时截断并提供 tooltip。保存状态应优先显示“未保存 / 保存中 / 已保存 / 保存失败”，进度条不能覆盖状态文字。

### 4.4 正文工作区

`ParallelWorkspace` 结构：

```text
WorkspaceHeader / mode toolbar
┌────────────── Source ──────────────┬─ Alignment ─┬──────────── Target ────────────┐
│ SegmentView list                   │ anchor/link │ SegmentView list               │
└────────────────────────────────────┴─────────────┴───────────────────────────────┘
FooterStatusBar
```

左右侧必须有独立可滚动语义但共享 `AlignmentViewportController`。实现可用双虚拟列表，首屏只加载 Slice；当前 anchor 由 `AlignmentId` 维护。关系线、link icon、选中框在连接区绘制，不能插入正文 DOM 流导致文本跳动。

## 5. 四种模式规范

### 5.1 Review / 平行阅读（V1）

Review 是默认工作模式，强调“当前读到哪里”。V1 中主体顶部仅显示中文/英文列标题，下面是连续的 000101–000108 组。每组有稳定 ID、左右正文和中央 link icon；当前 000104 左侧为淡绿色、右侧为淡黄色，连接图标为深绿圆形，正文加粗。000103 旁显示金色书签星标。

结构规则：

- 当前 Alignment 两侧同步高亮；source 使用 `--jm-green-100`，target 使用 `--jm-yellow-100`；
- 远端上下文可以降低对比度，但不透明度不能低到不可读；
- 连接区显示关系类型/状态的图标和必要线段，1:1 不重复文字；复杂 1:n、n:m 用多端连接或汇总徽标；
- 点击任意 Segment 更新 anchor 并双向定位，不写入 canonical data；
- 书签位于 gutter 或行尾，金色星形与正文留出至少 8 px；
- 到段落边界显示轻量 `上一段 · N 句` / `下一段 · N 句` 触发区，点击后 inline expand 并保持 anchor 视觉位置；
- 批注编号在正文行内采用圆形小标记，悬浮时显示摘要卡；打开 Annotation Rail 时不遮挡正文。

首屏比例：正文主体两侧总宽约 1,280 px，连接区约 64 px；普通行高度 96–116 px，长句通过增高行承载，不压缩文字。当前行边框可见但不使用强阴影。

### 5.2 Edit / 平行修改（V2）

Edit 只让一个对齐组进入编辑态，另一侧仍在同一中心保持可见。V2 观察到 000104 source 外层淡绿、target 外层淡黄；两侧各有白底编辑卡片、灰边框、文本区和底部 meta 工具带。source 下方有“取消/保存”，target 下方有“上一句/下一句”。

组件规则：

- 编辑区使用 CodeMirror 6，不能使用自制 `contenteditable` 作为主编辑器；
- 编辑卡最小高度约 164 px，正文区约 112 px，底部工具带约 48 px；
- 中文显示“字数：N”，英文显示“Words: N”；拼写检查是基础入口，若 Provider 不可用要有文本说明；
- `Ctrl+Enter` 保存、`Esc` 取消；离开模式前存在草稿必须弹出保存/放弃/取消切换确认；
- 保存按钮使用深绿实心，取消按钮白底描边；保存中按钮显示忙碌态且不可重复提交；
- 保存不改变 `SegmentId`/`AlignmentId`，下方状态栏进入“保存中”后回到“已保存”；
- 编辑器中 `Ctrl+F` 只查当前编辑器，不能声称是工程搜索。

### 5.3 Order / 语句重排（V3）

Order 取消 Review 的当前淡化与突出，使每个句子作为结构对象等权。V3 顶部增加“排序工具：拖动排序、上移、下移、恢复顺序”，右侧显示“共 8 句 | 拖动句子调整顺序”。正文有整表圆角边框、左侧 drag handle、序号、左右正文和低权重连接 icon。

交互视觉：

- drag handle 是 6 点阵/抓手图标，至少 24×24 px hit area；
- 拖动对象显示淡绿色卡片、轻阴影和 1 px 绿色边框；目标位置显示虚线绿色插入槽，含“将句子拖放到此处”；
- 拖拽结束只提交 `MoveSegment(segmentId, before|after)`，不提交 DOM index；
- 关系连接线降为灰色细线；crossing alignment 以小型 `alignment order differs` 提示，不自动改对齐；
- 上移/下移按钮支持键盘；拖拽有等价命令，不以鼠标为唯一方式；
- “恢复顺序”是当前 Order 会话的可撤销操作，不直接恢复历史 Revision；
- 句序变化只改 `PositionKey`，Segment、Alignment、Bookmark、Annotation 锚点保持稳定。

Order 视图不应以淡色区分“当前阅读点”；若有选中，仅用 1 px 边框和 focus ring，保持其他句子同等可读。

### 5.4 Search / 搜索与替换（V4）

V4 左侧仍为通用 SideNav，正文顶部有“查询”标签、带放大镜的输入框、绿色“搜索”按钮和下拉箭头；右侧有“正则”“区分大小写”“当前工程”复选框、“更多筛选”和“重置”。结果摘要显示命中数和耗时，表格列为 ID、中文左上下文、匹配词、中文右上下文、English、对齐 ID。

搜索规则：

- `Ctrl+F` 只搜索当前 View/当前编辑器/当前展开上下文/当前 Diff/当前批注；
- `Ctrl+Shift+F` 走 Rust Kernel Project Search，返回 `SegmentId`、`AlignmentId`、语言侧、上下文和命中范围；
- 结果表按 stable ID 绑定 key，虚拟滚动时不能以行号做 key；
- 命中词使用淡黄色标记，当前行使用淡绿色或浅黄边界，选择态不覆盖原文；
- 结果点击后跳回 ParallelWorkspace 并以 `AlignmentId` 定位；底部预览保持与结果一致；
- 替换必须先显示 before/after 预览，部分勾选后一次 ChangeSet 原子提交；
- 正则错误、命中集过期或写入失败时显示错误原因，不部分提交。

V4 的下方预览区为左右两栏文本，当前预览行浅黄色边界、中央 link button；高度约 240–260 px。搜索结果表头和表格线使用低对比度，不能让 1,248 条命中在视觉上变成可编辑正文。

### 5.5 History / 自动保存与历史（V5）

V5 左栏是历史时间线，右栏是比较工作区。左栏约 387 px：顶部“历史记录”和筛选按钮，卡片按 R128、R121、R117、R103 逆序排列，当前版本有绿色边框与“当前版本” badge。右栏顶部显示“比较：R117 → R128”、比较/恢复此版本/当前版本按钮。

右侧规范：

- 先选择两个 `RevisionId`，再加载 Diff，不预计算整个项目；
- 顶部有“段落：000104”和 link icon，可从 History 返回正文 anchor；
- 文本 Diff 使用 CodeMirror Merge，支持 unified/side-by-side；V5 默认为 side-by-side；
- 删除为低饱和淡红背景和删除线，新增为低饱和淡绿背景，保持内容为深色；
- 左右栏各有 R117（旧版本）/R128（当前版本）标题和复制图标；
- 结构变化使用摘要卡（删除 1 行、添加 1 行、术语优化等），不强行编码为文本 diff；
- 恢复旧版本提交 `RestoreRevision` 创建新 Revision，不能直接覆盖或删除当前版本；
- “自动保存”状态在 TopBar/Footer 反馈；崩溃恢复后展示最后成功落盘 Revision。

History 分三种投影：Text History、Alignment History、Project Operation Trace，共享 RevisionId；视觉上时间线卡片与文本 Diff 分离，避免让用户误以为对齐结构变化只是字符串变化。

### 5.6 Annotation / 单机批注（V6）

V6 在右侧打开约 355 px Annotation Rail，正文区域仍保持双栏且不被覆盖。Rail 顶部有“批注 / Annotations”、关闭按钮、状态 tab（全部、草稿、进行中、已解决）和排序下拉；底部有“+ 新建批注”按钮。

批注卡片规则：

- 每张卡显示编号、状态中文/英文、来源“本地”、时间、标题、正文摘要、关联 Segment；
- Draft 用淡紫底/紫色编号；In Progress 用信息色和清晰文字；Resolved 用绿色完成 icon/文字；
- 关联片段显示中文 Segment 和 English Segment 两行，ID 与文本都可点击；
- 操作含编辑、删除、标记已解决和更多菜单；删除必须确认或可撤销；
-正文标记与 Rail 卡片共享 `AnnotationId`，点击任一侧定位另一侧，不改变 Segment 文本；
- V6 中浮在 000103 下方的悬浮摘要卡是 inline popover，显示本地标记、标题、关联/查看源文与译文、删除；点击外部或 Escape 关闭；
- 侧栏宽度不足时卡片内容折行，不以省略号隐藏批注正文；
- 批注的状态、关联和时间必须有文字，不只用颜色。

Human Annotation 与 POS/Lemma 等机器 Annotation 分离命名。POS 图中的 token 标签、Surface/Lemma/Provider/Status 卡在 MVP 只做延期占位说明，不从 V6 复用为批注对象。

## 6. 组件清单与职责

### 6.1 通用壳组件

| 组件 | 职责 | 复用/自研 |
|---|---|---|
| `AppShell` | Tauri 窗口、安全区域、主题、路由出口 | 自研 |
| `TopBar` | 新建/打开/保存/导出/撤销/重做/设置、模式入口 | 自研领域壳 |
| `SideNav` | Project/Parallel/Search/Bookmarks/Annotations/History/Settings | 自研 |
| `ModeSwitcher` | Review/Edit/Order/History 统一切换 | 自研 |
| `FooterStatusBar` | 工程、文件、统计、保存、进度、路径 | 自研 |
| `CommandButton` | 语义按钮、busy/disabled/focus 状态 | 自研 + Reka UI 原语 |
| `Tooltip/Popover/Dialog/Menu` | 说明、确认、批注卡、菜单、焦点管理 | Reka UI wrapper |

### 6.2 平行领域组件

| 组件 | 职责 | 关键约束 |
|---|---|---|
| `ParallelWorkspace` | 四模式宿主，协调 selection/anchor/panel | 不复制 canonical data |
| `ParallelViewport` | 双侧虚拟列表、行测量和可见 Slice | `@tanstack/vue-virtual`，key 用 stable ID |
| `SegmentView` | 同一 Segment 在四模式下的 presentation | mode 改权限与视觉，不改身份 |
| `AlignmentGutter` | link/unlink 状态、关系线、当前 anchor | 不覆盖正文，不作为唯一语义 |
| `AlignmentViewportController` | 左右滚动、定位、anchor 维护 | 以 AlignmentId/SegmentId 定位 |
| `ContextLens` | inline expand、上一/下一段、上下文范围 | 保持 anchor 视觉位置 |
| `SegmentEditor` | CodeMirror 6 编辑器包装 | 草稿与提交态分离 |
| `OrderToolbar` | 拖动、上移、下移、恢复顺序 | command adapter 接 stable ID |
| `OrderDropIndicator` | 插入槽、drag ghost、键盘目标 | 不产生真实顺序数据 |
| `CorpusQueryView` | Kernel 搜索结果、筛选、跳转 | 虚拟化、命中集有 base revision |
| `SearchPreview` | 左右预览、命中和替换前后 | 不直接改 canonical data |
| `HumanAnnotationLayer` | gutter 标记、inline 卡、Rail、筛选 | sidecar layer，独立 schema |
| `HistoryWorkspace` | 时间线、Revision 选择、恢复 | `RevisionId` 事务边界 |
| `HistoryDiffViewer` | unified/side-by-side 文本 Diff | CodeMirror Merge |
| `ChangeSummary` | 对齐/顺序/书签/批注结构变化摘要 | 不伪装成文本 diff |

### 6.3 基础库边界

技术路线固定为 Tauri 2 + Rust stable Kernel + Vue 3 Composition API + TypeScript strict + Vite + pnpm。基础库边界：

- `@tanstack/vue-virtual`：虚拟滚动与测量；
- CodeMirror 6、`@codemirror/search`、`@codemirror/merge`：编辑、当前 View 查找、文本 Diff；
- Reka UI：Dialog、Popover、Menu、Tooltip、Select、Tabs、焦点和键盘原语；
- Dockview：P1 Context Pane；
- Tauri `WebviewWindow`：P1 新窗口上下文；窗口只共享 ID/Query，不复制可变 canonical state；
- Vue Router：顶层工作区路由；四种 mode 不做成独立 route；
- Pinia：UI state、selection、slice handle、panel、scroll state，不存全工程可变数组；
- CSS Custom Properties + scoped CSS：决明 token 和 Light/Eye Care 主题，不引入强视觉大型 UI 框架。

## 7. 交互状态与无障碍要求

### 7.1 共有状态

每个按钮、输入、列表和卡片至少定义 default、hover、focus-visible、pressed/selected、disabled、busy/error（适用时）状态。焦点环使用 `--jm-focus`，不少于 2 px，并保留至少 2 px 外间距；不能以去除 outline 作为视觉清理手段。

| 状态 | 视觉 | 必须文字/语义 |
|---|---|---|
| 当前 Alignment | source 淡绿、target 淡黄、连接 icon 深绿 | 当前段/对齐关系可读 |
| 未对齐 | 中性边界或“未对齐”徽标 | 明确说明，不伪装 1:1 |
| 保存中 | 按钮 busy、底栏“保存中” | 可等待，不重复提交 |
| 保存失败 | 淡红提示/错误 icon | 原因、重试和数据是否安全 |
| Draft | 淡紫底、编号徽标 | Draft 文本状态 |
| In Progress | 信息色/进行中文字样 | 当前工作状态 |
| Resolved | 绿色完成 icon/文字 | 已解决，不只变灰 |
| Diff 删除 | 淡红、删除线 | 删除语义可读 |
| Diff 新增 | 淡绿 | 新增语义可读 |
| UNBOUND Slot | 中性说明卡 | “尚无 Provider”，不提供假入口 |

### 7.2 键盘路径

- `Tab` 顺序：TopBar → ModeSwitcher → SideNav → 当前工作区工具 → 正文 Segment → Footer 操作；
- `Enter` 或双击进入 Segment Edit；`Ctrl+Enter` 保存；`Esc` 取消；
- Review `ArrowUp/ArrowDown` 在对齐组间移动，`Enter` 聚焦当前；
- Order 以 `Alt+ArrowUp/Down`（或设置中公开的等价快捷键）执行上移/下移；鼠标拖拽必须有键盘等价；
- Search 结果可用上下键选中、Enter 跳回；
- Annotation Rail 支持 tab、状态筛选、卡片编辑/删除/解决；
- `Ctrl+F` 与 `Ctrl+Shift+F` 的作用域在帮助/tooltip 中明确。

屏幕阅读器需能读出语言侧、Segment ID、当前/未对齐、Alignment cardinality、批注状态和保存状态；连接线和颜色不作为唯一信息通道。减少动画设置下，mode transition、定位和 drag ghost 关闭或缩短，不影响定位结果。

## 8. 响应与窗口缩放

### 8.1 支持区间

| 窗口逻辑尺寸 | 行为 |
|---|---|
| 1448×1086 | 截图基线，所有标注比例和布局直接对照 |
| ≥1280×800 | 完整桌面布局；可关闭 Annotation Rail 释放正文宽度 |
| 1100–1279 宽 | SideNav 可折叠为图标栏；连接区不低于 56 px；正文维持双栏 |
| 960–1099 宽 | 默认折叠 SideNav；Annotation Rail 改为可切换 overlay/dock，不能覆盖当前编辑卡 |
| <960 宽或高度 <680 | 显示桌面最小尺寸提示；不把双栏压成不可读的两列 |

### 8.2 缩放规则

- 侧栏、TopBar、Footer 使用 flex/grid，不写死截图像素；以 `clamp()` 控制内边距和字号上限；
- 正文列宽使用 `minmax(0, 1fr)`，连接区使用 `clamp(56px, 5vw, 72px)`；
- Annotation Rail 关闭时正文占满；打开时优先保留双栏最小宽度，再缩小 Rail；宽度不足时 Rail 变 Dock/overlay，但正文不被遮挡；
- 长文本允许行高增加，禁止横向溢出；中英文混排和 CJK 标点不产生水平滚动条；
- 125%/150% DPI 下图标不位图模糊，按钮文字不截断；若空间不足，优先收起文字而保留图标和 tooltip；
- 窗口大小、折叠状态和面板布局属于 UI state/用户设置，不进入项目 canonical data。

## 9. 截图对比验收矩阵

### 9.1 截图级验收

执行方式：在 Windows Tauri 开发/打包构建中，以 1448×1086、100% DPI 打开固定 Tiny fixture，等待字体、虚拟列表和保存状态稳定后截图。每张截图在同一窗口位置和同一数据 Revision 下对照原图；先做结构/锚点评审，再做像素差异。像素差异只作为定位工具，不能以反锯齿差异否定结构一致性。

| ID | 必须出现 | 关键比例/状态 | 行为验收 |
|---|---|---|---|
| V1 Review | TopBar、SideNav、双栏标题、000101–000108、link icon、000104 当前、高亮 000103 书签、Footer | 168 px 侧栏；64 px 连接区；当前 source 淡绿/target 淡黄；正文不被遮挡 | 点击左右 Segment 双向定位；上下文 inline expand 保持 anchor；书签可跳转；未对齐有文字 |
| V2 Edit | 000104 双侧编辑卡、文本区、字数/Words、拼写检查、取消/保存、上一句/下一句 | 编辑卡两侧分别保留 source/target 语义色；控件带 36–40 px 高；Footer 显示保存态 | Enter/Esc/Ctrl+Enter；取消不写入；保存生成一条 ChangeSet；SegmentId/AlignmentId 稳定 |
| V3 Order | 排序工具栏、拖动排序、上移、下移、恢复顺序、共 8 句、drag handle、拖拽 04、虚线插入槽 | 取消当前淡化；所有句子等权；拖拽卡淡绿；关系线降权 | 鼠标与键盘均可移动；只改 PositionKey；重排后书签/批注/对齐仍指向原对象 |
| V4 Search | 查询输入、普通/正则/大小写/当前工程、结果计数、结果表、匹配高亮、下方双语预览 | 结果表在上、预览在下；当前结果浅绿/黄；Rail 不出现 | 普通/正则查询；Ctrl+Shift+F 走 Kernel；结果回跳 Alignment；替换预览后原子提交/一次撤销 |
| V5 History | 历史时间线、R128 当前、R117→R128、旧/新双栏、淡红删除、淡绿新增、摘要、恢复按钮 | 左时间线约 387 px；右 Diff；低饱和色；比较操作在顶部 | 选择 Revision 才载入 Diff；结构摘要正确；恢复创建新 Revision；旧历史仍可见 |
| V6 Annotation | 双栏正文、编号标记、inline 悬浮卡、355 px Rail、状态 tabs、Draft/In Progress/Resolved 卡、“新建批注” | Rail 打开正文压缩而非覆盖；Draft 紫色、Resolved 绿色；状态有文字 | 新建/编辑/删除/解决；左右 Segment/Alignment 关联；重排后 ID 跳转不变；无账号依赖 |
| V7 POS（延期） | 仅用于设计 token overlay 研究 | 不列入 MVP 截图分数 | MVP 不显示词性覆盖层、Lemma、Provider 卡或 POS 入口；若出现必须说明 UNBOUND/延期 |

### 9.2 截图比较检查项

每个 V1–V6 至少检查：

1. **壳层**：标题、TopBar、SideNav、Footer 的层级和对齐；
2. **比例**：侧栏、正文双栏、连接区、Rail/History 分栏宽度；
3. **内容**：稳定 ID、语言标题、示例文本、状态文字和按钮位置；
4. **颜色**：source/target/current/action、Diff、批注状态和书签语义；
5. **密度**：正文行高、表格行高、卡片 padding、控件高度；
6. **锚点**：当前 Alignment、搜索结果、批注关联、History Revision；
7. **响应**：1448×1086、125%/150% DPI、窄窗口无截断或覆盖；
8. **无障碍**：focus ring、键盘等价操作、颜色之外的状态文本。

建议为每张图保留 baseline/reference/render 三份 PNG 和一份人工结论，差异分级：

- P0：正文覆盖、当前对齐错位、数据/状态不可见、窗口无法操作；
- P1：分栏比例、当前态颜色、卡片/表格层级明显偏离；
- P2：字号、图标、边界、间距的小幅偏差；
- P3：抗锯齿、系统字体 fallback 等不影响操作的差异。

### 9.3 计划退出条件映射

| 计划条目 | 本规范证据 |
|---|---|
| `ParallelWorkspace` 四模式共享 | 第 4、5、6.2 节，模式只改呈现/权限 |
| Design tokens 与主题 | 第 3 节；Light/Eye Care 保持语义色 |
| Review / Edit / Order / History | 第 5.1–5.5 节及 V1–V5 矩阵 |
| 搜索与替换 | 第 5.4 节及 V4 矩阵 |
| 单机批注 | 第 5.6 节及 V6 矩阵 |
| POS 延期 | 第 1、5.6、9.1 V7 |
| 虚拟列表与 stable ID | 第 4.4、5.4、6.2、7 节 |
| 响应/高 DPI/键盘 | 第 7、8、9.2 节 |

## 10. 实现前冻结清单

- [ ] 将 `--jm-*` token 写入 UI 设计系统并在组件中禁止散落颜色值；
- [ ] 固定 1448×1086 Tiny fixture 与 V1–V6 的可重复 Revision/Annotation 数据；
- [ ] 固定四模式 `ParallelMode`、进入/退出守卫和 `AlignmentId` anchor 合同；
- [ ] 固定 168 px SideNav、64 px 连接区、Footer 与 Rail/History 的最小宽度策略；
- [ ] 先完成 Review 的双栏虚拟列表和稳定 ID 跳转，再复用到 Edit/Order/History；
- [ ] 用 CodeMirror 6、Reka UI、TanStack Virtual、CodeMirror Merge 建立基础能力适配层；
- [ ] 明确 `Ctrl+F` View Find 与 `Ctrl+Shift+F` Kernel Project Search 的 UI 文案和快捷键提示；
- [ ] 为批注使用独立 `HumanAnnotation` schema，不与 POS/lemma 视觉对象共用状态；
- [ ] 为每张截图配置 Playwright/Tauri 截图回归与人工结构验收记录；
- [ ] 在 125%/150% DPI 和窄窗口完成无截断、无覆盖、可键盘操作检查；
- [ ] 在 MVP 发布前确认延期 Slot 以 `UNBOUND` 文本说明，不出现假功能入口。

## 11. 证据限制

本规范基于静态示意图和架构/功能文档，无法仅凭图片证明：实际 Tauri 窗口行为、虚拟滚动性能、输入法、键盘焦点顺序、崩溃恢复、文件编码、原子替换、屏幕阅读器输出和不同 DPI 下的真实渲染。上述内容已转为实现与测试验收项，必须在代码产出后用运行时测试、截图回归和 Windows smoke test 补证。截图中的示例文本、计数、路径和版本号是 fixture 展示值，不是业务常量。
