import type { Ref } from "vue";
import type { AlignmentDto, AlignmentGapEdge, KernelClient, ProjectSnapshot, SegmentDto } from "../domain/kernel-client";

interface StructureMutationOptions {
  kernelClient: KernelClient;
  sourceRows: Ref<SegmentDto[]>;
  alignmentRows: Ref<AlignmentDto[]>;
  selectedAlignmentId: Ref<string>;
  busy: Ref<boolean>;
  requireOpenProject: () => boolean;
  applySnapshot: (snapshot: ProjectSnapshot) => Promise<void>;
  clearWorkspaceSelection: () => void;
  notify: (message: string) => void;
  errorMessage: (error: unknown) => string;
}

export const useStructureMutations = ({
  kernelClient,
  sourceRows,
  alignmentRows,
  selectedAlignmentId,
  busy,
  requireOpenProject,
  applySnapshot,
  clearWorkspaceSelection,
  notify,
  errorMessage,
}: StructureMutationOptions) => {
  const insertAlignmentGap = async (segmentId: string, edge: AlignmentGapEdge) => {
    if (!requireOpenProject() || busy.value) return;
    const sideLabel = sourceRows.value.some((segment) => segment.id === segmentId) ? "中文" : "英文";
    busy.value = true;
    try {
      await applySnapshot(await kernelClient.insertAlignmentGap(segmentId, edge));
      selectedAlignmentId.value = alignmentRows.value.find((alignment) =>
        alignment.sourceIds.includes(segmentId) || alignment.targetIds.includes(segmentId))?.id ?? selectedAlignmentId.value;
      notify(`已在${sideLabel}句段${edge === "before" ? "上方" : "下方"}插入空位，并自动重建后续 1:1 对齐`);
    } catch (error) {
      notify(`插入空位失败：${errorMessage(error)}`);
    } finally {
      busy.value = false;
    }
  };

  const mergeSegmentContent = async (segmentIds: string[], mergedContent: string) => {
    if (!requireOpenProject()) return;
    busy.value = true;
    try {
      await applySnapshot(await kernelClient.mergeSegments(segmentIds, mergedContent));
      clearWorkspaceSelection();
      notify(`已合并 ${segmentIds.length} 个 Segment 内容；首项 ID 与关联锚点已保留`);
    } catch (error) {
      notify(`Merge 内容失败：${errorMessage(error)}`);
    } finally {
      busy.value = false;
    }
  };

  const splitSegmentContent = async (segmentId: string, parts: string[]) => {
    if (!requireOpenProject()) return;
    busy.value = true;
    try {
      await applySnapshot(await kernelClient.splitSegment(segmentId, parts));
      clearWorkspaceSelection();
      notify(`已无损拆分为 ${parts.length} 个 Segment；原 Alignment 关系保持不变`);
    } catch (error) {
      notify(`Split 内容失败：${errorMessage(error)}`);
    } finally {
      busy.value = false;
    }
  };

  const groupAlignments = async (alignmentIds: string[], unlinkedSegmentIds: string[]) => {
    if (!requireOpenProject()) return;
    const selectedAlignments = alignmentRows.value.filter((alignment) => alignmentIds.includes(alignment.id));
    const sourceSegmentIds = new Set(selectedAlignments.flatMap((alignment) => alignment.sourceIds));
    const targetSegmentIds = new Set(selectedAlignments.flatMap((alignment) => alignment.targetIds));
    const sourceIds = new Set(sourceRows.value.map((segment) => segment.id));
    for (const segmentId of unlinkedSegmentIds) {
      if (sourceIds.has(segmentId)) sourceSegmentIds.add(segmentId); else targetSegmentIds.add(segmentId);
    }
    busy.value = true;
    try {
      await applySnapshot(await kernelClient.groupAlignments(alignmentIds, unlinkedSegmentIds));
      clearWorkspaceSelection();
      selectedAlignmentId.value = alignmentRows.value.find((alignment) =>
        [...sourceSegmentIds].every((id) => alignment.sourceIds.includes(id))
        && [...targetSegmentIds].every((id) => alignment.targetIds.includes(id)))?.id ?? selectedAlignmentId.value;
      const parts = [
        alignmentIds.length ? `${alignmentIds.length} 个 Alignment` : "",
        unlinkedSegmentIds.length ? `${unlinkedSegmentIds.length} 条未对齐句段` : "",
      ].filter(Boolean);
      notify(`已将 ${parts.join("与")} Group 为一个 Alignment`);
    } catch (error) {
      notify(`Group 失败：${errorMessage(error)}`);
    } finally {
      busy.value = false;
    }
  };

  const ungroupAlignment = async (alignmentId: string, sourceGroups: string[][], targetGroups: string[][]) => {
    if (!requireOpenProject()) return;
    busy.value = true;
    try {
      await applySnapshot(await kernelClient.ungroupAlignment(alignmentId, sourceGroups, targetGroups));
      clearWorkspaceSelection();
      notify("已 Ungroup Alignment");
    } catch (error) {
      notify(`Ungroup 失败：${errorMessage(error)}`);
    } finally {
      busy.value = false;
    }
  };

  return { insertAlignmentGap, mergeSegmentContent, splitSegmentContent, groupAlignments, ungroupAlignment };
};
