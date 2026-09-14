# 译法研究功能与设置界面更新计划 v0.1

> 执行状态（2026-09-13）：已据此实现本地首版。本文保留设计目标；现有接口、实施取舍及发布限制见[实施交接](../architecture/local-research-runtime-implementation-v0.1.md)。

- 日期：2026-09-13。
- 状态：规划稿；由用户要求增加的 `settings_feature_plan` 子代理只读审计，主代理汇总并与公共接口对齐。未修改代码、启动页面或安装资源。
- 关联：[总实施计划](../architecture/local-slot-dataflow-plugin-implementation-plan-v0.1.md)、[功能启用与统一接口](../architecture/local-feature-activation-agent-mcp-contracts-v0.1.md)、[研究工作区交互](local-research-feature-interaction-plan-v0.1.md)、[现有设置设计](jueming-settings-system-design-v0.2.md)。
- 范围：本地功能开关、自动资源准备、默认算法、资源管理、设置持久化与能力门禁。云端、headless、商业 license、账号登录继续暂缓。

## 1. 设置入口与用户体验

沿用现有设置外壳及“应用偏好／数据与安全／系统”分组，建议在“数据与安全”增加“译法研究”，与“AI 与 Agent”并列。算法运行不依赖 Agent 聊天功能或模型账号。

普通页只呈现一个功能开关。用户开启即由 Host 自动准备该官方功能的两个模糊 Provider 与 XLM-R 资源；完整安装包已有资源时直接加载，不要求先进入插件管理。

```text
设置 → 数据与安全 → 译法研究

在本机查找相似表达、定位译文并整理译法。
首次开启会自动准备所需资源，工程内容在本机处理。

译法研究                                      [开关]
正在准备所需资源…                             [取消开启]
真实进度

就绪后：
[打开译法研究]
新任务默认设置                                   >
资源与运行信息                                   >
```

准备过程不弹出逐插件、逐模型确认，不要求选择仓库或运行 pip。下载／校验／启动的状态仍可见；“无感”指无需理解安装机制，并非隐藏耗时或失败。

### 1.1 新任务默认设置

- 相似匹配算法：两个真实已注册 Provider；本计划建议编辑距离与字符 n-gram，名称以 P0 冻结结果为准。
- 自动定位译文：默认开启，使用本机 XLM-R；它控制新任务是否执行定位，不是第二个插件安装开关。
- 默认值仅影响新建研究方法／任务。已有 Plan、运行、研究记录继续使用固定的 Provider、参数和模型版本。
- Provider 未就绪或不兼容时给出原因；设置值不能伪造能力就绪。

### 1.2 资源与运行信息

普通层显示可用状态、来源“随应用提供／已下载”、功能占用空间和运行状态；“管理资源”进入存储空间。

展开算法与接口详情后，才显示 Provider／Slot／Schema、资源版本和诊断信息。旧“插件与集成”分类首版可以继续隐藏；可独立安装是实现架构，不要求普通用户看到新的插件市场。

高级本地包导入属于资源管理，可复用同一验证和安装流程；它不成为功能开启的前置步骤。

## 2. 设置代码审计

