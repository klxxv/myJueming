# 决明对齐器桌面平台说明 v0.1

## macOS 快捷键

设置页的“快捷键布局”默认自动识别系统，也可以手动切换 macOS 或 Windows / Linux 布局。macOS 使用 Command 作为主修饰键，并保留文本输入框的系统原生撤销栈。

| 操作 | macOS | Windows / Linux |
| --- | --- | --- |
| 新建工程 | `⌘N` | `Ctrl+N` |
| 打开工程 | `⌘O` | `Ctrl+O` |
| 保存工程 | `⌘S` | `Ctrl+S` |
| 审阅内查找 | `⌘F` | `Ctrl+F` |
| 下一处 / 上一处 | `⌘G` / `⇧⌘G` | `Ctrl+G` / `Ctrl+Shift+G` |
| 工程撤销 / 重做 | `⌘Z` / `⇧⌘Z` | `Ctrl+Z` / `Ctrl+Y` |
| 保存当前句段并退出 | `⌘Enter` 或 `Esc` | `Ctrl+Enter` 或 `Esc` |

`⇧⌘F` / `Ctrl+Shift+F` 打开工程级搜索。工程撤销与重做只在焦点不位于输入框时接管快捷键；编辑句段时，系统文本撤销优先。

## macOS 触控板

macOS 首次运行默认开启触控板优化。双指滚动或触摸开始时会取消尚未结束的查找跳转动画，让用户输入立即取得滚动控制；快速查找动画最长 420 ms。排序拖拽从卡片手柄开始，减少轻触和双指滚动触发误拖的概率。列表使用纵向惯性滚动、边界 containment 和虚拟化，不使用常驻 `requestAnimationFrame` 或常驻 `will-change`。

设置页可关闭触控板优化；关闭后查找动画上限恢复到 650 ms，其他可访问性行为保持不变。系统启用“减少动态效果”时，跳转改为立即定位并禁用高亮动画。

## 双平台打包

从仓库根目录运行：

```powershell
# Windows x64：NSIS setup.exe 与 MSI
pnpm build:desktop:windows
```

```bash
# macOS：Intel + Apple Silicon Universal app 与 DMG（必须在 macOS 上运行）
rustup target add aarch64-apple-darwin x86_64-apple-darwin
pnpm build:desktop:macos
```

`.github/workflows/package-desktop.yml` 在 `main` push 和 pull request 上执行 TypeScript 构建、Rust format、Clippy 与测试门禁；手动触发或推送 `v*` 标签时，门禁通过后再分别在 Windows 与 macOS runner 打包。Windows 产物为 x64 NSIS / MSI，macOS 产物为同时包含 Apple Silicon 与 Intel 的 Universal `.app` / `.dmg`，均保存为 workflow artifacts。当前 macOS 测试包使用 ad-hoc 签名，避免 Apple Silicon 将完全未签名的下载包直接判断为损坏；正式分发仍需配置 Developer ID Application 证书并完成 notarization。

Actions 依赖使用官方主版本对应的完整提交 SHA，降低可变标签带来的供应链风险。仓库默认只授予工作流 `contents: read`；构建产物通过 workflow artifact 上传，不创建 Release，也不要求额外写权限。重复的 pull request 构建会取消旧运行，标签与手动打包不会被自动取消。

最低 macOS 版本为 11.0。Windows 安装包必须在 Windows runner 生成，macOS Universal 包必须在 macOS runner 生成。

## 应用图标

品牌母版位于 `assets/brand/jueming-aligner-icon-master-v2.png`，是 1024×1024 的透明 RGBA PNG。它保留用户提供的扁平花朵图形，并移除了原图画布外侧的点阵背景；不要从聊天截图或带点阵背景的原图再次生成发布图标。

Tauri 图标统一由母版生成：

```powershell
pnpm --dir apps/desktop tauri icon ../../assets/brand/jueming-aligner-icon-master-v2.png --output src-tauri/icons
```

该命令负责生成 Windows `.ico`、macOS `.icns`、通用 PNG 及商店尺寸。网页预览使用 `apps/desktop/public/favicon.png`，应用顶栏复用同一母版，避免桌面包、浏览器预览和界面品牌图形漂移。

## Apple Silicon 性能检查结论

前端长列表由 TanStack Virtual 只挂载可视行；拖拽注册随虚拟行挂载和卸载，不随滚动创建全量监听器。审阅内查找使用 120 ms 输入防抖和预归一化索引，避免每个按键对中英文全文重复做小写转换。跳转动画是有上限、可取消的短时 rAF，用户滚动会立即终止。缓存递归清理放入 Rust blocking worker，不占用 Tauri UI 线程。

这些路径不存在空闲轮询、无限动画、常驻 GPU 合成提示或无上限后台任务。仍需在真实 Apple Silicon 设备上用 Activity Monitor / Instruments 对 10 万句段压力工程执行 15 分钟滚动、连续查找、排序和缓存清理基线，才可形成硬件温升结论；CI 只能确认原生 ARM 构建与功能回归，不能替代温度传感器测试。
