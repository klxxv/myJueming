<script setup lang="ts">
import { ArrowRight, Info, Search, Star, X } from "@lucide/vue";
import { computed, ref, watch } from "vue";
import type { AlignmentDto, BookmarkDto, BookmarkPreviewDto, SegmentDto } from "../domain/kernel-client";

const props = defineProps<{
  bookmarks: BookmarkDto[];
  previews: BookmarkPreviewDto[];
  sourceSegments: SegmentDto[];
  targetSegments: SegmentDto[];
  alignments: AlignmentDto[];
  segmentLabels: Map<string, string>;
  alignmentLabels: Map<string, string>;
}>();
const emit = defineEmits<{ open: [segmentId: string, alignmentId: string | null]; remove: [bookmarkId: string] }>();
const query = ref("");
const sortMode = ref<"created" | "document">("created");
const selectedBookmarkId = ref("");
const previewsByBookmarkId = computed(() => new Map(props.previews.map((preview) => [preview.bookmark_id, preview])));
const allSegments = computed(() => new Map([...props.sourceSegments, ...props.targetSegments].map((segment) => [segment.id, segment])));
const filteredBookmarks = computed(() => {
  const needle = query.value.trim().toLocaleLowerCase();
  const matches = !needle ? [...props.bookmarks] : props.bookmarks.filter((bookmark) => {
    const preview = previewsByBookmarkId.value.get(bookmark.bookmark_id);
    return [bookmark.label, bookmark.segment_id, bookmark.alignment_id, preview?.segment_content]
      .some((value) => value?.toLocaleLowerCase().includes(needle));
  });
  return matches.sort((left, right) => sortMode.value === "created"
    ? new Date(right.created_at).getTime() - new Date(left.created_at).getTime()
    : (allSegments.value.get(left.segment_id)?.order ?? 0) - (allSegments.value.get(right.segment_id)?.order ?? 0));
});
watch(() => props.bookmarks, (bookmarks) => {
  if (!bookmarks.some((bookmark) => bookmark.bookmark_id === selectedBookmarkId.value)) selectedBookmarkId.value = bookmarks[0]?.bookmark_id ?? "";
}, { immediate: true });
const selectedBookmark = computed(() => props.bookmarks.find((bookmark) => bookmark.bookmark_id === selectedBookmarkId.value) ?? props.bookmarks[0]);
const selectedPreview = computed(() => selectedBookmark.value ? previewsByBookmarkId.value.get(selectedBookmark.value.bookmark_id) : undefined);
const selectedAlignment = computed(() => selectedBookmark.value?.alignment_id
  ? props.alignments.find((alignment) => alignment.id === selectedBookmark.value?.alignment_id)
  : undefined);
