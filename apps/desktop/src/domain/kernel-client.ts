import { invoke } from "@tauri-apps/api/core";
import { fallbackLanguages } from "./languages";
import type { Encoding } from "./import-encoding";
import type { AppSettingsEnvelope } from "../settings/schema";

export type LanguageSide = "source" | "target";
export type WorkspaceMode = "review" | "edit" | "order" | "history";
export type AlignmentGapEdge = "before" | "after";
export type { Encoding } from "./import-encoding";
export type SegmentationMode = "non_empty_line" | "sentence_rules" | "legacy_tagged_line";

export interface SegmentDto {
  id: string;
  side: LanguageSide;
  text: string;
  order: number;
  contentLength?: number;
  contentHash?: string;
  loaded?: boolean;
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
  auto_detect_encoding?: boolean;
  input: TextInput;
  profile: ImportProfile;
}

export interface SegmentPreview {
  ordinal: number;
  content: string;
  original_text: string;
  boundary: "non_empty_line" | "sentence_punctuation" | "text_end";
  boundary_marker: string | null;
  cleanups: Array<"seg_wrappers" | "pos_suffixes_and_whitespace" | "cjk_spaces" | "punctuation_spaces">;
}

export interface ImportPreviewResponse {
  encoding_detection: "manual" | "bom" | "utf8" | "statistical" | "unicode_text";
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
  expected_sha256?: string;
  language_id: string;
  title: string;
  input: TextInput;
  profile: ImportProfile;
}

export interface CreateProjectRequest {
  additional_targets?: ImportSideRequest[];
  project_path: string;
  name: string;
  source: ImportSideRequest;
  target: ImportSideRequest;
}

