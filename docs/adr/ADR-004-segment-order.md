# ADR-004: SegmentOrder 独立于 Segment

_Status: Accepted · Date: 2026-08-27 · Scope: document structure_

---

## 📋 Context

Order Mode 允许上移、下移和拖拽；顺序变化不能让 SegmentId、AlignmentId、Bookmark 或 Annotation 失效。把顺序字段放在 Segment 中也会混淆内容事实与某一视图的结构。

## 🎯 Decision

每个 Document 拥有独立 SegmentOrder，条目为 `segment_id + position_key`。客户端只提交 `MoveSegment(segment_id, before/after_segment_id)`，PositionKey 由 Kernel 生成和校验。

## ⚡ Consequences

重排只产生结构 ChangeSet，引用关系保持稳定，也为未来多结构视图预留空间。代价是读取正文必须同时加载顺序投影，且需要处理 PositionKey 重平衡。

## 🔗 References

- [Phase 0 合同：SegmentOrder](../architecture/mvp-phase0-contracts-v0.1.md)
- [MVP 实施计划：Order Mode](../../jueming-aligner-mvp-implementation-plan-v0.2.md)
