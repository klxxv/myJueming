# ADR-002: Stable ID 与物理位置分离

_Status: Accepted · Date: 2026-08-27 · Scope: all canonical objects_

---

## 📋 Context

平行文本支持重排、虚拟滚动、Chunk 存储和历史恢复。数组 index、DOM index、数据库 row 或 Chunk offset 都会随这些操作变化，不能承担 Segment、Alignment、批注或书签的逻辑身份。

## 🎯 Decision

Project、Document、Segment、SegmentOrder、Alignment、HumanAnnotation、Revision 和 Command 使用稳定 opaque ID。MVP 采用 UUIDv7 字符串；RevisionId 在 wire 上使用十进制字符串。所有 UI 定位、引用和命令参数都使用 ID，顺序另由 `PositionKey` 表达。

## ⚡ Consequences

重排和虚拟化不会破坏引用，跨进程和未来协作更安全。代价是需要维护 ID → 物理位置索引，且所有命令都必须显式处理 not found 和 stale revision。

## 🔗 References

- [Phase 0 合同：命名约定与不变量](../architecture/mvp-phase0-contracts-v0.1.md)
- [全局架构：Stable ID](../../jueming_global_architecture_handoff_v0.2.md)