| 当前位置 | 事实 | 需要实施的变更 |
| --- | --- | --- |
| [capabilities.ts](../../apps/desktop/src/settings/capabilities.ts) | 前端静态表；plugins 恒 UNBOUND | 由 Host 能力快照提供动态状态，保留页面能力与算法就绪的区别 |
| [SettingsWorkspace.vue](../../apps/desktop/src/components/SettingsWorkspace.vue) | 分类和设置搜索数组集中在组件中；仅 available 分类可见 | 增加 research 分类与搜索项；以管理入口是否实现决定显示 |
| 同组件的存储页 | 当前聚焦 `.jm/cache` 清理 | 新增应用级“功能资源”区域，独立于工程缓存 |
| 同组件的隐私页 | 本地保存及外部 AI 说明为主，未提供严格禁网执行 | 明确本地数据处理、资源取得和显式禁网策略 |
| [settings/schema.ts](../../apps/desktop/src/settings/schema.ts) | AppSettings v2，privacy.localOnlyMode 默认 true | v3 明确迁移；不将旧值直接解释为禁止资源下载 |
| [stores/app-settings.ts](../../apps/desktop/src/stores/app-settings.ts) | 静态 capability；防抖后全量保存设置；重置删除 JSON 后重建 | 普通偏好保留；功能控制使用立即的 typed Host 命令及对账 |
| [commands/settings.rs](../../apps/desktop/src-tauri/src/commands/settings.rs) | save_app_settings 直接保存 serde_json::Value | 禁止此通道覆盖 Host 资源事实、包路径和功能控制策略 |
| [kernel-client.ts](../../apps/desktop/src/domain/kernel-client.ts) | 已有 typed 设置 façade | 组合功能、偏好与资源管理的 typed 接口 |
| [App.vue](../../apps/desktop/src/App.vue) | 集成 SettingsWorkspace，已有 Agent 设置插槽 | 接入研究设置组件和应用级能力状态，不让设置组件启动进程 |
| [mcp/lib.rs](../../crates/jueming-mcp/src/lib.rs) | application_settings_metadata 当前仅转发 app.describe | 返回实际且脱敏的功能可用性，不暴露完整设置文件 |
| [agent-transport/server.rs](../../crates/jueming-agent-transport/src/server.rs) | is_native_only／requires_binding 各自列方法 | 接入共享 MethodDescriptor，设备功能接口不能被误判需要工程 |

当前没有单独的 SettingsPanels／设置 catalog 文件可直接复用；建议实施时拆出研究设置组件和类型化条目定义，避免持续扩大 SettingsWorkspace 分支。

## 3. 入口可见性与状态

**管理入口已经实现**与**算法资源已经就绪**是两件事。若把“尚未下载”映射成整个分类 UNBOUND，现有设置过滤会把启用入口隐藏，用户无法开通功能。

建议由 CapabilitySnapshot 分别投影 management_available、FeatureSnapshot 和 Slot readiness。只有真正没有实现功能管理时才隐藏入口；在支持的发行版本中，即使资源缺失，用户也能看见功能开关与准备说明。

| 情况 | 开关／状态文案 | 操作 |
| --- | --- | --- |
| 未开启且资源缺失 | 关闭；首次开启将自动准备所需资源 | 开启 |
| 未开启但资源完整 | 关闭；资源已准备，可直接启用 | 开启，跳过下载 |
| 正在提交启用请求 | 短暂忙碌，等待 Host 接受 | 避免重复提交；不能先显示就绪 |
| preparing | 开关反映 Host 接受的启用意图；正在准备译法研究 | 取消开启 |
| downloading | 已下载／总量；未知总量使用不定进度 | 取消开启 |
| verifying／installing／starting | 正在验证资源／准备运行环境／启动译文定位 | 取消开启，等待安全终止点 |
| ready、Worker 已释放 | 已就绪，使用时自动加载 | 打开功能、关闭 |
| failed | 未就绪，显示实际失败原因 | 重试、取消开启 |
| 下载被明确策略阻止 | 尚需下载资源，当前不允许下载 | 跳到资源取得策略、取消开启 |
| Worker 异常 | 运行组件启动失败 | 重试现有资源；不先重新下载 |
| disabled | 已关闭，资源与研究记录已保留 | 重新开启 |

界面“取消开启”对应 `features.cancel_prepare`：取消当前 activation generation，并撤销这一代启用意图。它不等于取消某次研究 Run；研究取消使用 Pipeline 接口。

当取消与准备完成相遇，以 Host 对代次和期望状态的处理为准。旧完成事件不能把关闭的开关拨回开启。

准备完成仅在用户仍停留于原开启流程、上下文有效且草稿守卫允许时自动进入新工作区。离开设置或进入编辑后只提示已就绪，不抢焦点。

