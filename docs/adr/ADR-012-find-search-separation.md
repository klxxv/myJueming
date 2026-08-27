# ADR-012: View Find 与 Kernel Project Search 分离

_Status: Accepted · Date: 2026-08-27 · Scope: search interaction_

---

## 📋 Context

正文采用虚拟列表，未显示的 Segment 不在 DOM；CodeMirror 的查找适合当前编辑器/当前视图，却不能可靠地遍历全工程。把两者混为一条快捷键会导致性能和语义错误。

## 🎯 Decision

`Ctrl+F` 只调用 CodeMirror `@codemirror/search` 或当前 View 的查找，`Ctrl+Shift+F` 调用 Kernel `search_segments` Project Search。项目级普通文本和基础正则在 Rust 执行，结果带 SegmentId、语言、上下文、Alignment 状态和 base revision；替换必须重新校验版本并原子提交。

## ⚡ Consequences

当前编辑体验快速且符合编辑器预期，全工程搜索可分页、可索引和可回到平行视图。代价是要维护两套入口、结果投影和快捷键冲突测试，不能用 DOM 或 Pinia 全文数组冒充项目搜索。

## 🔗 References

- [Phase 0 合同：Query 与不变量](../architecture/mvp-phase0-contracts-v0.1.md)
- [实施计划：Search/Replace](../../jueming-aligner-mvp-implementation-plan-v0.2.md)
