import { invoke } from "@tauri-apps/api/core";

export type LanguageSide = "source" | "target";
export type WorkspaceMode = "review" | "edit" | "order" | "history";
export type Encoding = "utf8" | "utf8-bom" | "gb18030";
export type SegmentationMode = "non_empty_line" | "sentence_rules" | "legacy_tagged_line";

export interface SegmentDto {
  id: string;
  side: LanguageSide;
  text: string;
  order: number;
}

export interface AlignmentDto {
  id: string;
  sourceIds: string[];
  targetIds: string[];
  status: "provisional" | "manual";
}

export interface ImportProfile {
  encoding: Encoding;
  segmentation_mode: SegmentationMode;
  strip_seg_wrappers: boolean;
  strip_pos_suffixes: boolean;
  compact_cjk_interchar_spaces: boolean;
}

export type TextInput =
  | { kind: "file"; path: string }
  | { kind: "paste"; label: string; text: string };

export interface PreviewImportRequest {
  input: TextInput;
  profile: ImportProfile;
}

export interface SegmentPreview {
  ordinal: number;
  content: string;
}

export interface ImportPreviewResponse {
  label: string;
  profile: ImportProfile;
  had_bom: boolean;
  sha256: string;
  byte_length: number;
  preview: {
    segments: SegmentPreview[];
    cleaned_text: string;
    warnings: string[];
  };
}

export interface ImportSideRequest {
  language_id: string;
  title: string;
  input: TextInput;
  profile: ImportProfile;
}

export interface CreateProjectRequest {
  project_path: string;
  name: string;
  source: ImportSideRequest;
  target: ImportSideRequest;
}

export interface CoreProject {
  project_id: string;
  name: string;
  source_language: string;
  target_language: string;
  document_ids: string[];
  current_revision_id: string;
  format_version: string;
  created_at: string;
  updated_at: string;
}

export interface CoreDocument {
  document_id: string;
  project_id: string;
  language_id: string;
  title: string;
  source_asset_id: string;
  segment_order_id: string;
  created_revision_id: string;
}

export interface CoreSegment {
  segment_id: string;
  document_id: string;
  kind: "sentence";
  content_ref: { key: string };
  content: string;
  content_hash: string;
  created_revision_id: string;
  updated_revision_id: string;
}

export interface CoreSegmentOrder {
  segment_order_id: string;
  document_id: string;
  entries: Array<{ segment_id: string; position_key: string }>;
  updated_revision_id: string;
}

export interface CoreAlignment {
  alignment_id: string;
  project_id: string;
  source_segment_ids: string[];
  target_segment_ids: string[];
  producer: "provisional" | "manual";
  cardinality: "1:1" | "1:n" | "n:1" | "n:m";
  created_revision_id: string;
  updated_revision_id: string;
}

export interface CoreRevision {
  revision_id: string;
  project_id: string;
  parent_revision_id: string | null;
  operation_id: string;
  change_set: { operation: string; affected_count: number };
  author_label: string;
  created_at: string;
  summary: string;
  state: "complete";
}

export interface BookmarkDto {
  bookmark_id: string;
  project_id: string;
  segment_id: string;
  alignment_id: string | null;
  label: string;
  created_at: string;
  updated_at: string;
}

export type AnnotationStatusDto = "draft" | "in_progress" | "resolved";
export interface HumanAnnotationDto {
  annotation_id: string;
  project_id: string;
  title: string;
  body: string;
  status: AnnotationStatusDto;
  linked_segment_ids: string[];
  alignment_id: string | null;
  local_author_label: string;
  created_at: string;
  updated_at: string;
}

export interface ProjectSnapshot {
  contract_version: string;
  project: CoreProject;
  documents: CoreDocument[];
  source_assets: Array<{
    asset_id: string;
    original_path: string | null;
    label: string;
    encoding: Encoding;
    sha256: string;
    byte_length: number;
    imported_at: string;
  }>;
  segments: CoreSegment[];
  segment_orders: CoreSegmentOrder[];
  alignments: CoreAlignment[];
  revisions: CoreRevision[];
  source_profile: ImportProfile;
  target_profile: ImportProfile;
  bookmarks: BookmarkDto[];
  annotations: HumanAnnotationDto[];
}

