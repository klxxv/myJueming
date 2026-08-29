import { nextTick, onBeforeUnmount, ref, type Ref } from "vue";

type TransientSegmentJumpOptions = {
  focus: (segmentId: string) => Promise<boolean | undefined>;
  onMissing?: (segmentId: string) => void;
  durationMs?: number;
};

export function useTransientSegmentJump(options: TransientSegmentJumpOptions): {
  highlightedSegmentId: Ref<string>;
  jumpToSegment: (segmentId: string) => Promise<boolean>;
  clearJumpHighlight: () => void;
} {
  const highlightedSegmentId = ref("");
  let timer: ReturnType<typeof setTimeout> | null = null;

  const clearJumpHighlight = () => {
    if (timer !== null) clearTimeout(timer);
    timer = null;
    highlightedSegmentId.value = "";
  };

  const jumpToSegment = async (segmentId: string) => {
    const focused = await options.focus(segmentId);
    if (!focused) {
      options.onMissing?.(segmentId);
      return false;
    }
    clearJumpHighlight();
    await nextTick();
    highlightedSegmentId.value = segmentId;
    timer = setTimeout(clearJumpHighlight, options.durationMs ?? 2400);
    return true;
  };

  onBeforeUnmount(clearJumpHighlight);
  return { highlightedSegmentId, jumpToSegment, clearJumpHighlight };
}
