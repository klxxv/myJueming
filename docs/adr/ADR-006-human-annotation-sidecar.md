# ADR-006: HumanAnnotation 作为 sidecar layer

_Status: Accepted · Date: 2026-08-27 · Scope: local annotations_

---

## 📋 Context

单机批注是 MVP 功能，但 POS、Lemma、NER 等机器标注也存在于总体架构。若把批注字段塞进 Segment 或复用模糊的 Annotation 类型，会污染内容模型并混淆两类生命周期。

## 🎯 Decision

HumanAnnotation 独立保存 body、状态、作者标签、时间和稳定链接，关联一个或多个 Segment，并可附带 AlignmentId。它通过专用 Command/Query/Event 管理，不改变 Segment 内容或 Alignment。

## ⚡ Consequences

批注可独立筛选、撤销、持久化和扩展，机器 Annotation 仍可由 Slot/Artifact 提供。代价是跳转需要解析 sidecar 链接，删除或迁移 Segment 时必须定义 orphan 策略。

## 🔗 References

- [Phase 0 合同：HumanAnnotation](../architecture/mvp-phase0-contracts-v0.1.md)
- [实施计划：单机批注](../../jueming-aligner-mvp-implementation-plan-v0.2.md)
