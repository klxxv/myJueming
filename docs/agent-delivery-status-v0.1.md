# Agent / MCP / Pipeline 交付进度

日期：2026-09-06。分支：`codex/agent-mcp-pipeline`。本轮已从规划转为执行，主线程负责界面、接口整合、验收及提交，六个 `gpt-5.6-terra` / high 子代理分别承担 host、transport、context、Pipeline、runtime/platform 和独立 QA。

## 已接入的工作链路

| 阶段 | 内容与框架 | 解耦接口 | 验收重点 |
| --- | --- | --- | --- |
| M1 应用本体 | Tauri 2 / Rust LocalAppHost，共享 Kernel、Revision、SearchSpec、上下文与有序事件 | typed agent facade；`AgentCall/Reply/AppEvent`；native approval | 外部工具与原生界面共享同一工程；跨绑定 ACK、过期版本、取消和幂等 |
| M1 外部连接 | `rmcp` stdio、Axum loopback、AG-UI CUSTOM SSE | 可撤销会话连接；权限允许列表；独立 MCP binary | 真实 MCP 子进程及 SSE，禁止外部批准；不泄露工程全量投影或密钥 |
| M2 全局界面 | Vue 3 全局助手/批注、项目隔离草稿、tab/focus/稳定选择 ID | `useAgentWorkspace`、context snapshot、全局 panel slots | 搜索/工程/设置/历史/Pipeline 保持侧栏，发送时固定上下文 |
| M3 Pipeline | Rust 方法历史与 Jieba，Vue Flow 图、参数与派生结果 | typed PipelineClient；方法 revision / artifact schema | 不覆盖原文，真实分词、词典更新、批准、冲突、并发取消与恢复 |
| M4 内置助手 | Rust provider/tool loop、reqwest、SQLite、keyring | AgentRuntimePort / ModelProvider；native runtime facade | 显式配置，本机/HTTPS，无默认远程兜底；持久会话、工具调用、批准等待与取消 |
| M5 小花园 | 原创 SVG 静态猫狗/决明草，有限 Web Animations，DOM 审核与蝴蝶定位 | CompanionController / CompanionRenderer | 默认静态；隐藏和减少动态仍能审核；懒加载、释放资源、不靠动画判断成功 |

设置同时提供外部 MCP 连接、模型 endpoint/model/key、上下文分享、花园静态/安静/生动/隐藏及猫狗/蝴蝶开关。模型密钥不写入工程或普通前端设置。外部连接默认关闭；启用后复制的是当前应用会话连接配置，重启需重新取得连接配置。

## 验收修复

- 外部绑定 A 的跳转由桌面绑定 B 确认；按项目/版本和精确 action ID 验证，终态不能被晚到 ACK 复活。
- 待批准时，停止发送与批准按钮分别控制；隐藏花园仍可在右栏批准。
- 搜索执行携带显式 SearchSpec 原子提交；原生替换预览以 Rust 的实际差异为准，冻结请求与版本。
- 工程切换不能让迟到的消息/方法请求覆盖新工程；聊天输入在请求期间继续编辑时保留新草稿。
- Pipeline 批准恢复使用 host 预留的精确方法版本 ID；人工保存相同参数不能被误认成批准，之后新增版本也不妨碍确认真实的先前提交。
- 内置 runtime 的模型函数名使用兼容的 `jueming_*` 别名；方法修改提案需原生批准后继续，实际 HTTP 协议和恢复链路均有回归测试。
- 全局侧栏 Grid 最小高度、紧凑窗口花园导航和护眼主题已调整。新增中文辅助文字至少 11 px。

## 当前验证边界

本地最终门禁已通过：冻结依赖安装、TypeScript 检查、前端生产构建、10 项生产代码前端合同检查、Rust 格式/全 workspace Clippy/全 workspace tests，以及 Windows Tauri debug 原生编译。独立执行证据见 [QA 报告](testing/agent-integration-qa-v0.1.md)。功能提交后另以 GitHub Actions 验证远程 Windows 与 macOS 构建，尚不把排队或运行中算作通过。

前端浏览器验收使用临时开发端口 4173 和生产预览端口 4174，因为本机 Windows 系统保留了包含 1420 的 TCP 段；仓库的 Vite/Tauri 1420 配置保持原值。生产预览已验证 Pipeline 动态加载和全局助手切换，浏览器无 warning/error。主入口约 373 kB，Pipeline 独立块约 162 kB，动画后端约 2 kB（均为压缩前）。

Windows 浏览器中已检查助手跨页面保留、花园开关、设置入口、护眼主题与输入框裁切修复。真实 `.jm`、MCP stdio、AG-UI 和双绑定 ACK 使用临时合成工程测试，没有提交真实用户语料。

macOS Universal 编译、双架构 MCP 及 `lipo` 检查已加入 CI；Windows 上的测试不能替代 macOS 实机的 Command/IME/触控板、签名、公证和温升检查。

## 后续对接点与未声称完成的能力

- 当前动画 renderer 是 Web Animations。Spine/Pixi renderer、骨骼资源、美术行走路径可以经 `CompanionRenderer` 替换；业务批准和导航不等待动画。
- 当前扩展 registry 是类型化声明与生命周期边界。任意第三方代码加载、沙箱及应用主动连接其他 MCP 服务尚未实现。
- 内置服务需用户在设置中提供实际可用 endpoint/model；模拟 provider 验收不等于已连接用户的真实模型。
- Pipeline 首版为固定算子集合；POS/Lemma/NER、OCR、语义对齐和云协作保持延期。方法节点批注的持久 anchor 扩展尚未加入 canonical HumanAnnotation；当前全局批注仍针对工程 Segment/Alignment。
