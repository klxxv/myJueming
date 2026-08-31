import { t, type LocalizedMessage } from '../i18n';
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
  notify: (message: LocalizedMessage) => void;
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
    const isSource = sourceRows.value.some((segment) => segment.id === segmentId);
    busy.value = true;
    try {
      await applySnapshot(await kernelClient.insertAlignmentGap(segmentId, edge));
      selectedAlignmentId.value = alignmentRows.value.find((alignment) =>
        alignment.sourceIds.includes(segmentId) || alignment.targetIds.includes(segmentId))?.id ?? selectedAlignmentId.value;
      notify(() => t('gapInserted', { p0: t(isSource ? 'sourceSide' : 'targetSide'), p1: edge === "before" ? t('above') : t('below') }));
    } catch (error) {
      notify(() => t('gapFailed', { p0: errorMessage(error) }));
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
      notify(() => t('segmentsMerged', { p0: segmentIds.length }));
    } catch (error) {
      notify(() => t('mergeFailed', { p0: errorMessage(error) }));
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
      notify(() => t('segmentSplit', { p0: parts.length }));
    } catch (error) {
      notify(() => t('splitFailed', { p0: errorMessage(error) }));
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
      notify(() => t('alignmentsGrouped', { p0: [
        alignmentIds.length ? t('alignmentCount', { p0: alignmentIds.length }) : "",
        unlinkedSegmentIds.length ? t('unalignedCount', { p0: unlinkedSegmentIds.length }) : "",
      ].filter(Boolean).join(t('and')) }));
    } catch (error) {
      notify(() => t('groupFailed', { p0: errorMessage(error) }));
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
      notify(() => t('alignmentUngrouped'));
    } catch (error) {
      notify(() => t('ungroupFailed', { p0: errorMessage(error) }));
    } finally {
      busy.value = false;
    }
  };

  return { insertAlignmentGap, mergeSegmentContent, splitSegmentContent, groupAlignments, ungroupAlignment };
};
