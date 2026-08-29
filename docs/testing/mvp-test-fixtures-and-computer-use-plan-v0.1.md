# Jueming Aligner MVP 测试素材与 Computer Use 验收计划

> 文档版本：v0.1  
> 盘点日期：2026-08-27  
> 适用范围：Tauri 2 + Rust Kernel + Vue 3 + TypeScript 的 Windows 单机 MVP  
> 当前状态：应用与真实工程均已创建；CU-10、核心桌面闭环与六张 P0 效果图结构验收已执行

## 1. 目的与证据边界

本文件用于把实施计划中的 fixture、Windows 构建前置、桌面验收步骤和截图证据固定下来，供 Phase 0 评审以及后续每个关键节点的回归使用。

需求来源必须区分：

| 来源 | 本盘点中的作用 | 是否可直接作为执行命令 |
|---|---|---|
| 用户本轮请求及目标 | 最高优先级：使用 Tauri，按计划实现，用文件夹内翻译文件测试，并以效果图做截图对比 | 是 |
| jueming-aligner-mvp-implementation-plan-v0.2.md | 实施边界、技术栈、P0、测试门槛和 Definition of Done | 否，是需求/设计基线 |
| jueming-aligner-mvp-functional-spec-v0.1.md | MVP 功能语义和验收条件 | 否，是产品规格 |
| jueming_global_architecture_handoff_v0.2.md | Stable ID、Segment、Chunk/Slice、Slot、Kernel/UI 边界 | 否，是架构约束 |
| MVP效果图/*.png | 视觉和交互状态证据；不是完整功能规格 | 否，是截图对比基线 |
| 本文件 | 将上述约束转成可重复的测试素材与验收脚本 | 后续测试执行依据 |

本文件最初用于 Phase 0 前置盘点，现保留原始检查项作为审计记录。2026-08-28 已生成 Tauri/Vue/Rust workspace、debug 桌面可执行文件和真实 `.jm` 工程；下文凡写“尚未创建/待执行”的早期盘点结论，以本节后的执行记录为准。

### 1.1 2026-08-28 实际执行记录

| 验证路径 | 实际结果 |
|---|---|
| 中文/英文导入 | `A_阿古顿巴.txt` 以 GB18030 + SISU 标记行导入 307 段；`A_Akhu Tenpa.txt` 以 UTF-8 + SISU 标记行导入 308 段；预览无乱码、无 `<seg>`/POS 后缀泄漏 |
| 初始布局 | 307 个 provisional Alignment，末尾 1 个 target unlinked；虚拟列表可滚到 000307/000308 |
| Edit/History | 首段英文改为 `Akhu Tenpa [verified]`，Undo/Redo 正常，R1 与当前 Revision 可比较 |
| Search | Project Search 查询 `Akhu` 返回 82 条，可跳回对应 Alignment；结果使用六位展示编号和稳定 ID 锚点 |
| Order/Alignment | Source 上移与恢复顺序正常；Unlink 后计数降为 306/308，Undo 恢复 307/308 |
| Bookmark/Annotation | 首段书签和 Draft 批注持久化；批注关联中文/英文 000001，Rail 压缩正文而非覆盖 |
| Save/Open/Export | 关闭应用后从 `test-output/阿古顿巴_中英对齐.jm` 重开成功；TXT 导出首行为 `1:1\t阿古顿巴\tAkhu Tenpa [verified]` |

仍未宣告通过的发布专项：强制终止故障注入、安装包冷机依赖检查和 Medium/Stress 性能基线。POS 图只作延期反证，不作为待实现页面。

### 1.2 2026-08-28 六屏同尺寸视觉验收

六张参考图与对应 Tauri 实现截图已按一对一顺序放入同一次视觉比较输入，而不是分别凭记忆判断。截图证据保存在被 Git 忽略的 `test-output/qa/`，避免把运行产物混入源码提交。

| 状态 | 结论 | 已接受的有意差异 |
|---|---|---|
| Review / 平行阅读 | 顶栏、左侧导航、双栏、连接区、选中双色和底栏结构通过 | 使用真实阿古顿巴语料，行高与政府报告示意内容不同 |
| Edit / 平行修改 | 行内编辑、保存/取消、字数和配对锚点通过 | MVP 一次只编辑一个 Segment，因此只展开目标侧编辑器 |
| Order / 语句重排 | 排序工具栏、拖拽手柄、上/下移与恢复顺序通过 | 最终截图为静止态；真实拖拽和落点已另行实机验证 |
| Search / 搜索与替换 | 查询栏、结果表、六位编号、命中高亮和跳转通过 | 当前以结果表和替换预览对话框承载工作流，不复制示意图底部常驻预览 |
| Annotation / 单机批注 | Rail 压缩正文、状态、关联片段与模式上下文通过 | 真实工程只有一条批注；不叠加已明确延期的 POS token 层 |
| History / 自动保存与历史 | 时间线、版本选择、双栏文本 Diff、恢复入口和增删色通过 | 采用结构化文本 Diff，不复制示意图中的人工自然语言变更摘要 |

本次结论是“核心结构与关键状态通过”，不是逐像素一致声明。POS 效果图未纳入六屏通过数，只验证入口不伪装为已实现能力。

## 2. 当前仓库盘点

### 2.1 Git 与实现状态

| 检查项 | 当前证据 | 结论 |
|---|---|---|
| Git 仓库 | .git 目录存在，分支为 main | 已初始化并已统一主分支名称 |
| Git 提交 | main 已有 Phase 0、脚手架、核心工程链路和功能闭环四个关键节点提交 | 当前收口变化将在视觉/真实语料验收节点提交 |
| 工作区 | 根目录文档、效果图、fixtures、Vue/Tauri/Rust 源码均已跟踪；`test-output/` 被忽略 | 运行证据与用户语料不进入源码提交 |
| 前端 manifest | 根 `package.json` 与 `apps/desktop/package.json` 存在 | Vue 3 + TypeScript + Vite 工程可生产构建 |
| Rust manifest | 根 `Cargo.toml` 与 crates workspace 存在 | Kernel、storage、Tauri host 均可测试和 Clippy |
| Tauri 配置 | `apps/desktop/src-tauri/tauri.conf.json` 存在 | debug 桌面应用可构建、启动并完成文件对话框流程 |
| 测试目录 | docs/design、docs/architecture、docs/adr、docs/testing 与 tests/fixtures 已创建 | Tiny fixture 用于确定性测试，真实语料用于兼容验收 |

上表反映 2026-08-28 收口状态；早期盘点中的缺失项已由实现阶段补齐。

### 2.2 文件清单与测试相关性

当前目录（排除 .git）中与后续验收有关的文件如下：

| 路径 | 大小/尺寸 | 分类 | 能否直接作为双语导入 fixture |
|---|---:|---|---|
| MVP效果图/单机批注.png | 1,405,608 bytes，1448×1086 | UI 参考图 | 否 |
| MVP效果图/平行修改.png | 1,276,405 bytes，1448×1086 | UI 参考图 | 否 |
| MVP效果图/平行阅读.png | 1,334,152 bytes，1448×1086 | UI 参考图 | 否 |
| MVP效果图/搜索与替换.png | 1,347,967 bytes，1448×1086 | UI 参考图 | 否 |
| MVP效果图/语句重排.png | 1,310,799 bytes，1448×1086 | UI 参考图 | 否 |
| MVP效果图/自动保存与历史.png | 1,081,645 bytes，1448×1086 | UI 参考图 | 否 |
| MVP效果图/暂时不实现-词性EOS.png | 1,273,170 bytes，1448×1086 | POS 延期参考图 | 否 |
| 最终版效果图-暂不用/*.png | 5 张，1672×941 | 未纳入 MVP 的参考图 | 否 |
| jueming-aligner-mvp-functional-spec-v0.1.md | 20,716 bytes | 产品规格 | 否 |
| jueming_global_architecture_handoff_v0.2.md | 65,489 bytes | 架构 handoff | 否 |
| jueming-aligner-mvp-implementation-plan-v0.2.md | 45,477 bytes | 实施计划 | 否 |
| tests/fixtures/government-report/report_zh.txt | 764 bytes，8 行 | Tiny source fixture | 是，UTF-8/LF 基线 |
| tests/fixtures/government-report/report_en.txt | 1,268 bytes，8 行 | Tiny target fixture | 是，UTF-8/LF 基线 |
| tests/fixtures/government-report/fixture.json | 407 bytes | fixture manifest/期望 8+8、8 个 1:1 | 是，导入期望依据 |
| A_阿古顿巴.txt | 24,104 bytes，GB18030，361 个物理行/309 个非空行 | 本地真实 source 语料，含 `<seg>` 包装和字间空格 | 是，使用旧版标注行 profile；不提交 Git |
| A_Akhu Tenpa.txt | 51,399 bytes，UTF-8，310 个物理行/309 个非空行 | 本地真实 target 语料，含 `<seg>` 包装和 POS 后缀 | 是，使用旧版标注行 profile；不提交 Git |
| 多语种语料平行对齐处理工具SISU Aligner 2.0.0 使用说明.pdf | 754,331 bytes | 旧版软件说明 | 否，不能替代 TXT fixture |
| SISU Aligner 2.0.0软件及说用说明.zip | 55,394,169 bytes | 旧版安装包归档 | 否；归档内仅有 exe 与说明 PDF |
| SISU Aligner 2.0.0.exe | 54,678,693 bytes | 旧版 Windows 程序 | 否；不能作为 Jueming MVP 被测应用 |

当前有两类双语验收材料：`tests/fixtures/government-report/` 下的 8+8 UTF-8 TXT 用于确定性断言与视觉截图；根目录“阿古顿巴”双语 TXT 用于 GB18030/UTF-8、旧版标注行和左右段数不等的真实兼容性验收。两类材料均已进入实际测试；阿古顿巴完整桌面路径的结果见 1.1。

### 2.3 剩余发布级缺口

1. 1:2、2:1、2:2、n:m、unlinked 已有 Kernel 测试，但仍需补充 Small、Medium、Stress 文件级 fixture。
2. TXT/JSON/XML 已有单元测试与 TXT 实机导出，三种格式的外部 golden round-trip 报告仍待发布阶段补齐。
3. 原子写入与重开已有测试，强制终止故障注入仍待执行。
4. 同尺寸结构比较已完成；125%/150% DPI 和窄窗口响应式截图仍待发布回归。
5. 旧版 SISU 只作为产品使用习惯和输入兼容参考，不是 Jueming 架构和功能验收证据。

## 3. Windows/Tauri 构建前置盘点

以下检查均为只读检查，版本是本机在盘点日返回的结果。

| 前置 | 结果 | 解释与后续动作 |
|---|---|---|
| Windows | Windows 11 家庭中文版，10.0.26200，64 位 | 满足桌面端测试环境；需在 Phase 0 冻结最低支持版本 |
| Git | git 2.53.0.windows.1 | 可用；首次基线提交仍待主任务决定 |
| rustc | 1.94.0，stable-x86_64-pc-windows-msvc | 可用 |
| cargo | 1.94.0 | 可用 |
| Rust target | x86_64-pc-windows-msvc 已安装 | 可用于 Windows Tauri 构建 |
| Rust quality tools | clippy、rustfmt 已安装 | 可用于质量门禁 |
| MSVC | Visual Studio 18 BuildTools；MSVC 14.50.35717 下有 x64/x86 cl.exe | 工具已安装，但 cl.exe 不在当前普通 PowerShell PATH；构建时使用 Developer PowerShell 或 vcvars 环境 |
| Windows SDK | 10.0.26100.0 目录下存在 x64/x86/arm64 signtool.exe | 具备打包签名相关工具；尚未验证签名证书 |
| WebView2 | Edge WebView2 Runtime 151.0.4129.107，注册表和安装目录均可见 | 满足 Tauri Windows WebView2 运行时前置；安装包仍需做干净环境 smoke test |
| Node | v24.14.0 | 可用；脚手架时固定兼容的 Node 版本范围 |
| npm | 11.9.0 | 可用 |
| pnpm | 11.19.0 | 可用，来自 Codex runtime fallback 路径；脚手架后应写入 packageManager 和 lockfile |
| Tauri CLI | cargo-tauri、tauri 均未找到 | 尚未安装；后续从项目本地 devDependency 或 cargo tauri CLI 固定版本，不在本盘点阶段安装 |
| PDF 文本工具 | pdftotext 未找到 | 旧版说明 PDF 目前只做资产清单，不作为 fixture 解析 |

构建前置的当前结论是“系统编译环境、固定项目工具链与应用工程均已可用”。旧版 SISU Aligner 2.0.0.exe 只用于参考，不得代替 Jueming 应用验收。

建议在 Phase 0 结束时补充并冻结：

- Tauri CLI、@tauri-apps/api、@tauri-apps/cli 的精确版本；
- Node/pnpm 版本和 lockfile；
- Tauri bundle 目标、应用 ID、安装包格式；
- Windows 最低版本、WebView2 安装策略；
- Developer PowerShell 构建命令和 CI 构建镜像；
- 测试机分辨率、DPI 缩放、字体和主题。

## 4. 建议测试 fixture

### 4.1 Tiny：截图回归与完整人工路径

目标：8+8 Segment，覆盖效果图中的主要文字和滚动高度，固定为 UTF-8、LF、无 BOM 的两个 TXT。当前已由并行 Phase 0 任务提供：

~~~text
tests/fixtures/government-report/report_zh.txt
tests/fixtures/government-report/report_en.txt
tests/fixtures/government-report/fixture.json
~~~

中文八句：

~~~text
过去一年，面对复杂严峻的外部环境和艰巨繁重的改革发展稳定任务，我们坚持稳中求进工作总基调，国民经济运行总体平稳、稳中有进。
经济结构持续优化，新动能加快成长，创新驱动发展战略深入实施。
就业物价总体平稳，居民收入稳步增长，民生保障有力有效。
我们深入推进高质量发展，加快构建新发展格局，经济发展质量和效益不断提升。
深化改革扩大开放，市场活力和社会创造力进一步激发。
生态文明建设扎实推进，绿色低碳转型取得积极进展。
政府自身建设不断加强，治理能力和服务水平持续提升。
这些成绩来之不易，是全国各族人民团结奋斗的结果。
~~~

英文八句使用效果图中的对应翻译，保留 curly apostrophe、逗号和连字符等 Unicode 字符：

~~~text
Over the past year, in the face of a complex and severe external environment and arduous tasks in reform, development, and stability, we adhered to the general principle of pursuing progress while maintaining stability. The national economy remained broadly stable and moved forward steadily.
The economic structure continued to improve, new growth drivers accelerated their development, and the innovation-driven development strategy was implemented in depth.
Employment and prices remained generally stable, residents’ incomes grew steadily, and people’s well-being and social security were effectively strengthened.
We advanced high-quality development in depth, accelerated the building of a new development paradigm, and continuously improved the quality and efficiency of economic growth.
Reforms were deepened and opening up was expanded, further stimulating market vitality and social creativity.
The building of ecological civilization was solidly advanced, and positive progress was made in the green and low-carbon transition.
The Government strengthened its own building, and governance capacity and service levels continued to improve.
These achievements are hard-won, achieved through the joint efforts of people of all ethnic groups across the country.
~~~

当前 fixture.json 的初始期望是 8 个 provisional 1:1。后续操作脚本在同一工程中把第三组改造成 1:2，把第五组改造成 2:1，另建一个 n:m 组并保留一条 unlinked，避免只测 happy path；这些操作不应回写破坏原始 Tiny fixture。

### 4.2 Alignment matrix fixture

建议另建一个短小的结构 fixture，使关系类型不依赖长文本：

| 关系 | source Segment | target Segment | 预期 |
|---|---|---|---|
| 1:1 | Z001 | E001 | 普通 provisional 或 manual link |
| 1:2 | Z002 | E002、E003 | Group/Link 后可见 |
| 2:1 | Z003、Z004 | E004 | Group 后可见 |
| 2:2 | Z005、Z006 | E005、E006 | 复杂组 |
| n:m | Z007、Z008、Z009 | E007、E008 | 关系 cardinality 正确 |
| unlinked | Z010 | 无 | 明确未连接，不能伪装为空白 1:1 |

expected-alignment.json 必须按 stable SegmentId 和 AlignmentId 描述，不能只按数组索引。重排后 JSON 中 ID 不变、顺序字段变化才是正确结果。

### 4.3 编码和输入异常 fixture

需要在创建 fixture 时明确记录编码和换行，至少包括：

- UTF-8 无 BOM、UTF-8 BOM；
- LF 与 CRLF；
- 空文件、只有空行、末尾多一个换行；
- 中英文混合标点、全角括号、中文引号、ASCII 与 curly apostrophe、em dash；
- 重复文本但不同 SegmentId；
- 超长单句、短句与多段空白；
- 查询用的正则字符：点号、括号、方括号、反斜杠、星号；
- 一侧行数比另一侧多，形成 unlinked；
- 无法以 UTF-8 解码的文件不得静默替换字符；可提示选择 GB18030 并先预览，其他未支持编码应明确拒绝。

### 4.4 规模 fixture

| 名称 | 规模 | 用途 | 当前状态 |
|---|---:|---|---|
| Tiny | 8+8 | 截图、全流程和手工 Computer Use | 已存在于 `tests/fixtures/government-report/` |
| Small | 1,000+1,000 | 常规 E2E、保存/搜索/导出 | 待确定性生成 |
| Medium | 50,000+50,000 | 虚拟列表、索引、保存和打开 | 待生成；不应提交巨大重复文件到 Git |
| Stress | 总计 500,000 Segment | Kernel Slice、索引、内存观察 | 待生成；建议运行时生成或使用受控 artifact |

生成器必须使用固定 seed、固定文本模板和可复现的 SegmentId，manifest 记录 seed、数量、语言、编码、换行与生成版本。

### 4.5 本地真实“阿古顿巴”语料

这两个文件由用户放在工作区根目录，只用于本机兼容性验收，不作为可再分发的 Git fixture。当前文件指纹：

- `A_阿古顿巴.txt`：SHA-256 `9997887AC697B437A2D27B1B98D5368B9B4B1C86584E5A0CB3D8C77A5319A29B`，GB18030；
- `A_Akhu Tenpa.txt`：SHA-256 `D0994FAF4D757420E41396F0EEF22BE0812BF6872A84CD7461814CDFF5B289DD`，UTF-8。

导入 profile 顺序为：解码 → 按非空物理行取候选 → 剔除 `<seg>`/`</seg>` 包装 → 英文词后 POS 后缀清理/中文字间空格紧缩 → 过滤空标记。预期结果为 source 307、target 308，因此初始布局是 307 个 provisional 1:1 和 1 个 target unlinked。清理结果必须在提交前可预览，且 SourceAsset 原始字节不被修改。

## 5. Computer Use 验收脚本（只描述，不执行）

### 5.1 通用前置与证据规则

1. 在脚手架、Tauri bundle 和 Tiny fixture 均已存在后，使用目标安装包或开发启动命令打开 Jueming Aligner；不得启动旧版 SISU exe 作为替代。
2. 固定窗口尺寸为效果图基线 1448×1086；若系统 DPI 为 125%/150%，同时保存实际物理像素和逻辑窗口尺寸。
3. 关闭其他遮挡窗口，使用统一 Light 主题、默认字体和语言；记录应用版本、Git commit、fixture manifest、Windows build、DPI。
4. 每个关键状态截图按“序号_场景_状态.png”命名，旁边保存 JSON 证据：当前模式、SegmentId/AlignmentId、RevisionId、保存状态和操作序列。
5. 每一步只通过可见 UI 和用户可观察的状态验证；需要核对导出内容、ID 和历史时，再用只读文件检查。
6. 出现数据损坏、半提交、无响应或无法恢复时立即保存截图和日志，停止该路径，不用刷新或重建工程掩盖问题。

### 5.2 主闭环脚本

#### CU-01 新建、导入与分句

1. 启动应用，确认顶栏、左侧导航、模式选择、底部状态栏出现。
2. 点击“新建”，输入工程名、保存目录、中文 Source、英文 Target。
3. 分别选择 Tiny 的 report_zh.txt 和 report_en.txt；确认显示文件名、字符数、行数和 UTF-8 解码成功。
4. 进入分句预览，核对两侧句边界、顺序和预览文本；修改一条规则或一个边界后取消，再确认不产生半成品工程。
5. 应用分句，确认每个 Segment 有稳定 ID，初始 8 个 provisional 1:1。
6. 截图：应用壳、导入状态、分句预览、初始平行阅读各一张。

#### CU-02 Review 与双向定位

1. 在 Review 模式点击中文第三组，确认英文侧定位到同一个 AlignmentId。
2. 从英文侧反向点击，确认中文侧跟随；检查没有两侧滚动反馈回路或视图抖动。
3. 展开上一段/下一段 Context Lens，确认当前 Alignment anchor 的视觉 offset 保持。
4. 筛选未对齐、书签或指定 Segment，确认跳转先加载局部 Slice，不要求目标原先就在 DOM。
5. 截图：当前组淡绿/淡黄高亮、连接区、上下文展开和未对齐状态。

#### CU-03 Link、Unlink、Group、Ungroup 与 Segment 内容结构

1. 选择未连接的 source/target，点击 Link；确认关系从 unlinked 变成 1:1，产生保存状态变化。
2. 选择已有关系，点击 Unlink；确认 Segment 保留而 Alignment 被删除。
3. 将一条 source 与两条 target Group 成 1:2；再对另一组 Group 成 2:1。
4. 创建 2:2 或 n:m，确认所有 refs 都属于同一个 Alignment 且不重复占用。
5. 对复杂关系执行 Ungroup，确认明确的分组结果；若无法确定语义，应 Unlink 后让用户重新 Link，而不是静默猜测。
6. 尝试把已属于不同 Alignment 的 Segment 直接重复 Link，确认弹出替换/先 Unlink 提示且没有错误 Revision。
7. 对同侧连续 Segment 做 Merge 内容，确认首项 ID 保留、被吸收 ID 离开 active 集合、书签/批注锚点被迁移；再 Split 内容，确认 parts 无损、首项 ID 保留、后续 ID 新建且先继承原 Alignment。
8. 尝试跨两个 active Alignment 内容 Merge，确认被拒绝或先展示明确 Group 后继续预览；每类 cardinality 保存截图，并在历史或只读导出中核对 AlignmentId。

#### CU-04 Edit 与 Undo/Redo

1. 切换 Edit，双击一个英文 Segment；确认 CodeMirror 编辑器、字数、拼写检查/状态、保存与取消控件出现。
2. 修改文本后按取消，确认原文和 Revision 不变；再次修改后保存，确认 SegmentId 和既有 AlignmentId 不变。
3. 在输入过程中切换模式，确认出现保存、放弃或取消切换守卫；不允许静默丢草稿。
4. 执行 Edit、Move、Merge 内容、Split 内容、Group、Ungroup 等操作，逐步 Undo 回到初态，再 Redo 到末态；保存每个关键状态。
5. 截图：编辑态、取消后的态、保存后的态、Undo/Redo 状态。

#### CU-05 Order

本迭代暂缓 drag 的自动化/Computer Use 验收；以下脚本保留为后续回归，不得作为当前完成证据。当前仅验证 Move Up/Move Down 的 stable-ID 语义与端点注册的集成契约。

1. 切换 Order 模式，确认工具栏显示拖动排序、上移、下移、恢复顺序。
2. 用拖动手柄移动中文第四组，确认插入位置提示；再用上移/下移完成一次等价操作。
3. 在跨 Alignment 边界重排，确认只改变 SegmentOrder/PositionKey，不自动重写 Alignment。
4. 返回 Review，确认两侧关系提示仍正确；只读核对 SegmentId 不变。
5. 截图：Order 工具栏、拖拽中插入标记、重排后关系。

#### CU-06 Search 与 Replace

1. 按 Ctrl+F，确认只打开当前 View Find；在当前编辑器、当前展开段或当前 Diff 内搜索。
2. 依次验证普通文本、区分大小写、基础正则、无结果和正则错误。
3. 按 Ctrl+Shift+F，打开 Project Search；限制中文或英文侧，检查结果显示 SegmentId、命中词、语言和 Alignment 状态。
4. 点击结果，确认跳回 Parallel View 并激活对应 Segment；确认不是遍历 DOM 或 Pinia 全文数组。
5. 打开替换预览，勾选部分结果后提交；确认 before/after、base revision、一次 ChangeSet 和单次 Undo 全部恢复。
6. 让命中集过期后提交，确认全批次拒绝，不发生部分替换。
7. 截图：搜索栏、结果表、平行预览、替换预览、撤销后状态。

#### CU-07 Bookmark 与单机批注

1. 给一个 1:2 Alignment 加书签，打开 Bookmark Panel 跳回；重排后再次跳转，确认按 SegmentId 而非位置。
2. 在正文添加 Draft 批注，关联一条中文和两条英文 Segment。
3. 编辑为 In Progress，再标记 Resolved；用侧栏状态筛选和正文编号跳转。
4. 删除批注，确认 Segment 文本、Alignment 和 Revision 语义未被错误修改。
5. 截图：正文标记、批注卡片、状态筛选、书签面板。

#### CU-08 Autosave、History、恢复

1. 完成编辑、重排、Alignment、书签和批注，观察“保存中/已保存/失败”状态。
2. 关闭并重新打开工程，确认文本、顺序、Alignment、书签、批注和当前 Revision 一致。
3. 在测试故障注入点终止进程，重新打开工程；确认最后完整 Revision 可见，不完整事务不可见。
4. 在 History 选择两个 Revision，检查 Text、Alignment、Project Operation 三种投影，确认 diff 的删除淡红/新增淡绿和 RevisionId。
5. 点击恢复旧版本，确认生成新 Revision，恢复前版本仍在时间线中。
6. 截图：自动保存状态、历史时间线、side-by-side/unified diff、恢复后的新 Revision。

#### CU-09 导出与关闭

1. 在同时包含 1:1、1:2、2:1、2:2、n:m、unlinked 的工程中分别导出 TXT、JSON、XML。
2. TXT 检查左右分隔规则和未对齐输出；JSON 检查稳定 ID、refs、顺序和 cardinality；XML 检查 alignment、source refs、target refs 和文本。
3. 对导出目标使用临时文件/原子替换语义进行失败测试，确认不会留下截断目标文件。
4. 重新打开工程，确认导出不会改变当前 Revision。
5. 截图：导出菜单、成功状态、错误状态；文件内容使用只读检查保存校验摘要。

#### CU-10 真实“阿古顿巴”文件

1. 新建一个独立工程，source 选择 `A_阿古顿巴.txt`，target 选择 `A_Akhu Tenpa.txt`。
2. 确认 source 的 UTF-8 严格解码失败不会生成乱码；显式选择 GB18030，target 保持 UTF-8。
3. 选择“旧版标注行” profile，在导入预览核对标题与前两个句段不含 `<seg>`、POS 后缀或中文字间空格。
4. 核对 source 307、target 308、307 个 provisional 1:1 和 1 个 target unlinked；记录预览计数和首屏截图。
5. 保存、关闭、重开，对首段/中间段/末段进行搜索和跳转，再导出 JSON；稳定 ID、文本与 unlinked 计数不变。
6. 只读重算两个原文件 SHA-256，确认导入未改写用户文件。

### 5.3 负向与兼容路径

- 选择不存在或无法读取的文件：显示明确错误，不创建半工程。
- 选择 UTF-8 BOM：内容不出现 BOM 字符。
- 选择非 UTF-8 文件：不静默替换无效字节；对 GB18030 给出可见预览与显式选择，对未支持编码明确拒绝。
- 删除/移动已被书签或批注引用的 Segment：锚点进入 orphaned 或按合同拒绝，不静默指向新 Segment。
- 关闭 Edit 草稿、进行过期 Replace、重复占用 Alignment：均应有守卫和可追踪错误。
- 关闭 WebView2 或在缺失 runtime 的隔离机安装：验证安装包的依赖提示和失败诊断；不要将该环境的启动失败归因于业务 UI。
- 访问 POS 效果图对应入口：MVP 必须显示 UNBOUND/延期状态，不能假装实现 token/POS。

## 6. MVP 效果图截图证据清单

参考图都为 1448×1086。后续截图比较应使用同一窗口和 DPI，并将“布局差异”和“功能状态差异”分开记录。

| 参考图 | 必须采集的 app 状态 | 视觉/交互核对点 | 负向证据 |
|---|---|---|---|
| MVP效果图/平行阅读.png | Review，选中 000104 类似组 | 顶栏、导航、双栏、中央连接区、当前左右高亮、底部状态栏、有限上下文 | 不出现整库 DOM 渲染或伪造 1:1 |
| MVP效果图/平行修改.png | Edit，单句双栏编辑 | CodeMirror 编辑区、保存/取消、字数/状态、配对侧稳定居中、前后句仍可见 | 编辑不改变 SegmentId/AlignmentId |
| MVP效果图/语句重排.png | Order，拖动中 | 排序工具栏、拖动手柄、插入虚线标记、上下移/恢复顺序、两侧关系提示 | 不把 index 直接作为 canonical 身份 |
| MVP效果图/搜索与替换.png | Project Search + replacement preview | 查询栏、正则/大小写/当前工程选项、结果 ID/上下文/命中高亮、底部平行预览 | Ctrl+F 不承担 Project Search；替换无部分提交 |
| MVP效果图/单机批注.png | Annotation，侧栏有 Draft/In Progress/Resolved | 正文批注编号、悬浮摘要、右侧筛选/列表、关联 Segment、编辑/删除/解决 | 不依赖账号/云协作；HumanAnnotation 不混入 POS |
| MVP效果图/自动保存与历史.png | History，比较 R117→R128 类似版本 | 时间线、当前版本、side-by-side diff、变更摘要、恢复按钮、删除/新增颜色 | Restore 创建新 Revision，不覆盖旧历史 |
| MVP效果图/暂时不实现-词性EOS.png | POS 入口或 Slot 状态页 | 如展示入口，必须明确 token/POS provider 的 UNBOUND/延期状态 | 不实现 POS/Lemma/NER/词性覆盖层 |

截图报告每张至少附：

- 参考图绝对路径、被测截图绝对路径；
- 窗口宽高、系统 DPI、主题、字体；
- app 版本与 Git commit；
- fixture/工程 RevisionId；
- 通过/失败及差异说明；
- 若失败，标注是结构布局、颜色/间距、文字内容、状态逻辑还是响应式问题。

推荐比较顺序：先比区域拓扑（顶栏/导航/正文/状态栏），再比关键组件位置和尺寸，最后比颜色、字体、边框、图标和文字。不要只用整图像素相似度证明交互完成。

## 7. 性能基线与测量方案

### 7.1 基准环境记录

当前盘点得到的基准机记录：

~~~text
OS: Windows 11 Home zh-CN, build 26200, x64
rustc/cargo: 1.94.0
node/npm/pnpm: 24.14.0 / 11.9.0 / 11.19.0
WebView2: 151.0.4129.107
MSVC: 14.50.35717, Windows SDK 10.0.26100.0
GPU/CPU/RAM: 待 Phase 0 补采
DPI/显示器/字体: 待 Phase 0 固定
~~~

GPU、CPU、内存、磁盘、显示器和 DPI 必须在第一次真正 benchmark 前补齐；在此之前只能报告环境版本，不能报告可比较的性能数字。

### 7.2 目标与测量

| 指标 | 初始门槛（来自实施计划） | 测量方式 | 证据 |
|---|---|---|---|
| 100,000 Segment 普通搜索 | 已建索引 P95 < 1 秒 | Medium/合成 100k fixture，预热后至少 30 次，记录 P50/P95/P99 | Kernel timer、查询参数、fixture manifest |
| 可见 Slice 编辑提交 | 到界面更新 P95 < 100 ms | 固定可见 Slice，执行 30 次保存，记录 command 到 UI ack | Rust/TS trace 和截图 |
| 虚拟列表 DOM | 不 materialize 全部 Segment | 运行 Medium/Stress，记录 DOM 节点、virtualizer range、内存 | 浏览器/应用诊断快照 |
| 自动保存 | 不阻塞输入主线程 | 连续输入并触发 autosave，记录输入延迟、保存耗时和 dropped frame | trace、SaveState 事件 |
| 冷/热打开 | 随元数据与首屏 Slice 增长，不随全量正文线性物化 | 冷启动、二次启动和随机 Segment 跳转分别测量 | 启动日志、首屏截图 |
| Stress 内存 | resident memory 明显小于工程磁盘体积 | 记录进程 private working set、工程大小、cache 大小 | Windows 性能计数器/日志 |

测量要求：每次运行记录 commit、fixture seed、构建模式、是否预热、运行次数和异常；切勿把不同 fixture 或不同 DPI 的结果混在一个基线中。CI 中 Tiny/Small 必跑，Medium 定期跑，Stress 作为发布前和性能专项。

## 8. 验收矩阵与退出条件

| ID | 覆盖能力 | 关键证据 | 通过条件 |
|---|---|---|---|
| T01 | 创建/导入/重开 | CU-01、CU-08、工程 manifest | UTF-8/BOM 内容、ID、顺序、关系可 round-trip |
| T02 | 分句 | CU-01 | 预览可见、规则边界可修正、无半成品 |
| T03 | 1:1/1:2/2:1/2:2/n:m/unlinked | CU-03、CU-09 | 每种关系保存、重开、撤销、导出均正确 |
| T04 | Review/双向定位/Context | CU-02、平行阅读截图 | AlignmentId anchor 正确，无滚动回路 |
| T05 | Edit/Undo/Redo | CU-04、平行修改截图 | ID 不变，草稿守卫有效 |
| T06 | Order | CU-05、语句重排截图 | stable ID 不变，Alignment 不被自动改写 |
| T07 | Search/Replace | CU-06、搜索截图 | View Find/Project Search 分界明确，批量替换原子化 |
| T08 | Bookmark/Annotation | CU-07、批注截图 | SegmentId 锚点在重排后仍有效，状态可筛选 |
| T09 | Autosave/History/Restore | CU-08、历史截图 | 故障后无半 Revision，恢复只追加新 Revision |
| T10 | Export | CU-09、三种 golden files | TXT/JSON/XML 无信息损失，失败不截断目标 |
| T11 | Visual parity | 七张 MVP 参考图 | 结构、状态、层级与参考图一致；延期 POS 不冒充已实现 |
| T12 | Performance | 第 7 节报告 | 达到冻结后的门槛且可复现 |
| T13 | GB18030/旧版标注行真实语料 | CU-10、导入计数、原文件 hash、导出 JSON | 307+308 段可预览、保存、重开、搜索和导出；无乱码、无标记泄漏、原文件未改写 |

当前 T01–T11 和 T13 的核心路径已有自动化或 Windows 实机证据；T12 性能、故障注入和干净环境安装仍属于发布级待执行项。具体通过范围以 1.1、1.2 和各测试命令结果为准，不把尚未执行的专项外推为通过。

## 9. 发布前后续动作

1. 为 Small、Medium、Stress 规模补建文件级 fixture，记录 CPU/GPU/RAM/磁盘/DPI 并冻结 benchmark 机器。
2. 执行自动保存强制终止故障注入，确认工程只能回到最后完整 Revision。
3. 在干净 Windows 环境检查安装包、WebView2 依赖提示、首次启动、三格式导出和卸载残留。
4. 补做 125%/150% DPI、键盘无障碍和窄窗口回归；必要时形成可版本化的图像差异阈值。
5. 给 TXT/JSON/XML 建立外部 golden files 与 round-trip 报告。
6. 继续以关键交付节点提交 Git；截图和真实工程保留在忽略目录，不提交用户语料与运行产物。
