# 界面国际化

## 选型（2026-08-31）

先检索并比较官方文档，再选择 **Vue I18n 11**，保持实施计划既定的 Vue 3 Composition API 路线。

| 库 | 适用能力 | 本项目取舍 |
| --- | --- | --- |
| [Vue I18n](https://vue-i18n.intlify.dev/guide/advanced/composition) | Vue 响应式消息、复数、格式化；[TypeScript 词典约束](https://vue-i18n.intlify.dev/guide/advanced/typescript) | 采用；无需另加框架适配层，契合现有技术栈 |
| [i18next-vue](https://i18next.github.io/i18next-vue/introduction.html) | 将 i18next 接入 Vue，适合复用跨框架词典生态 | 可行，但当前仅有 Vue 客户端，没有复用 i18next 生态的需求 |
| [Tolgee Vue SDK](https://docs.tolgee.io/js-sdk/integrations/vue/overview) | 可视化翻译编辑与翻译平台工作流 | 本地 MVP 无翻译平台需求，不引入额外服务；并非认为它不能离线运行 |

使用 [@intlify/unplugin-vue-i18n](https://vue-i18n.intlify.dev/guide/advanced/optimization) 在 Vite 构建时预编译词典。三种语言全部随包交付，没有运行时下载、云翻译、遥测或轮询。版本由 package.json / pnpm-lock.yaml 固定。

## 用户行为

- 设置 → 外观与缩放 → 界面语言，支持中文（简体）、English、Français；更改立即生效，不要求重启。
- 优先使用设备上的 `jueming-ui-locale` 偏好；未设置时匹配系统首个支持的语言（例如 `fr-CA` → `fr`）；全部不支持则使用中文。存储被禁用时仍可在当前会话切换。
- 导航、菜单、按钮、提示、状态、无障碍标签、日期、数字和已知诊断均跟随界面语言。项目内容语言与界面语言互相独立。
- 不翻译语料、批注正文/标题、书签文字、工程名、文件名、作者元数据；不改变 ID、顺序、Alignment 或 `.jm` 历史。演示工程名是界面示例，可翻译。
- 不重新挂载应用，不清空编辑草稿或选择，不触发 Kernel 写操作。拆分对话框的分隔标记在本次打开期间保持固定，避免切换语言破坏已输入的无损拆分草稿。
- 原生窗口标题与确认框文案跟随 UI；文件选择器的系统按钮由操作系统决定。Windows NSIS 提供三语选择器；MSI 分别生成 `zh-CN`、`en-US`、`fr-FR`，参见 [Tauri Windows 安装器文档](https://v2.tauri.app/distribute/windows-installer/)。安装器语言不改写应用偏好。

## 代码约定

### MSI 编码兼容

WiX 3 的英文/法文数据库使用西欧代码页，不能直接写入原有中文产品名，否则报 `LGHT0311`；不使用其未正式支持的 UTF-8 绕过方式（[WiX 3 代码页说明](https://docs.firegiant.com/wix3/overview/codepage/)）。`src-tauri/wix/main.wxs` 基于锁定的 Tauri CLI 2.11.4 默认模板，保留上游许可和安装行为，只把 MSI 内部目录/注册表/快捷方式标识固定为 ASCII 品牌，并把显示字符串移入三份 `.wxl`。应用 `productName`、bundle identifier、NSIS 配置和二进制名称不改变。

MSI UpgradeCode 显式固定为原有自动推导值 `2e90697d-b29d-58f5-b283-b08403419c63`，避免多语言被识别为不同产品。新 MSI 默认安装目录/路径注册表键为 `Jueming Aligner`；旧中文路径不再通过默认注册表搜索继承，升级安装时应确认目标目录。跨安装格式迁移与旧版本升级仍需在发布前做实际安装验收。更新 Tauri 时必须同步复核此模板。

### 前端

- 入口：`apps/desktop/src/i18n/index.ts`；词典：`locales/zh.json`、`en.json`、`fr.json`。
- `t()` 的 key 由中文词典静态推导；三份词典必须拥有相同 key 和插值参数。使用有含义的 key，不把运行时用户文本作为 key。
- 在模板、computed 或显示回调内调用 `t()`。状态提示传 `() => t(...)`，不能提前翻译并缓存语言相关的字符串。普通原始诊断仍可传字符串。
- 使用完整句子与命名插值，避免按某一语言语序拼接；数量句使用复数分支和原始数值 `count`，英文 0 为复数，法文 0/1 为单数。
- UI 数字用 `formatNumber()`，时间用 `formatDate()`。不要格式化 opaque ID、RevisionId、语言代码或导出协议字段。
- 已知 Rust 字符串诊断与自动生成的历史摘要仅在 `kernel-messages.ts` 展示时映射，保留未知诊断。控制流仍检查原始错误，不检查翻译后的内容。没有修改 IPC DTO 或 Rust 存储合同。
- 不使用 `v-html` 渲染翻译；技术示例中的 `<seg>` 等仅作为 Vue 转义文本显示。不要把翻译文本作为富 HTML。
- 法语长文案可使工具栏换行；不要恢复固定单行或令 grid 隐式列按最小内容宽度撑破桌面最小窗口。

## 验证

```powershell
corepack pnpm test
corepack pnpm typecheck
corepack pnpm build
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
corepack pnpm --dir apps/desktop tauri build --debug --no-bundle
```

Vitest 检查词典 key/参数对齐、非空与编译、英文/法文无遗漏汉字、模板可见文本与无障碍属性、系统语言匹配/持久化、日期数字/复数、运行时切换、草稿守卫、原始错误、历史原文、批注、搜索 DTO 和无损拆分。新 UI 文案必须同时补三份词典。

手工复核：在三语下切换各导航与模式、打开导入/导出/批注、刷新恢复语言偏好，并检查最小 1180×760 窗口及护眼主题。浏览器预览不能代替真实 `.jm` 的原生读写或 macOS 安装验收。
