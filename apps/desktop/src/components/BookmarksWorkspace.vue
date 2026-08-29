<script setup lang="ts">
import { computed } from "vue";
import { Star, X } from "@lucide/vue";
import type { BookmarkDto, BookmarkPreviewDto } from "../domain/kernel-client";

const props = defineProps<{
  bookmarks: BookmarkDto[];
  previews: BookmarkPreviewDto[];
  segmentLabels: Map<string, string>;
  alignmentLabels: Map<string, string>;
}>();

const emit = defineEmits<{
  open: [segmentId: string, alignmentId: string | null];
  remove: [bookmarkId: string];
}>();

const previewsByBookmarkId = computed(() => new Map(props.previews.map((preview) => [preview.bookmark_id, preview])));
const previewText = (bookmark: BookmarkDto) => {
  const preview = previewsByBookmarkId.value.get(bookmark.bookmark_id);
  if (!preview) return "当前版本暂未提供此书签的句段预览。";

  return [preview.before_context, preview.segment_content, preview.after_context]
    .filter((value): value is string => Boolean(value?.trim()))
    .join(" · ")
    .replace(/\s+/g, " ")
    .trim();
};

const segmentLabel = (segmentId: string) => props.segmentLabels.get(segmentId) ?? segmentId.slice(0, 8);
const alignmentLabel = (alignmentId: string | null) => alignmentId ? props.alignmentLabels.get(alignmentId) ?? alignmentId.slice(0, 8) : "未对齐";
</script>

<template>
  <section class="bookmarks-workspace aux-view bookmarks-view">
    <Star :size="28" />
    <h2>书签</h2>
    <p>书签始终锚定稳定的句段 ID；关系 ID 仅用作导航提示。</p>
    <div v-if="bookmarks.length" class="bookmark-list">
      <article v-for="bookmark in bookmarks" :key="bookmark.bookmark_id">
        <button type="button" @click="emit('open', bookmark.segment_id, bookmark.alignment_id)">
          <Star :size="16" fill="currentColor" />
          <span>
            <b>{{ bookmark.label }}</b>
            <small>{{ segmentLabel(bookmark.segment_id) }} · {{ alignmentLabel(bookmark.alignment_id) }}</small>
            <em>{{ previewText(bookmark) }}</em>
          </span>
        </button>
        <button type="button" title="移除书签" @click="emit('remove', bookmark.bookmark_id)"><X :size="15" /></button>
      </article>
    </div>
    <p v-else class="empty-copy">当前工程暂无书签。可在平行视图中点击句段右侧的星标。</p>
  </section>
</template>

<style scoped>
.bookmarks-workspace { justify-content: center; }
.bookmark-list { display: grid; gap: 8px; width: min(560px, 72vw); max-height: 420px; overflow: auto; }
.bookmark-list article { display: grid; grid-template-columns: 1fr 36px; align-items: center; border: 1px solid var(--line); border-radius: 7px; background: var(--paper); }
.bookmark-list button { display: flex; align-items: center; gap: 11px; min-width: 0; padding: 11px 13px; border: 0; color: var(--green-900); background: transparent; text-align: left; cursor: pointer; }
.bookmark-list button:last-child { justify-content: center; padding-inline: 8px; color: var(--ink-500); }
.bookmark-list span { display: grid; gap: 3px; }
.bookmark-list small { color: var(--ink-500); font-family: ui-monospace, Consolas, monospace; }
.bookmark-list em { overflow: hidden; color: var(--ink-700); font-size: 12px; font-style: normal; line-height: 1.4; text-overflow: ellipsis; white-space: nowrap; }
.empty-copy { padding: 20px; border: 1px dashed #c8d3c9; border-radius: 7px; }
</style>
