# 暗黑主题与 Tailwind 验证

入口：设置 → 外观与阅读 → 显示主题 → 暗黑。可选择“跟随系统”自动响应系统明暗变化；明亮与护眼仍可随时切回，主题只保存于设备设置。

## 实现边界

采用 Tailwind 3.4/PostCSS，禁用 Preflight；全局及组件共享颜色通过 Tailwind 语义 utilities/`@apply` 使用统一主题变量。设置页主要容器采用模板 utilities；对齐布局坐标、阅读尺寸、状态选择器和动画保留 CSS。未变更 Kernel、IPC、工程数据或最低 macOS 版本。

## 可复现检查

```sh
corepack pnpm typecheck
corepack pnpm build
node tests/themes/browser-regression.mjs
THEME_BROWSER=webkit node tests/themes/browser-regression.mjs
node tests/themes/native-fallback-regression.mjs
node tests/alignment-layout/run-contracts.mjs
```

浏览器脚本需要可用的 Playwright 与浏览器；可通过 `PLAYWRIGHT_MODULE` 指定模块路径，`THEME_SCREENSHOT_DIR` 指定截图目录。默认读取 `http://127.0.0.1:1420`，截图写系统临时目录。

脚本通过真实设置入口检查三主题、跟随系统的初始解析及实时明暗切换、自动偏好刷新持久化、手动主题不受系统变化影响、Tailwind 容器布局、暗色增强对比度及系统对比度偏好、审阅/编辑/排序模式、正文选中背景对比度，并截图批注侧栏、搜索、历史和新建工程弹窗。使用浏览器只读演示数据，不创建或编辑真实工程。空历史/批注页面不代表已验证有数据的所有 diff 和批注状态；没有覆盖原生 macOS 11 设备。

## 本次结果

- TypeScript 检查、生产构建、Chromium 与 WebKit 主题回归通过。
- 已逐张视觉检查明亮/护眼/暗黑设置、暗黑审阅/编辑/排序、增强对比度、搜索、历史空态与新建工程弹窗；修复工具栏白底以及浅绿按钮上白字对比度问题。
- 合并整理时将既有 `kernelUiMessages` 接入运行时，修复 `ja.errorNoUndo` 等仍使用英文占位的问题；`pnpm test:i18n` 的源码审计、五语言 1744 个键和 50 个设置页面渲染检查全部通过。
- 构建仍提示主 JS chunk 大于 500 kB；本次不涉及分包。

## 跟随系统切换组合回归

- `tests/themes/browser-regression.mjs` 追加暗黑 ↔ 护眼、暗黑 ↔ 明亮的即时切换及刷新持久化验证。
- `tests/themes/native-fallback-regression.mjs` 在实际浏览器加载设置 store，注入原生主题通知，验证旧 dark 原生缓存不会阻断新 light 媒体事件、原生事件仍生效、手动覆盖及释放监听后忽略回调。
- 原生通知通过测试替身注入；这不代替 macOS/Windows 实机系统外观切换验收。

## 阅读进度与布局回归

- 双条进度指示分别显示当前原文／译文的已对齐 Segment 数量；拖动期间预览游标，释放后跳转阅读位置，Home／End 支持键盘跳转。Chromium 与 WebKit 均覆盖两侧跳转、拖动释放和滚动反馈。
- 布局合同使用 10,000 个对齐块，验证可视区裁剪、同一动画帧内两列测量的单次发布、亚像素噪声过滤、重置及卸载取消。
- 测量合同已接入 `pnpm test:agent-ui`；旧工作区索引测试补充可控动画帧并在测量实际发布后验证可视区边界。
