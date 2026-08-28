# 决明对齐器决策总览

_更新时间：2026-08-29。本文是仓库级快速入口；正式架构依据仍是 [`docs/adr/`](docs/adr/000-index.md) 与 [Phase 0 合同](docs/architecture/mvp-phase0-contracts-v0.1.md)。_

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

### 主题与视觉

- 当前项目不使用 Tailwind；主题由全局语义 CSS 变量和组件 scoped CSS 组成。
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

## 明确延期

MVP 不实现 POS、Lemma、NER、自动语义对齐、OCR、云协作和外部插件加载。新增这些能力前，应先确认 Slot/Artifact 合同、资源与安全边界，并新增 ADR，而不是直接把 Provider 逻辑嵌入 Vue 组件或现有 Segment schema。

## 决策变更流程

1. 先说明要改变的现有不变量及原因。
2. 新增 ADR，记录 Context、Decision、Consequences 和迁移方案。
3. 更新 `docs/adr/000-index.md`、Phase 0 合同版本及本总览。
4. 再修改 Rust Core、KernelClient、前端交互和持久化实现。
