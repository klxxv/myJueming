import type { Ref } from "vue";
import type { AlignmentDto, AlignmentGapEdge, KernelClient, WorkspaceProject, SegmentDto } from "../domain/kernel-client";
import { t, type LocalizedMessage } from "../i18n";

interface StructureMutationOptions {
  kernelClient: KernelClient;
  targetDocumentId?: () => string | undefined;
  sourceRows: Ref<SegmentDto[]>;
  alignmentRows: Ref<AlignmentDto[]>;
  selectedAlignmentId: Ref<string>;
  busy: Ref<boolean>;
  requireOpenProject: () => boolean;
  applySnapshot: (snapshot: WorkspaceProject) => Promise<void>;
  clearWorkspaceSelection: () => void;
  notify: (message: LocalizedMessage) => void;
  errorMessage: (error: unknown) => string;
}

export const useStructureMutations = ({
  kernelClient,
  targetDocumentId,
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
    const sourceSide = sourceRows.value.some((segment) => segment.id === segmentId);
    const insertBefore = edge === "before";
    busy.value = true;
    try {
      await applySnapshot(await kernelClient.insertAlignmentGap(segmentId, edge, targetDocumentId?.()));
      selectedAlignmentId.value = alignmentRows.value.find((alignment) =>
        alignment.sourceIds.includes(segmentId) || alignment.targetIds.includes(segmentId))?.id ?? selectedAlignmentId.value;
      notify(() => t("opGapInserted", {
        side: t(sourceSide ? "opSourceSide" : "opTargetSide"),
        position: t(insertBefore ? "opAbove" : "opBelow"),
      }));
    } catch (error) {
      const message = errorMessage(error);
      notify(() => t("opGapInsertFailed", { error: message }));
    } finally {
      busy.value = false;
    }
  };

  const mergeSegmentContent = async (segmentIds: string[], mergedContent: string) => {
    if (!requireOpenProject()) return;
    const segmentCount = segmentIds.length;
    busy.value = true;
    try {
      await applySnapshot(await kernelClient.mergeSegments(segmentIds, mergedContent));
      clearWorkspaceSelection();
      notify(() => t("opMergeSegmentsSuccess", { count: segmentCount }));
    } catch (error) {
      const message = errorMessage(error);
      notify(() => t("opMergeSegmentsFailed", { error: message }));
    } finally {
      busy.value = false;
    }
  };

  const splitSegmentContent = async (segmentId: string, parts: string[]) => {
    if (!requireOpenProject()) return;
    const partCount = parts.length;
    busy.value = true;
    try {
      await applySnapshot(await kernelClient.splitSegment(segmentId, parts));
      clearWorkspaceSelection();
      notify(() => t("opSplitSegmentsSuccess", { count: partCount }));
    } catch (error) {
      const message = errorMessage(error);
      notify(() => t("opSplitSegmentsFailed", { error: message }));
    } finally {
      busy.value = false;
    }
  };

  const groupAlignments = async (alignmentIds: string[], unlinkedSegmentIds: string[]) => {
    if (!requireOpenProject()) return;
    const alignmentCount = alignmentIds.length;
    const unlinkedCount = unlinkedSegmentIds.length;
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
      notify(() => alignmentCount && unlinkedCount
        ? t("opGroupBothSuccess", { alignments: alignmentCount, segments: unlinkedCount })
        : alignmentCount
          ? t("opGroupAlignmentsSuccess", { count: alignmentCount })
          : t("opGroupSegmentsSuccess", { count: unlinkedCount }));
    } catch (error) {
      const message = errorMessage(error);
      notify(() => t("opGroupFailed", { error: message }));
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
      notify(() => t("opUngroupSuccess"));
    } catch (error) {
      const message = errorMessage(error);
      notify(() => t("opUngroupFailed", { error: message }));
    } finally {
      busy.value = false;
    }
  };

  return { insertAlignmentGap, mergeSegmentContent, splitSegmentContent, groupAlignments, ungroupAlignment };
};