## 4. 资源管理与网络行为

### 4.1 资源取得

Host 先查询完整本地 manifest；插件代码、平台运行时、模型权重、tokenizer 与必要配置齐备时直接加载，不先检查远程更新。Worker 自身不运行 pip、不联网取得缺失模型；依赖准备全部由 Host 管理。

缺资源时使用已声明的功能清单自动下载。准备前核算下载临时文件、解压与最终资源的峰值空间，不在空间不足时先删除仍可工作的旧 Release。

网络中断后按有界策略暂停／重试；实际状态持久化，应用重启对账恢复。Vue 不承担无限重试或轮询。后台准备表示应用仍运行时可进行，本版不承诺完全退出应用后继续下载。

### 4.2 关闭、缓存和删除资源

关闭功能停止接收新计算，处理不再需要的准备与运行；保留完整资源、已保存方法和研究结果。下次开启直接复用。

“删除功能资源”是单独空间管理动作：先显示可释放大小、引用它的功能和活动任务，再经 typed 删除命令执行。实际回收需重新校验资源 generation／引用，不能只信任旧预览。

共享模型、运行时与活跃 Run 按引用保留；随安装包内置的只读资源不计入“可移除下载资源”，不能展示无法实现的释放空间承诺。

下载临时数据、派生 cache、已安装算法资源和正式工程分别统计。清理 `.jm/cache` 不能顺带卸载模型，也不能删除 `revisions/`、人工研究 sidecar 或已保存 Pipeline 方法。

## 5. 隐私语义与设置迁移

当前 `privacy.localOnlyMode` 默认 true，但界面展示的是本地优先数据模式；当前检索未见它在 Host 中作为严格禁网开关执行。不能直接把这个旧默认绑定到“禁止全部资源下载”，也不能静默改成 false。

建议新增明确合同：

| 项目 | 本版语义 |
| --- | --- |
| 工程数据处理 | 两类算法均在本机执行；启用功能不改变正文位置 |
| featureResourceDownloads | `on_feature_enable`／`never`；默认只有用户主动启用才准备缺失资源 |
| 明确禁止联网的设备策略 | 若用户或管理策略明确设置，覆盖资源下载；完整本地资源仍可加载 |
| 外部 AI／Agent 连接 | 保持现有独立配置和授权，开启译法研究不改变它 |

AppSettings v3 迁移保留旧字段来源并记录迁移版本，明确它原来表示的数据模式。新资源策略不得因应用升级、打开工程或 Agent 查询就自动触发下载。严格禁网必须有明确来源，不能由组件自行推断。

新策略由 Rust Host 执行。若未来引入覆盖所有外部连接的全局禁网，它还需接入现有 Agent 网络配置；本版不能仅阻止模型资源下载就宣称已经禁止所有网络活动。

## 6. 持久化权威与 typed 接口

```text
AppSettings v3
  主题、导航、研究设置详情位置等普通界面偏好

Host FeaturePreferences
  desired_enabled、generation
  新任务默认 Provider／自动定位偏好
  功能资源取得策略

Host ResourceRegistry / PrepareJournal
  Release、完整性、引用、准备任务、恢复点

CapabilitySnapshot / FeatureSnapshot
  供 UI、Agent、MCP 查询的真实状态投影
```

不把安装目录、下载进度、校验结果、PID 或 Slot 已就绪状态写回 AppSettings。现有 save_app_settings 全量 JSON 通道不能覆盖 Host 的功能控制字段；旧 UI 保存主题也不能意外关闭功能或改变网络策略。

建议接口如下，名称与公共方法目录统一：