export interface CoreProject {
  comparison?: { source_document_id: string; target_document_ids: string[] };
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
export interface BookmarkPreviewDto extends BookmarkDto {
  document_title: string;
  language_id: string;
  segment_content: string;
  before_context: string | null;
  after_context: string | null;
}
export interface SupportedLanguageDto { language_id: string; native_name: string; english_name: string }

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

export interface ProjectIdentity { project: CoreProject }
export interface SegmentDescriptor { segment_id: string; document_id: string; content_hash: string; content_length: number }
export interface WorkspaceProject extends ProjectIdentity {
  contract_version: string;
  documents: CoreDocument[];
  segments: SegmentDescriptor[];
  segment_orders: CoreSegmentOrder[];
  alignments: CoreAlignment[];
  revisions: CoreRevision[];
  bookmarks: BookmarkDto[];
  annotations: HumanAnnotationDto[];
  summary: ProjectSummaryDto;
}
export interface SegmentText { segment_id: string; content: string; content_hash: string }
export interface ParallelSlice { project_id: string; revision_id: string; segments: SegmentText[] }
export interface CommandScope { project_id: string; base_revision_id: string }
export interface CommandContext extends CommandScope { command_id?: string }
export interface NativeCommandPayloads {
  update_segment: { segment_id: string; content: string };
  move_segment: { segment_id: string; before_segment_id: string | null; after_segment_id: string | null };
  reorder_segments: { ordered_segment_ids: string[] };
  insert_alignment_gap: { segment_id: string; edge: AlignmentGapEdge; target_document_id?: string };
  link_segments: { source_segment_ids: string[]; target_segment_ids: string[]; replace_existing: boolean };
  unlink_alignment: { alignment_id: string };
  merge_segments: { segment_ids: string[]; merged_content: string };
  split_segment: { segment_id: string; parts: string[] };
  group_alignment: { alignment_ids: string[]; unlinked_segment_ids: string[] };
  ungroup_alignment: { alignment_id: string; source_groups: string[][]; target_groups: string[][] };
  create_bookmark: { segment_id: string; alignment_id: string | null; label: string };
  delete_bookmark: { bookmark_id: string };
  create_annotation: AnnotationDraftRequest & { local_author_label: string };
  update_annotation: Omit<AnnotationDraftRequest, "local_author_label"> & { annotation_id: string };
  delete_annotation: { annotation_id: string };
  resolve_annotation: { annotation_id: string };
  apply_replace: { preview: ReplacePreviewRequest; selected_segment_ids: string[] };
  undo: Record<string, never>;
  redo: Record<string, never>;
  restore_revision: { target_revision_id: string };
}
export type NativeCommandKind = keyof NativeCommandPayloads;
export interface NativeCommand<K extends NativeCommandKind = NativeCommandKind> { contract_version: "1.0"; command_id: string; project_id: string; base_revision_id: string; kind: K; payload: NativeCommandPayloads[K] }
export interface CommandResult { command_id: string; project_id: string; committed_revision_id: string; status: "committed" | "duplicate" }
export const newCommandId = () => {
  const random = crypto.randomUUID().replace(/-/g, "");
  const value = Date.now().toString(16).padStart(12, "0") + "7" + random.slice(13);
  return `${value.slice(0,8)}-${value.slice(8,12)}-${value.slice(12,16)}-${value.slice(16,20)}-${value.slice(20)}`;
};

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
  document_ids?: string[] | null;
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
  createProject(request: CreateProjectRequest): Promise<WorkspaceProject>;
  openProject(projectPath: string): Promise<WorkspaceProject>;
  getCurrentProject(): Promise<WorkspaceProject>;
  getProjectSummary(): Promise<ProjectSummaryDto>;
  listSupportedLanguages(): Promise<SupportedLanguageDto[]>;
  flushProject(): Promise<void>;
  clearCache(): Promise<number>;
  updateSegment(segmentId: string, content: string, context?: CommandContext): Promise<CommandResult>;
  dispatchCommand(command: NativeCommand): Promise<CommandResult>;
  loadParallelSlice(project: WorkspaceProject, segmentIds: string[]): Promise<ParallelSlice>;
  moveSegment(segmentId: string, beforeSegmentId?: string, afterSegmentId?: string): Promise<void>;
  reorderSegments(orderedSegmentIds: string[]): Promise<WorkspaceProject>;
  insertAlignmentGap(segmentId: string, edge: AlignmentGapEdge, targetDocumentId?: string): Promise<WorkspaceProject>;
  linkSegments(sourceSegmentIds: string[], targetSegmentIds: string[], replaceExisting: boolean): Promise<WorkspaceProject>;
  unlinkAlignment(alignmentId: string): Promise<WorkspaceProject>;
  mergeSegments(segmentIds: string[], mergedContent: string): Promise<WorkspaceProject>;
  splitSegment(segmentId: string, parts: string[]): Promise<WorkspaceProject>;
  groupAlignments(alignmentIds: string[], unlinkedSegmentIds: string[]): Promise<WorkspaceProject>;
  ungroupAlignment(alignmentId: string, sourceGroups: string[][], targetGroups: string[][]): Promise<WorkspaceProject>;
  listRevisions(): Promise<RevisionListResponse>;
  compareRevision(fromRevisionId: string, toRevisionId: string): Promise<RevisionComparison>;
  undo(): Promise<WorkspaceProject>;
  redo(): Promise<WorkspaceProject>;
  restoreRevision(targetRevisionId: string): Promise<WorkspaceProject>;
  searchSegments(request: SearchSegmentsRequest): Promise<SearchSegmentsResponse>;
  previewReplace(request: ReplacePreviewRequest): Promise<ReplacePreviewResponse>;
  applyReplace(preview: ReplacePreviewRequest, selectedSegmentIds: string[]): Promise<WorkspaceProject>;
  createBookmark(segmentId: string, alignmentId: string | null, label: string): Promise<WorkspaceProject>;
  listBookmarks(): Promise<BookmarkPreviewDto[]>;
  deleteBookmark(bookmarkId: string): Promise<WorkspaceProject>;
  createAnnotation(request: AnnotationDraftRequest, context?: CommandContext): Promise<WorkspaceProject>;
  updateAnnotation(annotationId: string, request: AnnotationDraftRequest, context?: CommandContext): Promise<WorkspaceProject>;
  deleteAnnotation(annotationId: string): Promise<WorkspaceProject>;
  resolveAnnotation(annotationId: string): Promise<WorkspaceProject>;
  exportProject(format: ExportFormat, outputPath: string): Promise<void>;
  loadAppSettings(): Promise<unknown | null>;
  saveAppSettings(settings: AppSettingsEnvelope): Promise<void>;
  resetAppSettings(): Promise<void>;
}

const inTauri = () => "__TAURI_INTERNALS__" in window;
const browserSettingsKey = "jueming-app-settings-v1";

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

