# 文档导航

应用介绍与开发命令见[项目 README](../README.md)。本目录按架构、决策、设计、平台和验证组织资料；根目录保留既有规格与计划的稳定路径，避免破坏历史引用。

## 阅读顺序

1. [开发约定](../AGENTS.md)：范围、不可破坏的不变量与验证要求。
2. [跨层合同](architecture/mvp-phase0-contracts-v0.1.md)与 [ADR 索引](adr/000-index.md)：实现约束和已接受决策。
3. [MVP 功能规格](../jueming-aligner-mvp-functional-spec-v0.1.md)与[实施计划](../jueming-aligner-mvp-implementation-plan-v0.2.md)：初始范围、阶段与历史验收。
4. [长期架构背景](../jueming_global_architecture_handoff_v0.2.md)：扩展方向，不代表全部已经实现。

## 按任务查阅

| 任务 | 入口 |
| --- | --- |
| 开发与发布排障 | [CI 维护](testing/ci-maintenance.md)、[桌面平台指南](platform/desktop-platform-guide-v0.1.md) |
| 平行工作区设计 | [视觉规范](design/mvp-visual-spec-v0.1.md)、[参考图](../MVP效果图/) |
| 多译本工程 | [ADR-018](adr/ADR-018-multiple-translations-and-panel-boundary.md) |
| 导入编码 | [ADR-019](adr/ADR-019-import-encoding-detection.md) |
| Agent / MCP 接入 | [实施交接](architecture/agent-implementation-handoff-v0.1.md)、[连接使用](architecture/agent-transport-usage-v0.1.md)、[运行时 API](architecture/agent-runtime-api-v0.1.md) |
| Pipeline 与本地研究 | [Pipeline API](architecture/pipeline-api-v0.1.md)、[研究运行时](architecture/local-research-runtime-implementation-v0.1.md)、[算法包](../plugins/README.md) |
| 测试与验收 | [MVP 测试计划](testing/mvp-test-fixtures-and-computer-use-plan-v0.1.md)、[Agent QA](testing/agent-integration-qa-v0.1.md)、[国际化测试](../tests/i18n/README.md) |
| 实施记录 | [DECISIONS](../DECISIONS.md)、[Agent 阶段交付记录](agent-delivery-status-v0.1.md) |

## 文件维护

- `architecture/` 放跨层合同、模块接口与架构方案；领域决策变更放 `adr/`。
- `design/` 放交互与视觉说明；`platform/` 放系统行为与分发约束。
- `testing/` 放测试计划、排障方法与带日期的验证证据。
- 普通实现约定记入根目录 `DECISIONS.md`；历史验收记录保留时间与适用范围。
- 测试代码留在 `tests/` 和各 Rust crate，生成脚本留在 `scripts/`，图片留在 `assets/` 或对应设计目录。
- `target/`、`dist/`、`node_modules/`、`test-output/`、本地 `.jm` 工程和凭据属于本机文件，不提交。
