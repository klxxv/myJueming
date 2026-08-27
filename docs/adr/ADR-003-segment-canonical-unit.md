# ADR-003: Segment 作为 canonical relation unit

_Status: Accepted · Date: 2026-08-27 · Scope: content and parallel relations_

---

## 📋 Context

MVP 默认处理句子，但未来还要容纳 SubtitleCue、OCRRegion、TranscriptUtterance 等不同边界。若为每种内容建立不同 Kernel primitive，会使对齐、编辑、书签和导出无法复用。

## 🎯 Decision

统一使用 Segment 作为可编辑、可引用、可排序、可对齐的 canonical 内容单位；用 `kind` 区分类型，MVP 固定 `sentence`。Token 和段内分析属于 derived layer，不替代 Segment。

## ⚡ Consequences

对齐和 UI 可复用同一套 ID/Query/Command，未来 Parser 不需改 Kernel 核心。代价是格式专属 metadata 必须 schema 化，不能不断扩充核心 Segment struct。

## 🔗 References

- [Phase 0 合同：Segment](../architecture/mvp-phase0-contracts-v0.1.md)
- [MVP 功能文档：Segment](../../jueming-aligner-mvp-functional-spec-v0.1.md)