export const createKernelClient = (scope: () => CommandScope | null = () => null): KernelClient => {
  const dispatchCommand = async (command: NativeCommand) => {
    try { return await invoke<CommandResult>("execute_command", { command }); }
    catch (cause) {
      if (cause && typeof cause === "object" && "message" in cause && "code" in cause) {
        throw Object.assign(new Error(String(cause.message)), { code: String(cause.code) });
      }
      throw cause;
    }
  };
  const command = <K extends NativeCommandKind>(kind: K, payload: NativeCommandPayloads[K], context: CommandContext | null = scope()) => {
    if (!context || !inTauri()) throw new Error("请先打开真实工程");
    return dispatchCommand({ contract_version: "1.0", command_id: context.command_id ?? newCommandId(), project_id: context.project_id, base_revision_id: context.base_revision_id, kind, payload });
  };
  const mutate = async <K extends NativeCommandKind>(kind: K, payload: NativeCommandPayloads[K], context?: CommandContext) => {
    const result = await command(kind, payload, context);
    const project = await invoke<WorkspaceProject>("get_current_project");
    if (project.project.project_id !== result.project_id) throw new Error("操作已提交，工程已切换");
    return project;
  };
  return {
  dispatchCommand,
  loadParallelSlice(project, segmentIds) {
    return invoke<ParallelSlice>("load_parallel_slice", { request: { project_id: project.project.project_id, revision_id: project.project.current_revision_id, source_document_id: project.documents[0].document_id, target_document_id: project.documents[1].document_id, segment_ids: segmentIds, anchor_segment_id: null, anchor_alignment_id: null, halo: 0 } });
  },
  async previewImport(request) {
    if (!inTauri()) throw new Error("请在 Tauri 桌面应用中预览本地文件");
    return invoke<ImportPreviewResponse>("preview_import", { request });
  },
  async createProject(request) {
    if (!inTauri()) throw new Error("请在 Tauri 桌面应用中创建工程");
    return invoke<WorkspaceProject>("create_project", { request });
  },
  async openProject(projectPath) {
    if (!inTauri()) throw new Error("请在 Tauri 桌面应用中打开工程");
    return invoke<WorkspaceProject>("open_project", { projectPath });
  },
  async getCurrentProject() {
    if (!inTauri()) throw new Error("浏览器预览没有已打开的本地工程");
    return invoke<WorkspaceProject>("get_current_project");
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
  async updateSegment(segmentId, content, context) {
    return command("update_segment", { segment_id: segmentId, content }, context);
  },
  async moveSegment(segmentId, beforeSegmentId, afterSegmentId) {
    if (!inTauri()) return;
    await command("move_segment", { segment_id: segmentId, before_segment_id: beforeSegmentId ?? null, after_segment_id: afterSegmentId ?? null });
  },
  async reorderSegments(orderedSegmentIds) {
    if (!inTauri()) throw new Error("浏览器预览不支持持久化排序，请在 Tauri 中操作");
    return mutate("reorder_segments", { ordered_segment_ids: orderedSegmentIds });
  },
  async listSupportedLanguages() {
    if (!inTauri()) return fallbackLanguages;
    return invoke<SupportedLanguageDto[]>("list_supported_languages");
  },
  async insertAlignmentGap(segmentId, edge, targetDocumentId) {
    if (!inTauri()) throw new Error("浏览器预览不支持修改 Alignment，请在 Tauri 中操作");
    return mutate("insert_alignment_gap", { segment_id: segmentId, edge, target_document_id: targetDocumentId });
  },
  async linkSegments(sourceSegmentIds, targetSegmentIds, replaceExisting) {
    if (!inTauri()) throw new Error("浏览器预览不支持修改 Alignment，请在 Tauri 中操作");
    return mutate("link_segments", { source_segment_ids: sourceSegmentIds, target_segment_ids: targetSegmentIds, replace_existing: replaceExisting });
  },
  async unlinkAlignment(alignmentId) {
    if (!inTauri()) throw new Error("浏览器预览不支持修改 Alignment，请在 Tauri 中操作");
    return mutate("unlink_alignment", { alignment_id: alignmentId });
  },
  async mergeSegments(segmentIds, mergedContent) {
    if (!inTauri()) throw new Error("浏览器预览不支持修改 Segment 内容，请在 Tauri 中操作");
    return mutate("merge_segments", { segment_ids: segmentIds, merged_content: mergedContent });
  },
  async splitSegment(segmentId, parts) {
    if (!inTauri()) throw new Error("浏览器预览不支持修改 Segment 内容，请在 Tauri 中操作");
    return mutate("split_segment", { segment_id: segmentId, parts });
  },
  async groupAlignments(alignmentIds, unlinkedSegmentIds) {
    if (!inTauri()) throw new Error("浏览器预览不支持修改 Alignment，请在 Tauri 中操作");
    return mutate("group_alignment", { alignment_ids: alignmentIds, unlinked_segment_ids: unlinkedSegmentIds });
  },
  async ungroupAlignment(alignmentId, sourceGroups, targetGroups) {
    if (!inTauri()) throw new Error("浏览器预览不支持修改 Alignment，请在 Tauri 中操作");
    return mutate("ungroup_alignment", { alignment_id: alignmentId, source_groups: sourceGroups, target_groups: targetGroups });
  },
  async listRevisions() {
    if (!inTauri()) return { project_id: "fixture-project", current_revision_id: "0", revisions: [] };
    return invoke<RevisionListResponse>("list_revisions");
  },
  async compareRevision(fromRevisionId, toRevisionId) {
    return invoke<RevisionComparison>("compare_revision", { fromRevisionId, toRevisionId });
  },
  async undo() { return mutate("undo", {}); },
  async redo() { return mutate("redo", {}); },
  async restoreRevision(targetRevisionId) { return mutate("restore_revision", { target_revision_id: targetRevisionId }); },
  async searchSegments(request) { return invoke<SearchSegmentsResponse>("search_segments", { request }); },
  async previewReplace(request) { return invoke<ReplacePreviewResponse>("preview_replace", { request }); },
  async applyReplace(preview, selectedSegmentIds) { return mutate("apply_replace", { preview, selected_segment_ids: selectedSegmentIds }); },
  async createBookmark(segmentId, alignmentId, label) { return mutate("create_bookmark", { segment_id: segmentId, alignment_id: alignmentId, label }); },
  async listBookmarks() { return invoke<BookmarkPreviewDto[]>("list_bookmarks"); },
  async deleteBookmark(bookmarkId) { return mutate("delete_bookmark", { bookmark_id: bookmarkId }); },
  async createAnnotation(request, context) { return mutate("create_annotation", { ...request, local_author_label: request.local_author_label ?? "本地用户" }, context); },
  async updateAnnotation(annotationId, request, context) { const { local_author_label: _author, ...rest } = request; return mutate("update_annotation", { annotation_id: annotationId, ...rest }, context); },
  async deleteAnnotation(annotationId) { return mutate("delete_annotation", { annotation_id: annotationId }); },
  async resolveAnnotation(annotationId) { return mutate("resolve_annotation", { annotation_id: annotationId }); },
  async exportProject(format, outputPath) { await invoke("export_project", { request: { format, output_path: outputPath, include_unlinked: true, side_separator: " " } }); },
  async loadAppSettings() {
    if (inTauri()) return invoke<unknown | null>("load_app_settings");
    const raw = localStorage.getItem(browserSettingsKey);
    if (!raw) return null;
    try { return JSON.parse(raw) as unknown; } catch { return null; }
  },
  async saveAppSettings(settings) {
    if (inTauri()) { await invoke("save_app_settings", { settings }); return; }
    localStorage.setItem(browserSettingsKey, JSON.stringify(settings));
  },
  async resetAppSettings() {
    if (inTauri()) { await invoke("reset_app_settings"); return; }
    localStorage.removeItem(browserSettingsKey);
  },
};
};

export const snapshotToWorkspace = (snapshot: WorkspaceProject, texts: ReadonlyMap<string, SegmentText> = new Map(), targetDocumentId?: string) => {
  const sourceDocument = snapshot.documents[0];
  const targetDocument = snapshot.documents.find(document => document.document_id === targetDocumentId) ?? snapshot.documents[1];
  const orderFor = (documentId: string) =>
    snapshot.segment_orders.find((order) => order.document_id === documentId)?.entries ?? [];
  const segmentsById = new Map(snapshot.segments.map((segment) => [segment.segment_id, segment]));
  const mapSide = (document: CoreDocument, side: LanguageSide): SegmentDto[] =>
    orderFor(document.document_id).flatMap((entry, order) => {
      const segment = segmentsById.get(entry.segment_id);
      return segment ? [{ id: segment.segment_id, side, text: texts.get(segment.segment_id)?.content ?? "", loaded: texts.has(segment.segment_id), contentLength: segment.content_length, contentHash: segment.content_hash, order }] : [];
    });
  return {
    sourceSegments: mapSide(sourceDocument, "source"),
    targetSegments: mapSide(targetDocument, "target"),
    alignments: snapshot.alignments.filter(alignment => segmentsById.get(alignment.target_segment_ids[0])?.document_id === targetDocument.document_id).map<AlignmentDto>((alignment) => ({
      id: alignment.alignment_id,
      sourceIds: alignment.source_segment_ids,
      targetIds: alignment.target_segment_ids,
      status: alignment.producer,
    })),
  };
};
