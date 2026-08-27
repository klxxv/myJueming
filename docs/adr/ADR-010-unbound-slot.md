# ADR-010: UNBOUND Slot 是合法运行时状态

_Status: Accepted · Date: 2026-08-27 · Scope: Slot registry_

---

## 📋 Context

POS、自动对齐、OCR、KWIC 等能力在总体架构中有正式扩展位置，但人工对齐 MVP 不实现这些 Provider。删除 Slot 会让未来接入改变协议；显示不可用假按钮又会误导用户。

## 🎯 Decision

Slot Registry 从 MVP 开始注册稳定 Slot。没有 Provider 时状态为 `UNBOUND`，UI 以能力状态显示缺失原因；只有 BOUND/可用状态才能提交对应操作。MVP 不加载外部插件运行时。

## ⚡ Consequences

架构扩展不需要改 Core schema，能力可通过事件动态出现。代价是 UI、测试和文档必须区分 BOUND、UNBOUND、DISABLED 和 schema 不兼容，不能仅靠按钮颜色表达。

## 🔗 References

- [Phase 0 合同：Slot 与事件](../architecture/mvp-phase0-contracts-v0.1.md)
- [实施计划：Slot Profile](../../jueming-aligner-mvp-implementation-plan-v0.2.md)
