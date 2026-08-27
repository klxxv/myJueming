# ADR-011: ParallelWorkspace 共享四种模式

_Status: Accepted · Date: 2026-08-27 · Scope: Vue domain interaction_

---

## 📋 Context

效果图分别展示 Review、Edit、Order、History，但它们都围绕同一双栏语料、selection、滚动位置和对齐关系。为每个模式创建独立页面会造成状态漂移和重复加载。

## 🎯 Decision

由一个 `ParallelWorkspace` 宿主共享 Slice/Segment/Alignment ViewModel、active Alignment anchor、selection、Context Lens 和控制器。Review、Edit、Order、History 只改变呈现、焦点和允许的 Command；Edit 离开前必须处理未提交草稿，History 由 RevisionId 驱动。

## ⚡ Consequences

模式切换更稳定，双向定位和视觉一致性可集中测试。代价是 SegmentView 和 controller 需要明确 mode 状态机，不能把每个效果图当成互不相关的页面。

## 🔗 References

- [Phase 0 合同：跨层边界](../architecture/mvp-phase0-contracts-v0.1.md)
- [实施计划：ParallelWorkspace](../../jueming-aligner-mvp-implementation-plan-v0.2.md)
