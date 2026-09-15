# 决明对齐器 · Jueming Aligner

本地优先的双语平行文本阅读、人工对齐与审校桌面应用，面向 Windows 和 macOS。核心编辑流程离线可用，工程以 `.jm` 目录保存在本机。

[下载安装包](https://github.com/klxxv/myJueming/releases) · [构建状态](https://github.com/klxxv/myJueming/actions/workflows/package-desktop.yml) · [文档导航](docs/README.md) · [开发约定](AGENTS.md)

## 可以做什么

- **导入与阅读**：双语 TXT、独立编码与分段预览、多译本工程、虚拟化平行阅读。
- **人工对齐与审校**：建立或解除关系、关系分组、句段合并与拆分、文本编辑和拖拽重排，支持 1:1、1:n、n:1、n:m。
- **查找与标记**：当前视图查找、工程搜索与替换预览、书签、关联句段的人工批注。
- **保存与追溯**：本地持久化、撤销与重做、版本历史与恢复，导出 TXT / JSON / XML。
- **工作环境**：明亮、暗黑、护眼主题，中英法界面，以及全局侧栏和小花园。

仓库还包含可选的 Agent、MCP、本地模型连接和 Pipeline 研究模块。模型助手需要配置可用的服务；HTTPS 模型服务需要联网。研究功能取决于本机资源包和能力状态，详见[研究运行时](docs/architecture/local-research-runtime-implementation-v0.1.md)。这些模块的实现与验收记录分别维护在文档中，历史验收不代表当前提交的 CI 状态。

## 下载与使用

在 [Releases](https://github.com/klxxv/myJueming/releases) 中选择平台：

| 平台 | 产物 |
| --- | --- |
| Windows x64 | NSIS 安装程序、MSI、便携 EXE、含研究资源的便携 ZIP |
| macOS Intel / Apple Silicon | Universal DMG、包含 `.app` 的便携 ZIP |
| 外部工具连接 | 独立 Windows / macOS Universal MCP 服务端 |

打开应用后新建工程、导入原文与译文，在平行工作区完成审校，再导出所需格式。备份时复制整个 `.jm` 目录，保留 `revisions/` 正式历史。

快捷键与系统要求见[桌面平台说明](docs/platform/desktop-platform-guide-v0.1.md)。

## 本地开发

需要 Node.js **24.14.0 或以上**、pnpm **11.19.0**、Rust **1.94 或以上**，以及对应平台的 [Tauri 2 系统依赖](https://v2.tauri.app/start/prerequisites/)。具体依赖版本以 `package.json`、`pnpm-lock.yaml`、`Cargo.toml` 和 `Cargo.lock` 为准。

在仓库根目录运行；如已配置 Corepack，可将 `pnpm` 替换为 `corepack pnpm`：

```sh
pnpm install --frozen-lockfile
pnpm dev
```

`pnpm dev` 启动 Tauri 桌面开发程序；`pnpm dev:web` 只启动浏览器 UI。开发地址固定为 `http://127.0.0.1:1420`。浏览器预览不提供原生文件操作与完整桌面能力。

### 验证

```sh
pnpm typecheck
pnpm build
pnpm test:agent-ui
pnpm test:i18n
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
pnpm --dir apps/desktop tauri build --debug --no-bundle
```

### 打包

```sh
# 在 Windows 上生成 NSIS / MSI
pnpm build:desktop:windows

# 在 macOS 上生成 Intel + Apple Silicon Universal app / DMG
rustup target add aarch64-apple-darwin x86_64-apple-darwin
pnpm build:desktop:macos
```

完整离线研究资源需要额外准备，步骤以[发布工作流](.github/workflows/package-desktop.yml)和[资源说明](apps/desktop/src-tauri/resources/research/README.md)为准。

## 项目结构

| 路径 | 内容 |
| --- | --- |
| `apps/desktop/src/` | Vue 3 / TypeScript 界面、领域 façade、布局与主题 |
| `apps/desktop/src-tauri/` | Tauri 2 桌面壳、原生命令与打包配置 |
| `crates/jueming-{core,protocol,kernel,storage}/` | 领域模型、协议、操作与工程持久化 |
| `crates/jueming-application/` | 共享应用宿主与工程会话 |
| `crates/jueming-agent-{runtime,transport}/`、`crates/jueming-mcp/` | 模型运行时、连接适配与 MCP 服务端 |
| `crates/jueming-pipeline/`、`schemas/`、`plugins/` | 研究方法、数据合同与本地算法包 |
| `tests/`、`scripts/` | 回归测试、资源生成与平台构建脚本 |
| `docs/` | 架构合同、ADR、设计、平台与验收资料 |
| `assets/`、`MVP效果图/` | 品牌母版与视觉参考 |

界面通过 typed façade 进入 Rust；稳定 ID 与显示顺序分离，重排不改变对齐关系。人工批注独立存储，成功的数据修改追加 Revision。详细约束见[架构合同](docs/architecture/mvp-phase0-contracts-v0.1.md)和 [ADR](docs/adr/000-index.md)。

## GitHub Actions

[Package desktop](.github/workflows/package-desktop.yml) 在 `main`、`codex/agent-mcp-pipeline` 的 push、面向 `main` 的 PR 和 `v*` 标签上运行，也支持手动触发。

- Windows：类型检查、前端合同与构建、Rust 格式 / Clippy / 测试、真实 MCP 子进程验收。
- macOS：类型检查、Universal 桌面编译、双架构 MCP 构建检查。
- `v*` 标签：版本匹配检查后，等待两端门禁通过，打包并发布 GitHub Release；普通分支手动运行只执行验证。

排障与手动运行方法见 [CI 维护说明](docs/testing/ci-maintenance.md)。

## 参与开发

先阅读 [AGENTS.md](AGENTS.md) 与[文档导航](docs/README.md)。新增代码按现有模块归属放置，测试保留在相应领域目录；构建输出、凭据和真实用户语料不纳入版本控制。变更提交前执行相应验证和 `git diff --check`。

## 许可证

[MIT](LICENSE)