export interface RevisionListResponse {
  project_id: string;
  current_revision_id: string;
  revisions: CoreRevision[];
}

export interface RevisionComparison {
  project_id: string;
  from_revision_id: string;
  to_revision_id: string;
  segment_changes: Array<{ segment_id: string; before: CoreSegment | null; after: CoreSegment | null }>;
  order_changes: Array<{ document_id: string; before: CoreSegmentOrder | null; after: CoreSegmentOrder | null }>;
  alignment_changes: Array<{ alignment_id: string; before: CoreAlignment | null; after: CoreAlignment | null }>;
}

export interface SearchSegmentsRequest {
  project_id: string;
  query: string;
  regex: boolean;
  case_sensitive: boolean;
  language_id: string | null;
  base_revision_id: string;
}

export interface SegmentSearchHit {
  segment_id: string;
  document_id: string;
  language_id: string;
  content: string;
  alignment_id: string | null;
  revision_id: string;
}

export interface SearchSegmentsResponse { revision_id: string; hits: SegmentSearchHit[] }
export interface ReplacePreviewRequest extends Omit<SearchSegmentsRequest, "query"> { query: string; replacement: string }
export interface ReplacePreviewResponse {
  base_revision_id: string;
  selected_segment_ids: string[];
  items: Array<{ segment_id: string; before: string; after: string }>;
}

export interface AnnotationDraftRequest {
  title: string;
  body: string;
  status: AnnotationStatusDto;
  linked_segment_ids: string[];
  alignment_id: string | null;
  local_author_label?: string;
}

export type ExportFormat = "txt" | "json" | "xml";

export interface ProjectSummaryDto {
  project_id: string;
  name: string;
  source_label: string;
  target_label: string;
  source_count: number;
  target_count: number;
  alignment_count: number;
  source_unlinked_count: number;
  target_unlinked_count: number;
  revision_id: string;
}

export interface KernelClient {
  previewImport(request: PreviewImportRequest): Promise<ImportPreviewResponse>;
  createProject(request: CreateProjectRequest): Promise<ProjectSnapshot>;
  openProject(projectPath: string): Promise<ProjectSnapshot>;
  getCurrentProject(): Promise<ProjectSnapshot>;
  getProjectSummary(): Promise<ProjectSummaryDto>;
  flushProject(): Promise<void>;
  clearCache(): Promise<number>;
  updateSegment(segmentId: string, content: string): Promise<void>;
  moveSegment(segmentId: string, beforeSegmentId?: string, afterSegmentId?: string): Promise<void>;
  reorderSegments(orderedSegmentIds: string[]): Promise<ProjectSnapshot>;
  linkSegments(sourceSegmentIds: string[], targetSegmentIds: string[], replaceExisting: boolean): Promise<ProjectSnapshot>;
  unlinkAlignment(alignmentId: string): Promise<ProjectSnapshot>;
  mergeAlignments(alignmentIds: string[]): Promise<ProjectSnapshot>;
  splitAlignment(alignmentId: string, sourceGroups: string[][], targetGroups: string[][]): Promise<ProjectSnapshot>;
  listRevisions(): Promise<RevisionListResponse>;
  compareRevision(fromRevisionId: string, toRevisionId: string): Promise<RevisionComparison>;
  undo(): Promise<ProjectSnapshot>;
  redo(): Promise<ProjectSnapshot>;
  restoreRevision(targetRevisionId: string): Promise<ProjectSnapshot>;
  searchSegments(request: SearchSegmentsRequest): Promise<SearchSegmentsResponse>;
  previewReplace(request: ReplacePreviewRequest): Promise<ReplacePreviewResponse>;
  applyReplace(preview: ReplacePreviewRequest, selectedSegmentIds: string[]): Promise<ProjectSnapshot>;
  createBookmark(segmentId: string, alignmentId: string | null, label: string): Promise<ProjectSnapshot>;
  deleteBookmark(bookmarkId: string): Promise<ProjectSnapshot>;
  createAnnotation(request: AnnotationDraftRequest): Promise<ProjectSnapshot>;
  updateAnnotation(annotationId: string, request: AnnotationDraftRequest): Promise<ProjectSnapshot>;
  deleteAnnotation(annotationId: string): Promise<ProjectSnapshot>;
  resolveAnnotation(annotationId: string): Promise<ProjectSnapshot>;
  exportProject(format: ExportFormat, outputPath: string): Promise<void>;
}

