# ADR-005: Alignment 只引用 Segment

_Status: Accepted · Date: 2026-08-27 · Scope: manual alignment_

---

## 📋 Context

翻译边界不总是一致，MVP 必须支持 1:1、1:n、n:1、n:m 和未对齐。把对齐绑定到行号或 Token 会让编辑、重排及未来字幕/OCR 失去一致语义。

## 🎯 Decision

Alignment 保存有序的 `source_segment_ids[]` 和 `target_segment_ids[]`，两侧均非空；cardinality 由数量派生。一个 Segment 默认最多属于一个 active Alignment。`Group` / `Ungroup` 产生新 AlignmentId，Unlink 仅使关系离开 active 集合；Segment 的 `Merge 内容` / `Split 内容` 属于另一条内容结构操作线，详见 ADR-014。

## ⚡ Consequences

复杂关系可完整保存、导出和回到平行视图，Kernel 可集中校验重复占用。代价是 UI 必须支持多选和复杂组呈现，不能用简单行号连接代替关系模型。

## 🔗 References

- [Phase 0 合同：Alignment](../architecture/mvp-phase0-contracts-v0.1.md)
- [MVP 功能文档：Alignment Model](../../jueming-aligner-mvp-functional-spec-v0.1.md)
