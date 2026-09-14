import { t, type MessageKey } from "./index";

/** Retain a UI diagnostic key so an already-visible error can follow a language change. */
export class LocalizedError extends Error {
  constructor(readonly key: MessageKey) {
    super(key);
    this.name = "LocalizedError";
  }
}

export function rawErrorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  if (error && typeof error === "object" && "message" in error && typeof error.message === "string") return error.message;
  try { return JSON.stringify(error) ?? String(error); } catch { return String(error); }
}

type MessageRule = readonly [RegExp, MessageKey];
// Compatibility with the current string-based Tauri errors. Do not translate errors
// before control-flow checks, and never discard unknown diagnostic details.
const errorRules: readonly MessageRule[] = [
  [/^No local project is open\.$/, "openProjectFirst"],
  [/^The local Kernel state is unavailable; restart Jueming Aligner\.$/, "errorKernelUnavailable"],
  [/^请先打开真实工程$/, "openProjectFirst"],
  [/^请在 Tauri 桌面应用中预览本地文件$/, "desktopPreviewRequired"],
  [/^请在 Tauri 桌面应用中创建工程$/, "desktopCreateRequired"],
  [/^请在 Tauri 桌面应用中打开工程$/, "desktopOpenRequired"],
  [/^浏览器预览没有已打开的本地工程$/, "browserNoProject"],
  [/^浏览器预览不支持持久化排序，请在 Tauri 中操作$/, "desktopOrderRequired"],
  [/^浏览器预览不支持修改 Alignment，请在 Tauri 中操作$/, "desktopAlignmentRequired"],
  [/^浏览器预览不支持修改 Segment 内容，请在 Tauri 中操作$/, "desktopContentRequired"],
  [/^嵌入式 Agent 仅可在已打开本地工程的 Tauri 桌面应用中使用$/, "openProjectFirst"],
  [/^嵌入式 Agent 仅可在本地 Tauri 桌面应用中运行$/, "agentBrowserUnavailable"],
  [/^操作已提交，工程已切换$/, "errorProjectMismatch"],
  [/^both sides must produce at least one segment$/, "errorEmptyImport"],
  [/^pasted Unicode text must use UTF-8$/, "errorUtf8Paste"],
  [/^invalid project snapshot: ([\s\S]+)$/, "errorInvalidProject"],
  [/^segment (.+) does not exist in the current project$/, "errorMissingSegment"],
  [/^alignment (.+) does not exist in the current project$/, "errorMissingAlignment"],
  [/^segment move anchor is not valid for this document$/, "errorMoveAnchor"],
  [/^an alignment selection must contain unique, non-empty source and target segments$/, "errorAlignmentSelection"],
  [/^one or more selected segments are already aligned; confirm replacement before linking$/, "replaceAlignmentWarning"],
  [/^segment (.+) is on the wrong alignment side$/, "errorWrongSide"],
  [/^language (.+) is not in the supported left-to-right language catalogue$/, "errorUnsupportedLanguage"],
  [/^segment merge requires at least two distinct segments$/, "mergeSelectHint"],
  [/^segments can only be merged within one document$/, "mergeSameSideRequired"],
  [/^segments can only be merged when they are consecutive in SegmentOrder$/, "mergeConsecutiveRequired"],
  [/^segments must all be unlinked or all belong to one active alignment before merging$/, "mergeGroupFirst"],
  [/^a segment split requires two or more non-empty parts$/, "errorSplitParts"],
  [/^split parts must concatenate exactly to the original segment content$/, "errorSplitMismatch"],
  [/^merge requires at least two distinct alignments or unlinked segments$/, "groupSelectionHint"],
  [/^segment .+ is already part of an active alignment and cannot be merged as unlinked$/, "unlinkFirstHint"],
  [/^segment (.+) must belong to an active alignment before a gap can be inserted$/, "errorGapLinked"],
  [/^an alignment gap requires at least one later source\/target pair$/, "errorGapRange"],
  [/^ungroup requires at least two non-empty paired groups that partition the original alignment$/, "errorUngroup"],
  [/^revision (.+) is not available$/, "errorRevisionMissing"],
  [/^(?:revision belongs to a different project|project ID does not match the current project)$/, "errorProjectMismatch"],
  [/^no earlier revision is available for undo$/, "errorNoUndo"],
  [/^no redo branch is available from the current revision$/, "errorNoRedo"],
  [/^stale revision: expected (.+), provided (.+)$/, "errorStaleRevision"],
  [/^invalid search regular expression: ([\s\S]+)$/, "errorRegex"],
  [/^replace selection does not belong to the preview$/, "errorReplaceSelection"],
  [/^bookmark (.+) does not exist$/, "errorBookmarkMissing"],
  [/^annotation (.+) does not exist$/, "errorAnnotationMissing"],
  [/^annotation links must contain unique existing segments$/, "errorAnnotationLinks"],
  [/^alignment anchor does not include the bookmarked\/annotated segment$/, "errorAnchorMismatch"],
  [/^project directory must use the .jm extension: ([\s\S]+)$/, "errorProjectExtension"],
  [/^I\/O error at ([\s\S]+)$/, "errorIo"],
  [/^invalid project JSON at ([\s\S]+)$/, "errorInvalidProject"],
  [/^text is not valid ([\s\S]+)$/, "errorDecode"],
  [/^GB18030 decoding is unavailable on this platform$/, "errorGbUnavailable"],
  [/^(alignment must contain at least one segment on each side|an alignment contains duplicate segment references|segment order has duplicate or missing segment references|segment belongs to the wrong document|alignment contains a segment from the wrong project\/document|project contains duplicate or missing document references|segment .+ belongs to more than one active alignment)$/, "errorInvariant"],
  [/^(?:The cache cleanup worker failed|path has no parent directory|export JSON failed): ([\s\S]+)$/, "errorUnknown"],
];

