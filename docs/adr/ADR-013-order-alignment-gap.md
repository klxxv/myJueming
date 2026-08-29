# ADR-013: Order Mode 空位是原子关系重建命令

_Status: Accepted · Date: 2026-08-29 · Scope: manual alignment_

---

## 📋 Context

人工顺序校对时，某一语言侧可能缺少与另一侧对应的句段。用户需要在所选 Segment 上方或下方插入一个视觉空位，让另一侧的一条 Segment 保持未对齐，并按当前两侧 `SegmentOrder` 继续建立后续 1:1 关系。

空位不能实现为正文为空的假 Segment，也不能创建单侧 Alignment；两者都会破坏 Segment 与 Alignment 的冻结不变量。普通 Move 仍必须只改变顺序而不改变关系，因此这一行为也不能隐式塞进重排命令。

## 🎯 Decision

新增显式 `InsertAlignmentGap(segment_id, edge)` Command，只从 Order Mode 的真实已对齐 Segment 发起：

- `edge` 为 `before` 或 `after`，空位位于所选 Segment 的同一语言侧；
- Kernel 以所选 Segment 当前 Alignment 的另一侧边界为锚点，使该边界处的另一侧 Segment 变为未对齐；
- 从边界开始移除受影响的 active Alignment，并按两侧当前 `SegmentOrder` 创建新的 1:1 manual Alignment；任一侧尾部多余 Segment 保持未对齐；
- 整个关系重建是一次原子 ChangeSet 和一个完整 Revision；失败不写盘；
- Segment、SegmentOrder、书签和批注身份不变；不持久化空字符串 Segment、空 Alignment 或 UI 行号。

这里的“后续自动对齐”是用户显式触发的确定性顺序重建，不绑定或冒充延期的语义自动对齐 Provider。

## ⚡ Consequences

界面可以显示真实的单侧空位，同时 canonical 模型仍只保存 Segment 与双侧 Alignment。后续关系会获得新的 AlignmentId，旧关系只保留在 Revision 历史中。复杂 Alignment 跨越重建边界时会整体失活，边界外未重新配对的成员保持未对齐。

## 🔗 References

- [Phase 0 合同：Alignment](../architecture/mvp-phase0-contracts-v0.1.md)
- [ADR-004：SegmentOrder](ADR-004-segment-order.md)
- [ADR-005：Alignment Segment refs](ADR-005-alignment-segment-refs.md)

