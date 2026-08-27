# 决明对齐器（Jueming Aligner）

决明对齐器是一个本地优先的双语平行文本阅读、人工对齐与审校桌面应用。首个 MVP 使用 Tauri 2、Vue 3、TypeScript 与 Rust Kernel，围绕稳定的 `SegmentId`、`AlignmentId` 和 `RevisionId` 完成导入、阅读、编辑、重排、搜索、批注、历史和导出闭环。

## 当前状态

MVP Phase 1 核心垂直链路已接通：Tauri 文件选择、每侧独立编码/分段预览、稳定 ID、暂定布局、`.jm` 原子持久化、打开/保存、编辑和重排 Revision 已实现。政府报告 fixture 的 create/open、编辑重开、重排稳定 ID 与多余目标段未链接均有测试；前端生产构建、Rust Clippy/test 和 Windows 调试可执行文件构建已通过。下一阶段是人工 Alignment、虚拟化以及真实桌面验收。

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
  → SQLite + Operation Log + Chunk/Slice Storage
```

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
