# 决明 Agent / MCP 桌面交付说明 v0.1

## 交接：MCP stdio 二进制

本文件是 transport worker 与主集成者的发布交接。transport worker 注册新 crate 后，必须满足以下可由 CI 调用的接口：

```text
crate manifest: crates/jueming-mcp/Cargo.toml
Cargo package:  jueming-mcp
binary target:  jueming-mcp
```

二进制是本地 stdio MCP server，只通过 `LocalAppHost` 的应用服务访问工程；它不是 Tauri sidecar，也不由桌面包启动。因而 Tauri 配置中不应声明尚未存在的 externalBin，也不应让 Vue 或外部客户端绕过 host 直接访问存储。

在 crate manifest 被提交前，工作流不会引用它或期待 MCP 发布产物。manifest 存在后，标签构建会运行下列实际命令：

```text
# Windows x64
cargo build --locked --package jueming-mcp --bin jueming-mcp --release

# macOS Universal
cargo build --locked --package jueming-mcp --bin jueming-mcp --release --target aarch64-apple-darwin
cargo build --locked --package jueming-mcp --bin jueming-mcp --release --target x86_64-apple-darwin
lipo -create <arm64> <x86_64> -output target/universal-apple-darwin/release/jueming-mcp
```

macOS 合并产物必须通过 `lipo -archs` 检查为同时含有 `arm64` 与 `x86_64`，再以 ZIP 发布以保留可执行文件属性。Windows 以 `.exe` 发布。发布文件名固定为：

```text
Jueming-MCP_vX.Y.Z_windows-x64.exe
Jueming-MCP_vX.Y.Z_macos-universal.zip
```

用户将 MCP server 放入受信任目录后，可让 MCP client 将该文件作为 stdio command 启动。macOS 从网络下载的独立二进制可能被 Finder 标记 quarantine；只有在确认下载来源可信且 Gatekeeper 阻止启动时，才可移除该文件的 quarantine 属性：

```bash
xattr -d com.apple.quarantine /absolute/path/to/jueming-mcp
```

该命令不进行签名或公证，也不替代它们。当前工作流没有 Apple Developer ID 证书或 notarization secret；正式 macOS 分发应在证书和 notarization 流程到位后验证。

## 桌面应用构建合同

Windows x64 继续生成 NSIS、MSI 和无安装 portable `.exe`。macOS runner 安装 Rust 的 `aarch64-apple-darwin` 与 `x86_64-apple-darwin` targets，并调用：

```bash
pnpm --dir apps/desktop tauri build --target universal-apple-darwin --bundles app,dmg
```

输出根目录为 `target/universal-apple-darwin/release/bundle`。工作流在打包前校验 `.app/Contents/MacOS` 中唯一可执行文件的 `lipo -archs` 结果，随后发布 Universal DMG 和保留 `.app` 元数据的 portable ZIP。Tauri 官方文档确认该虚拟目标生成同时支持 Apple Silicon 与 Intel 的 Universal App；DMG 是面向非 App Store 分发的常见安装封装。[Tauri macOS build guide](https://v2.tauri.app/distribute/app-store/) [Tauri DMG guide](https://v2.tauri.app/distribute/dmg/)

## CI 触发和验证范围

`package-desktop.yml` 在 `main`、`codex/agent-mcp-pipeline` 的 push 和面向 `main` 的 pull request 执行质量门禁。Windows job 执行 TypeScript typecheck/build、Rust format、Clippy 和 workspace tests；macOS job 在每次这些非发布 CI 触发中执行 typecheck、Universal debug no-bundle 构建和 Mach-O 双架构检查。仅 `v*` tag 可进入 package/release jobs，因此 feature push 与 pull request 不会创建 GitHub Release 或发布下载。

macOS CI 证明 GitHub 的 macOS runner 能编译 Universal 工件，但不替代真实 Intel 与 Apple Silicon 硬件上的界面、权限、性能和 Gatekeeper 测试。Windows 主机也不能验证 `lipo`、DMG、codesign 或 macOS 启动行为。
