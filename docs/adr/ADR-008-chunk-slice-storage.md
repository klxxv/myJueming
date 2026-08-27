# ADR-008: Chunk/Slice 存储与交互边界

_Status: Accepted · Date: 2026-08-27 · Scope: local data access_

---

## 📋 Context

虚拟滚动和未来大语料不能依赖把全工程正文放进 DOM、Editor 或 Pinia 数组。与此同时，Chunk 只是物理存储/调度单位，不能成为 Segment 或 Alignment 的语义边界。

## 🎯 Decision

canonical 文本以 Chunk/overlay 持久化，Kernel 通过 Slice 按 Document、Segment/Alignment 锚点、范围、halo、字段和 Revision 提供局部视图。UI、搜索结果和历史列表均使用分页/虚拟化与稳定 ID。

## ⚡ Consequences

首版小数据也保持可扩展访问方式，缓存和索引可独立重建。代价是 Slice loader、测量缓存和失效逻辑比全量数组复杂，测试必须覆盖冷/热读取。

## 🔗 References

- [Phase 0 合同：工程格式与 Query](../architecture/mvp-phase0-contracts-v0.1.md)
- [全局架构：Chunk/Slice](../../jueming_global_architecture_handoff_v0.2.md)
