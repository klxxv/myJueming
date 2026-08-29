# ADR-014: Segment 内容结构与 Alignment 分组解耦

_Status: Accepted · Date: 2026-08-29 · Scope: manual editing, alignment, history and sidecars_

---

## 📋 Context

旧 UI 同时把 `Merge` / `Split` 用于“改文本边界”和“改翻译关系”，使用户无法预期一个操作是否会删除 Segment、改变文本、重排 refs 或生成新的 Alignment。该歧义也会让 Revision、书签、批注与跨 Block 拖动缺少可验证的迁移规则。

`Alignment Block` 是当前 `SegmentOrder` 和 Alignment refs 的视觉投影，不是领域对象。拖动跨越 Block 有时只是阅读/审校顺序调整，不能因此悄悄改写翻译关系。反之，跨多个 Alignment 合并文本一定涉及关系层选择，必须由用户明确同意。

## 🎯 Decision

冻结两条正交操作线：

1. `Merge Segments` / `Split Segment` 是 Segment 内容和结构操作。Merge 只接受同一 Document 的连续 Segment，按当前顺序保留首个 `SegmentId` 并吸收其余项；Split 要求 `parts[]` 是原内容的有序、无损、非空划分，保留原 ID 给第一 part，后续 part 创建新 ID 并紧邻插入 `SegmentOrder`。
2. `Group Alignment` / `Ungroup Alignment` 是 Alignment 关系操作。它们不改 Segment content 或 order；Group、Ungroup 均让旧 active `AlignmentId` 失活并创建新 ID。Unlink 是完全解除关系，不是 Ungroup。
3. `Alignment Block` 不持久化、没有 Block ID。`MoveSegment` 可以跨 Block，也可产生 non-contiguous、interleaved 或 crossed 投影，但只改 `SegmentOrder`，绝不改变 Alignment 成员或 ID。
4. 跨 active Alignment 的内容 Merge 被拒绝，除非用户先执行 Group，或显式确认包含 `Group → Merge Segments` 的原子复合 ChangeSet。Split 结果先继承原 Alignment；若要分别对齐，必须再 Ungroup。
5. 每个内容/关系 Command 在同一事务内维护 sidecar：Segment Merge 将 Bookmark 和 Annotation 的被吸收 Segment links 转到保留首项；Split 使 Annotation links 扩展到所有 parts、Bookmark 仍锚定第一 part。Group/Ungroup/Unlink 仅在可唯一确定时更新其 `alignment_id` 导航 hint，否则清空 hint，保留所有 Segment anchors。
6. 每次成功操作都产生一个完整 append-only Revision。历史和新 UI 使用 `Merge Segments`、`Split Segment`、`Group Alignment`、`Ungroup Alignment` 术语。旧 `merge_alignment` / `split_alignment` 最多是短期 IPC 兼容别名，不能作为新的 canonical history 语义。
7. “跨 Block”不等于“跨关系”：Move/Drag 可跨 Block 且只改顺序；Group/Ungroup 可处理跨行投影但必须提交完整关系或明确 partitions；跨多个 active Alignment 的 Merge 内容直接拒绝。UI 可对非相邻选择显示 crossed/interleaved 风险预览，但不能以视觉 Block 或显示行不相邻作为拒绝 Command 的依据。

## ⚡ Consequences

用户能直接判断操作将改变“文本分段”还是“翻译分组”。内核需要实现内容的无损校验、连续性校验、sidecar 迁移与复合 ChangeSet 回滚；UI 需要将 Segment selection 与 Alignment selection 分离，提供 Group 后继续的预览，并提示跨 Block Move 只改变顺序。

书签的 canonical data 不复制正文；`BookmarkView` 从当前 Revision 的 stable Segment 锚点派生语言、顺序号、关系摘要及截断内容预览。这样 Segment 编辑后预览不会陈旧，也不会造成第二份正文真值。

拖动实现以 `SegmentId` 注册连接端点。Overlay 接管端点坐标、原位保留占位、drop 后虚拟化完成测量才移除 Overlay；连线不能依赖 row index、旧 DOM 矩形或 Alignment Block。自动化 drag E2E/视觉回归暂缓，但 stable-ID 端点和重测量契约仍必须测试。

## 🔗 References

- [Phase 0 合同：Segment / Alignment / Sidecars](../architecture/mvp-phase0-contracts-v0.1.md)
- [ADR-003：Segment canonical unit](ADR-003-segment-canonical-unit.md)
- [ADR-004：SegmentOrder](ADR-004-segment-order.md)
- [ADR-005：Alignment Segment refs](ADR-005-alignment-segment-refs.md)
- [ADR-006：HumanAnnotation sidecar](ADR-006-human-annotation-sidecar.md)
- [ADR-007：Append-only Revision](ADR-007-append-only-revision.md)
- [Pragmatic drag and drop virtualization recipe](https://atlassian.design/components/pragmatic-drag-and-drop/core-package/recipes/virtualization)
- [TanStack Virtual API](https://tanstack.com/virtual/latest/docs/api/virtualizer)
- [Vue Teleport](https://vuejs.org/guide/built-ins/teleport)
