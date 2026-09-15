# 决明桌面端

Tauri 2 + Vue 3 Composition API + TypeScript strict，支持 Windows 与 macOS。安装、运行和打包命令统一维护在[根 README](../../README.md)。

## 目录职责

- `src/components/`：工作区、侧栏、设置与复用界面组件。
- `src/composables/`：界面状态、生命周期与交互协调。
- `src/domain/`：类型化客户端、领域 DTO 和前端投影；界面不直接访问存储。
- `src/i18n/`：离线语言资源。
- `src/styles.css`、`tailwind.config.ts`：主题变量与语义样式。
- `src-tauri/`：原生命令、Rust 入口、能力与打包配置。
- `src-tauri/resources/research/`：本地生成的研究资源；见该目录 README。

主平行视图使用 `AlignedWorkspaceViewport` 与 `useAlignedBlockLayout`；搜索结果使用 TanStack Virtual。原生操作通过 typed façade 进入 Rust Kernel。开发地址固定为 `http://127.0.0.1:1420`。

跨层修改遵循[架构合同](../../docs/architecture/mvp-phase0-contracts-v0.1.md)和[开发约定](../../AGENTS.md)。
