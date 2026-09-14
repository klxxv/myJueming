# 架构审查前四项修复记录

日期：2026-09-13。范围：本地工程写入安全、原生命令合同、编辑草稿切换、工作区数据边界。

## 修复对应关系

| 审查问题 | 当前实现 | 回归证据 |
| --- | --- | --- |
| 多个 Host／进程可覆盖同一 Revision | Host 持有工程 OS 排他写锁；Kernel 提交时另加短期锁并校验磁盘版本。重开同一工程复用锁，新建拒绝覆盖已有工程 | 两个 Host 竞争、独立子进程锁竞争和释放、直接 Kernel 旧快照写入拒绝、历史文件字节保持不变 |
| 原生命令缺少工程／版本／幂等校验 | canonical 修改统一走 `execute_command`，检查 `command_id/project_id/base_revision_id`；回执随 Revision 持久化，重试返回原提交版本 | 旧版本、错误工程、同 ID 异参数拒绝；同请求重复、重启后重试、Restore 后重试不增加 Revision |
| 打开／新建工程绕过编辑草稿 | 复用模式状态机的保存／放弃／继续编辑守卫；保存失败保留输入；放弃前等待正在执行的保存 | 生产 composable 验证保存失败、取消、原版本同 ID 重试及等待在途保存；真实 Vue 页面验证打开工程被拦截 |
| canonical 全量快照用于 IPC、UI 与 Agent 同步 | `WorkspaceProject` 返回结构、hash／长度、摘要与 sidecar；正文通过 revision-bound `ParallelSlice` 按需读取；Agent 投影只携带 `ProjectIdentity` | 无正文／存储引用的投影断言，Slice 范围／版本／锚点验证，缓存淘汰和旧响应拒收，页面远距离滚动及 View Find |

Undo／Redo／Restore 生成的新 Revision 的 parent 统一指向操作前的当前版本。编辑与批注草稿保留其起始工程和版本，失败重试保留命令身份；内容合并／拆分对话框拒绝过期确认。恢复两侧排序时逐次采用已提交的新版本。

## 主平行视图的实现基准

按当前用户要求，主平行视图继续使用 `AlignedWorkspaceViewport` 与 `useAlignedBlockLayout`：Alignment Block 布局、双侧高度计算、ResizeObserver 测量、可视区裁剪、overscan 和稳定 ID 跳转锚定。

本次接入正文 Slice，并使用未加载正文的实际长度估算行高；加载完成后沿用现有测量路径。TanStack Virtual 继续用于搜索结果列表。现行约定已同步至 `AGENTS.md`、`DECISIONS.md` 和 MVP 实施计划；Phase 0 与 Agent 文档已同步数据边界。

## 验证

在 macOS 本机完成：

- `pnpm typecheck`、`pnpm build`。
- `cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings`。
- `cargo test --workspace`：96 个测试通过。
- `pnpm test:agent-ui`：含新增架构合同回归，全部通过。
- `pnpm --dir apps/desktop tauri build --debug --no-bundle`。
- `git diff --check`。

建议在正常开发环境使用仓库规定的 `corepack pnpm`。本机无 Corepack，验证使用可用的 pnpm 运行相同脚本，未安装或更新项目依赖。

页面脚本：`tests/architecture/browser-regression.mjs`。先运行 `pnpm dev:web`，确保 Playwright 与 Chromium 可用，再运行：

```sh
node tests/architecture/browser-regression.mjs
```

Playwright 位于外部工具环境时可用 `PLAYWRIGHT_MODULE` 指向该环境的入口；可选 `ARCHITECTURE_SCREENSHOT` 指定截图文件。

脚本加载真实 App，模拟 Tauri IPC 与 800 对句段：首屏请求和渲染 34 条正文，滚动到第 800 对能够加载；保存失败阻止工程打开、取消保留草稿、重试沿用同一命令 ID；Review／Order 切换、View Find 和批注保存失败重试通过。

## 验证边界

页面回归使用 Chromium 与模拟 IPC；真实磁盘、跨进程锁和命令合同由 Rust 测试验证，Tauri 由本机 debug 构建验证。本轮未做 Windows 原生窗口与安装包验证。

本次去除了 canonical 全量快照跨工作区 IPC 的依赖。Rust 仍持有 canonical 内存快照，工作区结构仍按版本整体同步；未引入磁盘 Chunk 存储、远程传输或云端同步。显式 Slice 每批最多 200 个稳定 ID，常规正文缓存保留最多 1200 条并保护当前可见／请求中的内容；需要同时编辑的内容不在使用途中淘汰。View Find 批量扫描正文但只保留命中 ID，耗时仍随工程正文量增长。