const selectedSource = computed(() => selectedAlignment.value?.sourceIds.map((id) => allSegments.value.get(id)?.text).filter(Boolean).join("\n") ?? "");
const selectedTarget = computed(() => selectedAlignment.value?.targetIds.map((id) => allSegments.value.get(id)?.text).filter(Boolean).join("\n") ?? "");
const segmentLabel = (segmentId: string) => props.segmentLabels.get(segmentId) ?? segmentId.slice(0, 8);
const alignmentLabel = (alignmentId: string | null) => alignmentId ? props.alignmentLabels.get(alignmentId) ?? alignmentId.slice(0, 8) : "未对齐";
const languageLabel = (bookmark: BookmarkDto) => props.sourceSegments.some((segment) => segment.id === bookmark.segment_id) ? "中文" : "English";
const excerpt = (bookmark: BookmarkDto) => previewsByBookmarkId.value.get(bookmark.bookmark_id)?.segment_content ?? bookmark.label;
const formatDate = (value: string) => new Date(value).toLocaleString("zh-CN", { hour12: false, year: "numeric", month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit" });
</script>

<template>
  <section class="bookmarks-workspace">
    <aside class="bookmark-index">
      <header><div><h2>书签</h2><span>{{ bookmarks.length }}</span></div><div class="bookmark-index__tools"><label><Search :size="15" /><input v-model="query" type="search" placeholder="搜索书签内容或段落 ID…" /></label><select v-model="sortMode" aria-label="书签排序方式"><option value="created">创建时间</option><option value="document">文档顺序</option></select></div></header>
      <div v-if="filteredBookmarks.length" class="bookmark-scroll">
        <article v-for="bookmark in filteredBookmarks" :key="bookmark.bookmark_id" class="bookmark-row" :class="{ active: selectedBookmark?.bookmark_id === bookmark.bookmark_id }">
          <button class="bookmark-row__select" type="button" @click="selectedBookmarkId = bookmark.bookmark_id"><Star :size="18" :fill="selectedBookmark?.bookmark_id === bookmark.bookmark_id ? 'currentColor' : 'none'" /><span><strong>{{ segmentLabel(bookmark.segment_id) }} <i>· {{ languageLabel(bookmark) }}</i></strong><em>{{ excerpt(bookmark) }}</em></span></button>
          <button class="bookmark-row__remove" type="button" title="移除书签" aria-label="移除书签" @click="emit('remove', bookmark.bookmark_id)"><X :size="14" /></button>
        </article>
      </div>
      <p v-else class="bookmark-empty">{{ bookmarks.length ? '没有匹配的书签。' : '当前工程暂无书签。可在平行视图中点击句段星标。' }}</p>
    </aside>

    <main v-if="selectedBookmark" class="bookmark-detail">
      <header><div><h2>书签 {{ segmentLabel(selectedBookmark.segment_id) }}</h2><Star :size="20" fill="currentColor" /></div><dl><div><dt>语言</dt><dd>{{ languageLabel(selectedBookmark) }}</dd></div><div><dt>段落 ID</dt><dd>{{ segmentLabel(selectedBookmark.segment_id) }}</dd></div><div><dt>Alignment</dt><dd>{{ alignmentLabel(selectedBookmark.alignment_id) }}</dd></div><div><dt>创建时间</dt><dd>{{ formatDate(selectedBookmark.created_at) }}</dd></div></dl></header>
      <button class="bookmark-return" type="button" @click="emit('open', selectedBookmark.segment_id, selectedBookmark.alignment_id)"><ArrowRight :size="16" />回到平行视图</button>
      <p class="bookmark-note"><Info :size="14" />将按稳定的 Segment ID 定位，并用绿色边框标出准确位置。</p>

      <section class="context-preview"><h3>上下文预览</h3><div>
        <article><span>中文 <small>zh</small></span><p>{{ selectedSource || (selectedPreview?.language_id === 'zh' ? selectedPreview.segment_content : '此书签当前没有对应的中文 Alignment。') }}</p></article>
        <article><span>English <small>en</small></span><p>{{ selectedTarget || (selectedPreview?.language_id !== 'zh' ? selectedPreview?.segment_content : 'This bookmark currently has no aligned English segment.') }}</p></article>
      </div></section>

      <section class="bookmark-metadata"><h3>书签信息</h3><dl><div><dt>文档</dt><dd>{{ selectedPreview?.document_title || '当前工程' }}</dd></div><div><dt>锚点</dt><dd>{{ selectedBookmark.segment_id }}</dd></div><div><dt>对齐状态</dt><dd>{{ selectedBookmark.alignment_id ? '已关联 Alignment' : '未对齐' }}</dd></div><div><dt>导航依据</dt><dd>稳定 Segment ID</dd></div></dl></section>
    </main>
    <main v-else class="bookmark-detail bookmark-detail--empty"><Star :size="30" /><h2>还没有书签</h2><p>在平行视图中为重要句段添加星标，之后就能从这里快速返回。</p></main>
  </section>
</template>

<style scoped>
.bookmarks-workspace { display: grid; width: 100%; height: 100%; min-height: 0; grid-template-columns: minmax(340px, 38%) minmax(0, 1fr); color: var(--ink-900); background: var(--paper); }
.bookmark-index { display: flex; min-width: 0; min-height: 0; flex-direction: column; border-right: 1px solid var(--line); background: var(--surface-subtle); }.bookmark-index > header { flex: 0 0 auto; padding: 25px 26px 16px; }.bookmark-index > header > div:first-child { display: flex; align-items: baseline; gap: 12px; }.bookmark-index h2 { margin: 0; color: var(--green-900); font-size: 23px; }.bookmark-index header span { color: var(--ink-500); font-size: 15px; }.bookmark-index__tools { display: grid; grid-template-columns: minmax(0, 1fr) 112px; gap: 9px; margin-top: 17px; }.bookmark-index label { display: flex; height: 38px; align-items: center; gap: 8px; padding: 0 11px; border: 1px solid #cbd4cc; border-radius: 7px; color: var(--ink-500); background: var(--surface-input); }.bookmark-index input { width: 100%; min-width: 0; border: 0; outline: 0; color: var(--ink-900); background: transparent; font-size: 12px; }.bookmark-index select { min-width: 0; height: 38px; padding: 0 8px; border: 1px solid #cbd4cc; border-radius: 7px; color: var(--ink-700); background: var(--surface-input); font-size: 11px; }
.bookmark-scroll { min-height: 0; overflow-y: auto; padding: 0 18px 26px; scrollbar-width: none; }.bookmark-scroll::-webkit-scrollbar { display: none; }.bookmark-row { position: relative; display: grid; width: 100%; min-height: 66px; grid-template-columns: minmax(0, 1fr) 28px; align-items: center; border-top: 1px solid var(--line); color: var(--ink-700); background: transparent; }.bookmark-row:first-child { border-top: 0; }.bookmark-row:hover { background: var(--surface-hover); }.bookmark-row.active { border-radius: 7px; color: var(--green-900); background: var(--surface-green-selected); box-shadow: inset 3px 0 var(--green-700); }.bookmark-row__select { display: grid; min-width: 0; grid-template-columns: 25px minmax(0, 1fr); align-items: center; gap: 9px; padding: 10px 8px 10px 13px; border: 0; color: inherit; background: transparent; text-align: left; cursor: pointer; }.bookmark-row__select > svg { color: var(--green-700); }.bookmark-row__select > span { display: grid; min-width: 0; gap: 6px; }.bookmark-row strong { font-size: 12px; }.bookmark-row strong i { color: var(--ink-500); font-size: 10px; font-style: normal; font-weight: 500; }.bookmark-row em { overflow: hidden; color: var(--ink-700); font-size: 11px; font-style: normal; text-overflow: ellipsis; white-space: nowrap; }.bookmark-row__remove { display: grid; width: 26px; height: 26px; place-items: center; border: 0; border-radius: 5px; color: var(--ink-500); background: transparent; cursor: pointer; opacity: 0; }.bookmark-row:hover .bookmark-row__remove, .bookmark-row.active .bookmark-row__remove, .bookmark-row__remove:focus-visible { opacity: 1; }.bookmark-row__remove:hover { color: #a24c4c; background: var(--surface-danger-soft); }.bookmark-empty { margin: 18px; padding: 22px; border: 1px dashed #c8d3c9; border-radius: 8px; color: var(--ink-500); font-size: 12px; line-height: 1.6; }
.bookmark-detail { min-width: 0; min-height: 0; overflow-y: auto; padding: 28px 32px 42px; scrollbar-width: none; }.bookmark-detail::-webkit-scrollbar { display: none; }.bookmark-detail > header > div { display: flex; align-items: center; gap: 11px; color: var(--green-900); }.bookmark-detail > header h2 { margin: 0; font-size: 23px; }.bookmark-detail > header dl, .bookmark-metadata dl { display: flex; flex-wrap: wrap; gap: 8px 0; margin: 14px 0 0; }.bookmark-detail > header dl div { display: flex; gap: 7px; padding: 0 14px; border-left: 1px solid var(--line); font-size: 10px; }.bookmark-detail > header dl div:first-child { padding-left: 0; border-left: 0; }.bookmark-detail dt { color: var(--ink-500); }.bookmark-detail dd { margin: 0; color: var(--ink-700); font-family: ui-monospace, SFMono-Regular, Consolas, monospace; }
.bookmark-return { display: inline-flex; height: 38px; align-items: center; gap: 8px; margin-top: 22px; padding: 0 16px; border: 1px solid var(--green-900); border-radius: 7px; color: #fff; background: var(--green-900); cursor: pointer; }.bookmark-return:hover { background: var(--green-700); }.bookmark-note { display: flex; align-items: center; gap: 7px; margin: 10px 0 26px; color: var(--ink-500); font-size: 10px; }
.context-preview h3, .bookmark-metadata h3 { margin: 0 0 12px; font-size: 14px; }.context-preview > div { overflow: hidden; border: 1px solid var(--line); border-radius: 9px; background: var(--surface-raised); }.context-preview article { padding: 17px 18px; }.context-preview article + article { border-top: 1px solid var(--line); }.context-preview article > span { display: inline-flex; align-items: center; gap: 6px; padding: 4px 8px; border-radius: 5px; color: var(--green-900); background: var(--surface-green-soft); font-size: 11px; font-weight: 700; }.context-preview article small { font-size: 9px; font-weight: 500; }.context-preview p { margin: 12px 0 0; font-size: calc(14px * var(--reading-font-scale)); line-height: 1.7; white-space: pre-line; }
.bookmark-metadata { margin-top: 27px; }.bookmark-metadata dl { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); border-top: 1px solid var(--line); }.bookmark-metadata dl div { display: grid; grid-template-columns: 90px minmax(0, 1fr); gap: 12px; padding: 11px 0; border-bottom: 1px solid var(--line); font-size: 10px; }.bookmark-metadata dd { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }.bookmark-detail--empty { display: grid; place-content: center; justify-items: center; color: var(--ink-500); text-align: center; }.bookmark-detail--empty h2 { margin: 14px 0 5px; color: var(--green-900); font-size: 20px; }.bookmark-detail--empty p { max-width: 360px; font-size: 12px; line-height: 1.6; }
@media (max-width: 1050px) { .bookmarks-workspace { grid-template-columns: 330px minmax(0, 1fr); }.bookmark-detail { padding-inline: 24px; }.bookmark-metadata dl { grid-template-columns: 1fr; } }
</style>
