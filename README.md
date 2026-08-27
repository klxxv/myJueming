# 决明对齐器（Jueming Aligner）

决明对齐器是一个本地优先的双语平行文本阅读、人工对齐与审校桌面应用。首个 MVP 使用 Tauri 2、Vue 3、TypeScript 与 Rust Kernel，围绕稳定的 `SegmentId`、`AlignmentId` 和 `RevisionId` 完成导入、阅读、编辑、重排、搜索、批注、历史和导出闭环。

## 当前状态

MVP 功能代码已完成本地桌面闭环：Tauri 文件选择、双侧独立编码/分段预览、稳定 ID、暂定布局、`.jm` 原子持久化、可变高度虚拟列表、人工 Link/Unlink/Merge/Split、编辑与拖拽重排、Undo/Redo、搜索/原子替换、书签、单机批注、持久 History、TXT/JSON/XML 导出及本地外观设置均已接入 Rust Kernel。Windows Tauri 已用根目录真实“阿古顿巴”文件完成 GB18030/UTF-8 导入、307×308 不等段布局、编辑、搜索、重排、书签、批注、历史、TXT 导出和关闭重开；Review、Edit、Order、Search、Annotation、History 六个 P0 状态已完成同尺寸效果图结构验收。Rust 全工作区测试（26 项）、Clippy `-D warnings`、前端生产构建与 Tauri debug 构建均已通过。POS、Lemma、自动语义对齐、OCR、云协作和外部插件加载明确延期。

## 主要文档

- [MVP 实施计划](./jueming-aligner-mvp-implementation-plan-v0.2.md)
- [MVP 功能规格](./jueming-aligner-mvp-functional-spec-v0.1.md)
- [全局架构 Handoff](./jueming_global_architecture_handoff_v0.2.md)
- [MVP 效果图](./MVP效果图/)
- [Phase 0 架构合同](./docs/architecture/mvp-phase0-contracts-v0.1.md)
- [ADR 索引](./docs/adr/000-index.md)
- [MVP 视觉规范](./docs/design/mvp-visual-spec-v0.1.md)
- [测试与 Computer Use 验收计划](./docs/testing/mvp-test-fixtures-and-computer-use-plan-v0.1.md)
- [品牌图标源文件](./assets/brand/jueming-aligner-icon-source.png)

## MVP 技术主线

```text
Tauri 2 Desktop
  → Vue 3 ParallelWorkspace
  → TypeScript KernelClient
  → Tauri Command / Query / Event
  → Rust Kernel
  → .jm Atomic Snapshot + Append-only Revision Storage
```

当前工程格式 v0.1 在 `jueming-storage` 边界内使用原子 JSON snapshot 与逐 Revision 文件；未来切换 SQLite/Chunk backend 不改变 `KernelClient` 或核心 ID 合同。

以下命令从仓库根目录运行。

## 开发命令

```powershell
pnpm install
pnpm build
pnpm typecheck
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
pnpm --dir apps/desktop tauri build --debug --no-bundle
```

当前工具链锁定在 `pnpm-lock.yaml` 和 `Cargo.lock`；开发器运行 `pnpm dev`。