| 接口 | 作用 | 权限／范围 |
| --- | --- | --- |
| capabilities.get、features.list/get | 查询能力与功能状态 | 设备级；Agent／MCP 读取脱敏投影 |
| features.enable/disable | 修改启用意图并调度生命周期 | 原生用户；request ID 去重、generation 校验 |
| features.cancel_prepare/retry_prepare | 取消开启／重试准备 | 原生用户；明确 activation 身份 |
| features.get_preferences/update_preferences | 新任务默认方法、自动定位、资源取得策略 | 原生设置；typed 局部更新，不能改写安装事实 |
| packages.list | 本地资源、来源、引用和用量 | 原生资源管理；外部仅获能力所需的脱敏摘要 |
| packages.preview_remove/remove | 计算影响，再移除可删除资源 | 原生用户；校验引用／运行和预览基线 |
| features.reset_preferences | 恢复功能控制偏好并处理相关任务 | 原生协调动作；保留资源与研究成果 |

无需工程即可调用设备设置接口；分析执行需要真实工程 binding。功能命令立即提交并返回任务状态，不走主题设置的防抖全量保存。

“恢复本机默认设置”协调普通 AppSettings 重置与 Host FeaturePreferences 重置，处理草稿和活动任务，再报告真实结果。若某一步失败，报告部分完成并允许恢复，不把它说成全局原子成功。该操作关闭相应启用意图并恢复新任务默认值，保留已安装资源、工程及研究事实。

## 7. Pipeline、Agent 与 MCP 的一致性

设置、研究视图、Pipeline、Agent 和 MCP 消费同一 Host 能力状态；客户端缓存的旧工具 Schema 不能绕过执行时检查。

设备作用域、原生专属权限、资源影响与功能前置条件进入 MethodDescriptor。当前 Host 的 scoped_method、HTTP 的 requires_binding 和各处 native-only 表都需要对齐，不能只修改 settings 组件。

MCP 的设置 metadata 返回当前功能／Provider 状态和缺失原因，不能暴露完整设置 JSON、本机路径或下载凭据。Agent 请求未启用功能时可以引导用户到原生开关，不能通过 Pipeline execute、低层 Worker 或绑定操作隐式安装。

用户按开关之后，宿主在这一固定准备范围内继续执行，无需逐包再次批准。模型已经完整、功能已启用但 Worker 被空闲释放时，按需重启不要求用户重新开启。

## 8. 实施位置与验收

建议拆出研究设置面板和资源详情组件；SettingsWorkspace 继续负责分类、搜索和导航。新增应用级 feature-runtime store 由 Host 快照／事件驱动，详细文件职责与研究工作区计划一致。

P0 冻结设置迁移与资源取得策略；P1 接入 capability 和方法元信息；P5 完成开关及自动准备；P8 验证设置、研究工作区、Agent、MCP 与资源管理的完整链路。

必须验收：

1. 未下载时仍能找到“译法研究”启用入口；未实现的云端分类仍不显示假操作。
2. 干净联网环境点一次开关自动准备；完整安装包在断网环境直接加载，无强制联网前置。
3. 重复开启、中途取消、准备完成与取消竞态、断网重试、重启恢复、空间不足、Worker 崩溃均显示真实状态。
4. 切换两个模糊算法默认值只影响新任务；自动定位偏好不触发模型卸载或改写已有方法。
5. 旧 v2 设置迁移保留本地数据模式，不误变为全体禁下载，也不自动下载；明确禁止资源下载时 Host 确实阻止获取。
6. 全量保存普通设置不能覆盖 Host 控制字段；重置默认值与任务状态一致，并保留资源及研究成果。
7. 清理工程 cache、临时下载、移除资源各自只作用于正确对象；共享和随包只读资源占用说明准确。
8. 从详情返回恢复入口焦点；开关有标签和描述；进度可读，阶段播报节制。键盘、缩放、护眼主题、焦点强化与 MotionPolicy 均遵守现有设置。
9. 设置搜索只索引功能与选项，不搜索正文、凭据或诊断原始内容。
10. 设置关闭后 MCP 的旧工具调用被 Host 拒绝，历史研究结果依然可以阅读和导出。

本轮为计划审计，没有运行上述实现验收。