function matchMessage(value: string, rules: readonly MessageRule[]): string | undefined {
  for (const [pattern, key] of rules) {
    const match = value.match(pattern);
    if (match) return t(key, Object.fromEntries(match.slice(1).map((value, index) => [`p${index}`, value])));
  }
}

export function formatError(error: unknown): string {
  if (error instanceof LocalizedError) return t(error.key);
  const raw = rawErrorMessage(error);
  // Known structured error codes remain separate from human-readable diagnostics.
  if (error && typeof error === "object" && "code" in error) {
    if (error.code === "cancelled") return t("cancel");
  }
  return matchMessage(raw, errorRules) ?? raw;
}

const operations: Record<string, MessageKey> = {
  create_project: "newProject", replace_segments: "applyReplace",
  create_bookmark: "addBookmark", update_bookmark: "bookmarkInfo", delete_bookmark: "removeBookmark",
  create_annotation: "newAnnotation", update_annotation: "annotationUpdated", delete_annotation: "annotationDeleted",
  resolve_annotation: "annotationResolved", update_segment: "edit", move_segment: "dragOrder", reorder_segments: "dragOrder",
  insert_alignment_gap: "gapInserted", link_alignment: "link", unlink_alignment: "unlink",
  merge_segments: "mergeContent", split_segment: "splitContent", group_alignment: "groupAction", ungroup_alignment: "ungroup",
  undo: "undo", redo: "redo", restore: "restoreVersion",
};

export function revisionAction(operation: string): string {
  const base = operation.split(":")[0];
  return base === "insert_alignment_gap" ? t("orderActions") : operations[base] ? t(operations[base]) : operation;
}

const revisionRules: readonly MessageRule[] = [
  [/^Imported (\d+) source and (\d+) target segments$/, "revisionImport"],
  [/^Replaced text in (\d+) segments$/, "revisionReplace"],
  [/^Created bookmark on segment (.+)$/, "revisionBookmarkCreate"],
  [/^Updated bookmark (.+)$/, "revisionBookmarkUpdate"],
  [/^Deleted bookmark (.+)$/, "revisionBookmarkDelete"],
  [/^Created annotation ([\s\S]+)$/, "revisionAnnotationCreate"],
  [/^Updated annotation (.+)$/, "revisionAnnotationUpdate"],
  [/^Deleted annotation (.+)$/, "revisionAnnotationDelete"],
  [/^Undo to revision (.+)$/, "revisionUndo"],
  [/^Redo to revision (.+)$/, "revisionRedo"],
  [/^Restored revision (.+)$/, "revisionRestore"],
  [/^Edited segment (.+)$/, "revisionEdit"],
  [/^Moved segment (.+)$/, "revisionMove"],
  [/^Reordered (\d+) segments$/, "revisionReorder"],
  [/^Linked (\d+) source and (\d+) target segments$/, "revisionLink"],
  [/^Unlinked alignment (.+)$/, "revisionUnlink"],
  [/^Merged (\d+) segments into (.+)$/, "revisionMerge"],
  [/^Split segment (.+) into (\d+) parts$/, "revisionSplit"],
  [/^Grouped (\d+) alignments and (\d+) unlinked segments$/, "revisionGroup"],
  [/^Ungrouped alignment (.+) into (\d+) groups$/, "revisionUngroup"],
];

export function revisionSummary(summary: string): string {
  const gap = summary.match(/^Inserted alignment gap (Before|After) segment (.+) and realigned (\d+) pairs$/);
  if (gap) return t("revisionGap", { p0: t(gap[1] === "Before" ? "above" : "below"), p1: gap[2], p2: Number(gap[3]) });
  // Generated metadata only; never rewrite user-authored titles, bodies or stored history.
  return matchMessage(summary, revisionRules) ?? summary;
}

export function importWarning(warning: string): string {
  return warning === "文本未产生可用 Segment" ? t("importEmptyWarning") : warning;
}
