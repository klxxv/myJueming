# 2026-09-15 主分支合并验证

## 合并范围

- 合并前 `main`／`origin/main`：`7e7d70d`，已 fetch 确认一致。
- `codex/agent-mcp-pipeline`：`fdf4aac`，包含本地 Agent 宿主、MCP、全局面板及处理流程。
- `anime-dev` 原提交：`e3b940d`，继承上述分支，包含研究运行时、多译本、导入编码识别等既有实现。
- 整理提交 `1e35251`：接入已有 Kernel 多语言文案，修正测试对系统语言和首次引导的隐式依赖。
- 整理提交 `19bfc90`：保留并提交当前主题、系统外观跟随、阅读进度与布局测量改动，补齐相关回归和实施记录。
- 合并策略：在干净工作区依次将 `codex/agent-mcp-pipeline`、`anime-dev` 快进合入本地 `main`，保留两个来源分支和全部历史；远端推送不属于本次操作。

## 验证结果

本机使用 Node 24.19.0、仓库锁定的 pnpm 11.19.0；环境未提供 Corepack，使用同版本 pnpm 执行。依赖冻结安装通过，未升级锁定版本。

| 检查 | 结果 |
| --- | --- |
| `pnpm install --frozen-lockfile` | 通过 |
| `pnpm typecheck`、`pnpm build` | 通过 |
| `cargo fmt --all -- --check` | 通过 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 通过 |
| `cargo test --workspace` | 125 项通过，2 项按既有配置忽略 |
| `pnpm test:agent-ui` | 生产合同全部通过；包含 10,000 个对齐块的测量批处理、裁剪和生命周期清理 |
| `node tests/agent/m1-real-subprocess-acceptance.mjs` | 真实 MCP 子进程工具调用、原生专属操作隔离、搜索及 AG-UI SSE 事件通过 |
| `pnpm test:i18n` | 五语言 1744 个键、50 个设置页面渲染及 Vue 源码审计通过 |
| Chromium／WebKit 主题浏览器回归 | 三主题、系统跟随组合、持久化、增强对比度、审阅／编辑／排序及进度条导航通过 |
| `node tests/themes/native-fallback-regression.mjs` | 原生通知替身、媒体事件、手动覆盖、监听释放通过 |
| `node tests/architecture/browser-regression.mjs` | 800 对句段初始仅加载及渲染 34 条正文；远距离滚动、草稿保存失败／取消／重试、模式切换、查找、批注重试通过 |
| `node tests/comparison/browser-regression.mjs` | Chromium 多列关系、虚拟化、列拖拽／缩放、文档对切换、解除关系和模式导航通过 |
| `pnpm --dir apps/desktop tauri build --debug --no-bundle --target universal-apple-darwin` | 最终代码构建通过；`lipo -archs` 确认为 `x86_64 arm64` |
| `git diff --check` | 通过 |

浏览器脚本以 `http://127.0.0.1:1420` 为入口；主题使用演示数据，跨层交互使用确定性 Host 替身，不修改用户真实工程。本次视觉检查覆盖暗黑审阅、暗黑设置及护眼设置。日志和截图位于本机临时目录 `/tmp/jueming-merge-20260915/`，不纳入 Git。

## 验证边界

- 两项忽略测试需要包含托管 Python／固定 XLM-R 权重的 `JUEMING_RESEARCH_BUNDLE`，本次未执行真实离线模型验收。
- 生产构建仍有大于 500 kB 的 JavaScript chunk 提示；构建成功，未通过调整阈值隐藏提示。
- 本次未在 Windows 实机执行安装包验收，也未执行签名、DMG／NSIS／MSI 发布或真实系统主题切换验收。
- 工作流保留 Windows x64 与 macOS Universal 矩阵。Universal、debug 和 no-bundle 参数依据 [Tauri CLI 官方文档](https://v2.tauri.app/reference/cli/#build) 核对；Action 的项目路径及参数透传依据 [tauri-action 官方说明](https://github.com/tauri-apps/tauri-action) 核对。本次没有改写打包工作流。
