# ADR-007: Append-only Revision 与恢复语义

_Status: Accepted · Date: 2026-08-27 · Scope: history and autosave_

---

## 📋 Context

MVP 要求 Undo/Redo、自动保存、版本比较、崩溃恢复和恢复旧版本。直接覆盖当前数据库会破坏审计、未来版本和恢复边界。

## 🎯 Decision

每个成功 canonical ChangeSet 追加一个 complete Revision，并关联 parent、OperationId 和摘要。Undo、Redo、RestoreRevision 都提交新的 Revision；只有完整事务可见，自动保存合并连续输入为语义操作。

## ⚡ Consequences

历史可追溯，崩溃恢复只需定位最后完整 Revision，旧版本永不被覆盖。代价是需要日志保留、压缩/快照策略和版本差异投影，存储会随操作增长。

## 🔗 References

- [Phase 0 合同：Revision 与写入顺序](../architecture/mvp-phase0-contracts-v0.1.md)
- [实施计划：Autosave/History](../../jueming-aligner-mvp-implementation-plan-v0.2.md)