const inTauri = () => "__TAURI_INTERNALS__" in window;

const placeholderSummary: ProjectSummaryDto = {
  project_id: "fixture-project",
  name: "2024政府工作报告_中英对齐",
  source_label: "report_zh.txt",
  target_label: "report_en.txt",
  source_count: 8,
  target_count: 8,
  alignment_count: 8,
  source_unlinked_count: 0,
  target_unlinked_count: 0,
  revision_id: "0",
};

export const makeImportProfile = (
  encoding: Encoding,
  segmentationMode: SegmentationMode,
): ImportProfile => {
  const legacy = segmentationMode === "legacy_tagged_line";
  return {
    encoding,
    segmentation_mode: segmentationMode,
    strip_seg_wrappers: legacy,
    strip_pos_suffixes: legacy,
    compact_cjk_interchar_spaces: legacy,
  };
};

export const createKernelClient = (): KernelClient => ({
  async previewImport(request) {
    if (!inTauri()) throw new Error("请在 Tauri 桌面应用中预览本地文件");
    return invoke<ImportPreviewResponse>("preview_import", { request });
  },
  async createProject(request) {
    if (!inTauri()) throw new Error("请在 Tauri 桌面应用中创建工程");
    return invoke<ProjectSnapshot>("create_project", { request });
  },
  async openProject(projectPath) {
    if (!inTauri()) throw new Error("请在 Tauri 桌面应用中打开工程");
    return invoke<ProjectSnapshot>("open_project", { projectPath });
  },
  async getCurrentProject() {
    if (!inTauri()) throw new Error("浏览器预览没有已打开的本地工程");
    return invoke<ProjectSnapshot>("get_current_project");
  },
  async getProjectSummary() {
    if (!inTauri()) return placeholderSummary;
    return invoke<ProjectSummaryDto>("get_project_summary");
  },
  async flushProject() {
    if (!inTauri()) return;
    await invoke("flush_project");
  },
  async clearCache() {
    if (!inTauri()) return 0;
    return invoke<number>("clear_cache");
  },
  async updateSegment(segmentId, content) {
    if (!inTauri()) return;
    await invoke("update_segment", { segmentId, content });
  },
  async moveSegment(segmentId, beforeSegmentId, afterSegmentId) {
    if (!inTauri()) return;
    await invoke("move_segment", { segmentId, beforeSegmentId, afterSegmentId });
  },
  async reorderSegments(orderedSegmentIds) {
    if (!inTauri()) throw new Error("浏览器预览不支持持久化排序，请在 Tauri 中操作");
    return invoke<ProjectSnapshot>("reorder_segments", { orderedSegmentIds });
  },
  async linkSegments(sourceSegmentIds, targetSegmentIds, replaceExisting) {
    if (!inTauri()) throw new Error("浏览器预览不支持修改 Alignment，请在 Tauri 中操作");
    return invoke<ProjectSnapshot>("link_segments", { sourceSegmentIds, targetSegmentIds, replaceExisting });
  },
  async unlinkAlignment(alignmentId) {
    if (!inTauri()) throw new Error("浏览器预览不支持修改 Alignment，请在 Tauri 中操作");
    return invoke<ProjectSnapshot>("unlink_alignment", { alignmentId });
  },
  async mergeAlignments(alignmentIds) {
    if (!inTauri()) throw new Error("浏览器预览不支持修改 Alignment，请在 Tauri 中操作");
    return invoke<ProjectSnapshot>("merge_alignments", { alignmentIds });
  },
  async splitAlignment(alignmentId, sourceGroups, targetGroups) {
    if (!inTauri()) throw new Error("浏览器预览不支持修改 Alignment，请在 Tauri 中操作");
    return invoke<ProjectSnapshot>("split_alignment", { alignmentId, sourceGroups, targetGroups });
  },
  async listRevisions() {
    if (!inTauri()) return { project_id: "fixture-project", current_revision_id: "0", revisions: [] };
    return invoke<RevisionListResponse>("list_revisions");
  },
  async compareRevision(fromRevisionId, toRevisionId) {
    return invoke<RevisionComparison>("compare_revision", { fromRevisionId, toRevisionId });
  },
  async undo() { return invoke<ProjectSnapshot>("undo"); },
  async redo() { return invoke<ProjectSnapshot>("redo"); },
  async restoreRevision(targetRevisionId) { return invoke<ProjectSnapshot>("restore_revision", { targetRevisionId }); },
  async searchSegments(request) { return invoke<SearchSegmentsResponse>("search_segments", { request }); },
  async previewReplace(request) { return invoke<ReplacePreviewResponse>("preview_replace", { request }); },
  async applyReplace(preview, selectedSegmentIds) { return invoke<ProjectSnapshot>("apply_replace", { request: { preview, selected_segment_ids: selectedSegmentIds } }); },
  async createBookmark(segmentId, alignmentId, label) { return invoke<ProjectSnapshot>("create_bookmark", { request: { segment_id: segmentId, alignment_id: alignmentId, label } }); },
  async deleteBookmark(bookmarkId) { return invoke<ProjectSnapshot>("delete_bookmark", { bookmarkId }); },
  async createAnnotation(request) { return invoke<ProjectSnapshot>("create_annotation", { request: { ...request, local_author_label: request.local_author_label ?? "本地用户" } }); },
  async updateAnnotation(annotationId, request) { const { local_author_label: _author, ...rest } = request; return invoke<ProjectSnapshot>("update_annotation", { request: { annotation_id: annotationId, ...rest } }); },
  async deleteAnnotation(annotationId) { return invoke<ProjectSnapshot>("delete_annotation", { annotationId }); },
  async resolveAnnotation(annotationId) { return invoke<ProjectSnapshot>("resolve_annotation", { annotationId }); },
  async exportProject(format, outputPath) { await invoke("export_project", { request: { format, output_path: outputPath, include_unlinked: true, side_separator: " " } }); },
});

export const snapshotToWorkspace = (snapshot: ProjectSnapshot) => {
  const [sourceDocument, targetDocument] = snapshot.documents;
  const orderFor = (documentId: string) =>
    snapshot.segment_orders.find((order) => order.document_id === documentId)?.entries ?? [];
  const segmentsById = new Map(snapshot.segments.map((segment) => [segment.segment_id, segment]));
  const mapSide = (document: CoreDocument, side: LanguageSide): SegmentDto[] =>
    orderFor(document.document_id).flatMap((entry, order) => {
      const segment = segmentsById.get(entry.segment_id);
      return segment ? [{ id: segment.segment_id, side, text: segment.content, order }] : [];
    });
  return {
    sourceSegments: mapSide(sourceDocument, "source"),
    targetSegments: mapSide(targetDocument, "target"),
    alignments: snapshot.alignments.map<AlignmentDto>((alignment) => ({
      id: alignment.alignment_id,
      sourceIds: alignment.source_segment_ids,
      targetIds: alignment.target_segment_ids,
      status: alignment.producer,
    })),
  };
};
