# UI 国际化回归

在仓库根目录运行 `corepack pnpm test:i18n`：

- `vue-source-audit.mjs` 解析全部生产 Vue 单文件组件，检查脚本字符串、模板正文、表达式、属性、无障碍标签及 CSS `content` 中的中文。注释、其他样式与正则表达式不作为界面文案。非 UI 内容只能使用文件名与完整字符串匹配的明确例外；失效例外也会导致测试失败。
- `run-contracts.mjs` 校验五个语言目录的键、非空内容及插值参数，并使用真实 Vue/i18n 渲染设置、面板和工作区的多种状态。缺失词条直接失败，不允许通过默认中文回退掩盖遗漏。

启动 `corepack pnpm dev:web` 后运行：

```sh
node tests/i18n/browser-shell.mjs
node tests/import-wizard/browser-regression.mjs
```

浏览器测试需要 Playwright；可用 `PLAYWRIGHT_MODULE` 指定安装位置。它们使用独立浏览器上下文，不修改真实工程。前者检查实际页面导航、设置语言切换、既有状态提示与教程；后者检查五语言新建向导和原有导入生命周期。

项目名称、文件名、用户正文、语言原生名称与未知外部诊断不是待翻译 UI。不要为了消除扫描结果翻译这些数据，也不要把界面文案移到普通 TypeScript 常量里绕过检查；被组件引用的说明和标签同样应通过 i18n 呈现。技术标识（如稳定 ID、方法名、Segment、Alignment、JSON）应保持其语义。
