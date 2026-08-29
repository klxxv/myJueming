//! Versioned, serde-friendly DTOs for the local `KernelClient` boundary.
//!
//! The DTO layer reuses core's typed IDs and canonical objects; it does not
//! introduce a second representation of Segment or Alignment identity.

pub use jueming_core::{
    AlignmentId, AnnotationId, AssetId, BookmarkId, CommandId, DocumentId, Encoding, ImportProfile,
    OperationId, ProjectId, RevisionId, SegmentId, SegmentOrderId, SegmentationMode,
};
use serde::{Deserialize, Serialize};

pub const CONTRACT_VERSION: &str = "1.0";

/// Canonical IDs accepted by the MVP's left-to-right language chooser. They
/// serialize as BCP-47 base language codes; `zh-CN` is accepted separately as
/// a legacy input when opening or creating an older project.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SupportedLanguageId {
    En,
    Zh,
    Hi,
    Es,
    Fr,
    Bn,
    Pt,
    Ru,
    Id,
    De,
}

impl SupportedLanguageId {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Zh => "zh",
            Self::Hi => "hi",
            Self::Es => "es",
            Self::Fr => "fr",
            Self::Bn => "bn",
            Self::Pt => "pt",
            Self::Ru => "ru",
            Self::Id => "id",
            Self::De => "de",
        }
    }

    pub fn parse_compatible(value: &str) -> Option<Self> {
        Some(match value {
            "en" => Self::En,
            "zh" | "zh-CN" => Self::Zh,
            "hi" => Self::Hi,
            "es" => Self::Es,
            "fr" => Self::Fr,
            "bn" => Self::Bn,
            "pt" => Self::Pt,
            "ru" => Self::Ru,
            "id" => Self::Id,
            "de" => Self::De,
            _ => return None,
        })
    }
}

/// The initial language chooser catalogue.  All entries use a left-to-right
/// script, while language IDs remain ordinary BCP-47 strings so older project
/// files (for example `zh-CN`) stay readable.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct SupportedLanguage {
    pub language_id: &'static str,
    pub native_name: &'static str,
    pub english_name: &'static str,
}

pub const COMMON_LTR_LANGUAGES: [SupportedLanguage; 10] = [
    SupportedLanguage {
        language_id: "en",
        native_name: "English",
        english_name: "English",
    },
    SupportedLanguage {
        language_id: "zh",
        native_name: "\u{4e2d}\u{6587}",
        english_name: "Chinese",
    },
    SupportedLanguage {
        language_id: "hi",
        native_name: "\u{0939}\u{093f}\u{0928}\u{094d}\u{0926}\u{0940}",
        english_name: "Hindi",
    },
    SupportedLanguage {
        language_id: "es",
        native_name: "Espa\u{00f1}ol",
        english_name: "Spanish",
    },
    SupportedLanguage {
        language_id: "fr",
        native_name: "Fran\u{00e7}ais",
        english_name: "French",
    },
    SupportedLanguage {
        language_id: "bn",
        native_name: "\u{09ac}\u{09be}\u{0982}\u{09b2}\u{09be}",
        english_name: "Bengali",
    },
    SupportedLanguage {
        language_id: "pt",
        native_name: "Portugu\u{00ea}s",
        english_name: "Portuguese",
    },
    SupportedLanguage {
        language_id: "ru",
        native_name: "\u{0420}\u{0443}\u{0441}\u{0441}\u{043a}\u{0438}\u{0439}",
        english_name: "Russian",
    },
    SupportedLanguage {
        language_id: "id",
        native_name: "Bahasa Indonesia",
        english_name: "Indonesian",
    },
    SupportedLanguage {
        language_id: "de",
        native_name: "Deutsch",
        english_name: "German",
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlignmentGapEdge {
    Before,
    After,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum TextInput {
    File { path: String },
    Paste { label: String, text: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PreviewImportRequest {
    pub input: TextInput,
    pub profile: ImportProfile,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ImportPreviewResponse {
    pub label: String,
    pub profile: ImportProfile,
    pub had_bom: bool,
    pub sha256: String,
    pub byte_length: u64,
    pub preview: SegmentationPreview,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ImportSideRequest {
    pub language_id: String,
    pub title: String,
    pub input: TextInput,
    pub profile: ImportProfile,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CreateProjectRequest {
    pub project_path: String,
    pub name: String,
    pub source: ImportSideRequest,
    pub target: ImportSideRequest,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceAssetRecord {
    pub asset_id: AssetId,
    pub original_path: Option<String>,
    pub label: String,
    pub encoding: Encoding,
    pub sha256: String,
    pub byte_length: u64,
    pub imported_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectSnapshot {
    pub contract_version: String,
    pub project: Project,
    pub documents: Vec<Document>,
    pub source_assets: Vec<SourceAssetRecord>,
    pub segments: Vec<Segment>,
    pub segment_orders: Vec<SegmentOrder>,
    pub alignments: Vec<Alignment>,
    pub revisions: Vec<Revision>,
    pub source_profile: ImportProfile,
    pub target_profile: ImportProfile,
    #[serde(default)]
    pub bookmarks: Vec<Bookmark>,
    #[serde(default)]
    pub annotations: Vec<HumanAnnotation>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Bookmark {
    pub bookmark_id: BookmarkId,
    pub project_id: ProjectId,
    pub segment_id: SegmentId,
    pub alignment_id: Option<AlignmentId>,
    pub label: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Read model for the bookmark panel. The canonical bookmark remains a small
/// sidecar record; preview text is derived from the current snapshot so it can
/// never become stale after content edits, merges, or splits.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BookmarkPreview {
    #[serde(flatten)]
    pub bookmark: Bookmark,
    pub document_title: String,
    pub language_id: String,
    pub segment_content: String,
    pub before_context: Option<String>,
    pub after_context: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnnotationStatus {
    Draft,
    InProgress,
    Resolved,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HumanAnnotation {
    pub annotation_id: AnnotationId,
    pub project_id: ProjectId,
    pub title: String,
    pub body: String,
    pub status: AnnotationStatus,
    pub linked_segment_ids: Vec<SegmentId>,
    pub alignment_id: Option<AlignmentId>,
    pub local_author_label: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub project_id: ProjectId,
    pub name: String,
    pub source_label: String,
    pub target_label: String,
    pub source_count: u64,
    pub target_count: u64,
    pub alignment_count: u64,
    pub source_unlinked_count: u64,
    pub target_unlinked_count: u64,
    pub revision_id: RevisionId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RevisionListResponse {
    pub project_id: ProjectId,
    pub current_revision_id: RevisionId,
    pub revisions: Vec<Revision>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SegmentRevisionDiff {
    pub segment_id: SegmentId,
    pub before: Option<Segment>,
    pub after: Option<Segment>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrderRevisionDiff {
    pub document_id: DocumentId,
    pub before: Option<SegmentOrder>,
    pub after: Option<SegmentOrder>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AlignmentRevisionDiff {
    pub alignment_id: AlignmentId,
    pub before: Option<Alignment>,
    pub after: Option<Alignment>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RevisionComparison {
    pub project_id: ProjectId,
    pub from_revision_id: RevisionId,
    pub to_revision_id: RevisionId,
    pub segment_changes: Vec<SegmentRevisionDiff>,
    pub order_changes: Vec<OrderRevisionDiff>,
    pub alignment_changes: Vec<AlignmentRevisionDiff>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SearchSegmentsRequest {
    pub project_id: ProjectId,
    pub query: String,
    pub regex: bool,
    pub case_sensitive: bool,
    pub language_id: Option<String>,
    pub base_revision_id: RevisionId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SegmentSearchHit {
    pub segment_id: SegmentId,
    pub document_id: DocumentId,
    pub language_id: String,
    pub content: String,
    pub alignment_id: Option<AlignmentId>,
    pub revision_id: RevisionId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SearchSegmentsResponse {
    pub revision_id: RevisionId,
    pub hits: Vec<SegmentSearchHit>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReplacePreviewRequest {
    pub project_id: ProjectId,
    pub query: String,
    pub replacement: String,
    pub regex: bool,
    pub case_sensitive: bool,
    pub language_id: Option<String>,
    pub base_revision_id: RevisionId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReplacePreviewItem {
    pub segment_id: SegmentId,
    pub before: String,
    pub after: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReplacePreviewResponse {
    pub base_revision_id: RevisionId,
    pub selected_segment_ids: Vec<SegmentId>,
    pub items: Vec<ReplacePreviewItem>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReplaceApplyRequest {
    pub preview: ReplacePreviewRequest,
    pub selected_segment_ids: Vec<SegmentId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BookmarkCreateRequest {
    pub segment_id: SegmentId,
    pub alignment_id: Option<AlignmentId>,
    pub label: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BookmarkUpdateRequest {
    pub bookmark_id: BookmarkId,
    pub segment_id: SegmentId,
    pub alignment_id: Option<AlignmentId>,
    pub label: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AnnotationCreateRequest {
    pub title: String,
    pub body: String,
    pub status: AnnotationStatus,
    pub linked_segment_ids: Vec<SegmentId>,
    pub alignment_id: Option<AlignmentId>,
    pub local_author_label: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AnnotationUpdateRequest {
    pub annotation_id: AnnotationId,
    pub title: String,
    pub body: String,
    pub status: AnnotationStatus,
    pub linked_segment_ids: Vec<SegmentId>,
    pub alignment_id: Option<AlignmentId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Txt,
    Json,
    Xml,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExportRequest {
    pub format: ExportFormat,
    pub output_path: String,
    #[serde(default = "default_true")]
    pub include_unlinked: bool,
    #[serde(default = "default_separator")]
    pub side_separator: String,
}

fn default_true() -> bool {
    true
}
fn default_separator() -> String {
    " ".into()
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CommandEnvelope<P> {
    pub contract_version: String,
    pub command_id: CommandId,
    pub project_id: ProjectId,
    pub base_revision_id: RevisionId,
    pub kind: CommandKind,
    pub payload: P,
}
impl<P> CommandEnvelope<P> {
    pub fn new(
        command_id: CommandId,
        project_id: ProjectId,
        base_revision_id: RevisionId,
        kind: CommandKind,
        payload: P,
    ) -> Self {
        Self {
            contract_version: CONTRACT_VERSION.into(),
            command_id,
            project_id,
            base_revision_id,
            kind,
            payload,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandKind {
    CreateProject,
    ApplySegmentation,
    UpdateSegment,
    MoveSegment,
    LinkSegments,
    UnlinkAlignment,
    /// Legacy wire name kept for reading old command logs. New commands use
    /// `GroupAlignment`.
    MergeAlignments,
    /// Legacy wire name kept for reading old command logs. New commands use
    /// `UngroupAlignment`.
    SplitAlignment,
    CreateAlignment,
    DeleteAlignment,
    /// Legacy wire name kept for reading old command logs.
    MergeAlignment,
    GroupAlignment,
    UngroupAlignment,
    MergeSegments,
    SplitSegment,
    CreateAnnotation,
    UpdateAnnotation,
    DeleteAnnotation,
    CreateBookmark,
    DeleteBookmark,
    SaveRevision,
    Undo,
    Redo,
    RestoreRevision,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ApplySegmentationPayload {
    pub document_id: DocumentId,
    pub encoding: Encoding,
    pub mode: SegmentationMode,
    pub profile: ImportProfile,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UpdateSegmentPayload {
    pub segment_id: SegmentId,
    pub content: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MergeSegmentsPayload {
    pub segment_ids: Vec<SegmentId>,
    pub merged_content: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SplitSegmentPayload {
    pub segment_id: SegmentId,
    /// Parts must be non-empty and concatenate losslessly to the current
    /// Segment content.
    pub parts: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MoveSegmentPayload {
    pub segment_id: SegmentId,
    pub before_segment_id: Option<SegmentId>,
    pub after_segment_id: Option<SegmentId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AlignmentSelectionPayload {
    pub source_segment_ids: Vec<SegmentId>,
    pub target_segment_ids: Vec<SegmentId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GroupAlignmentPayload {
    pub alignment_ids: Vec<AlignmentId>,
    #[serde(default)]
    pub unlinked_segment_ids: Vec<SegmentId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LinkSegmentsPayload {
    pub source_segment_ids: Vec<SegmentId>,
    pub target_segment_ids: Vec<SegmentId>,
    /// Existing active relations are only removed after an explicit user
    /// confirmation. Omitting this field is equivalent to `false`.
    #[serde(default)]
    pub replace_existing: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UngroupAlignmentPayload {
    pub alignment_id: AlignmentId,
    /// At least two groups are required and paired by index. Every group must
    /// be non-empty and the groups must partition the original references.
    pub source_groups: Vec<Vec<SegmentId>>,
    pub target_groups: Vec<Vec<SegmentId>>,
}

/// Compatibility DTO for callers serialized before Group/Ungroup terminology
/// was introduced. New command envelopes must use `UngroupAlignmentPayload`.
pub type SplitAlignmentPayload = UngroupAlignmentPayload;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AlignmentRefPayload {
    pub alignment_id: AlignmentId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ParallelSliceRequest {
    pub project_id: ProjectId,
    pub source_document_id: DocumentId,
    pub target_document_id: DocumentId,
    pub anchor_segment_id: Option<SegmentId>,
    pub anchor_alignment_id: Option<AlignmentId>,
    pub halo: u32,
    pub revision_id: RevisionId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SaveRevisionRequest {
    pub project_id: ProjectId,
    pub base_revision_id: RevisionId,
    pub author_label: String,
    pub summary: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum KernelEvent {
    RevisionAdvanced {
        revision_id: RevisionId,
    },
    SegmentChanged {
        segment_id: SegmentId,
        revision_id: RevisionId,
    },
    AlignmentChanged {
        alignment_id: AlignmentId,
        revision_id: RevisionId,
    },
    SaveStateChanged {
        dirty: bool,
        revision_id: RevisionId,
    },
    IndexUpdated {
        revision_id: RevisionId,
    },
}

pub use jueming_core::{
    Alignment, AlignmentProducer, Cardinality, ChangeSetSummary, ContentRef, DecodedText, Document,
    InitialLayout, PositionKey, Project, Revision, RevisionState, Segment, SegmentKind,
    SegmentOrder, SegmentPreview, SegmentationPreview,
};
pub use jueming_core::{
    CoreError, build_provisional_layout, decode_bytes, provisional_alignment_layout, segment_text,
    validate_alignment, validate_segment_order,
};

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn envelope_uses_contract_version_and_stable_ids() {
        let project = ProjectId::new();
        let envelope = CommandEnvelope::new(
            CommandId::new(),
            project,
            RevisionId::new(0),
            CommandKind::ApplySegmentation,
            ApplySegmentationPayload {
                document_id: DocumentId::new(),
                encoding: Encoding::Utf8,
                mode: SegmentationMode::NonEmptyLine,
                profile: ImportProfile::new(Encoding::Utf8, SegmentationMode::NonEmptyLine),
            },
        );
        let value = serde_json::to_value(&envelope).unwrap();
        assert_eq!(value["contract_version"], "1.0");
        assert!(value["command_id"].as_str().unwrap().contains('-'));
        assert_eq!(
            serde_json::from_value::<CommandEnvelope<ApplySegmentationPayload>>(value)
                .unwrap()
                .project_id,
            project
        );
    }

    #[test]
    fn language_catalogue_uses_ten_ltr_base_codes_and_reads_legacy_chinese() {
        assert_eq!(COMMON_LTR_LANGUAGES.len(), 10);
        assert_eq!(
            SupportedLanguageId::parse_compatible("zh-CN"),
            Some(SupportedLanguageId::Zh)
        );
        assert_eq!(SupportedLanguageId::parse_compatible("ar"), None);
        assert_eq!(SupportedLanguageId::Ru.as_str(), "ru");
    }

    #[test]
    fn bookmark_preview_serializes_as_a_flat_read_model() {
        let preview = BookmarkPreview {
            bookmark: Bookmark {
                bookmark_id: BookmarkId::new(),
                project_id: ProjectId::new(),
                segment_id: SegmentId::new(),
                alignment_id: Some(AlignmentId::new()),
                label: "Review this passage".into(),
                created_at: "2026-08-29T00:00:00Z".into(),
                updated_at: "2026-08-29T00:00:00Z".into(),
            },
            document_title: "Source".into(),
            language_id: "zh".into(),
            segment_content: "当前正文".into(),
            before_context: Some("上文".into()),
            after_context: Some("下文".into()),
        };

        let value = serde_json::to_value(preview).unwrap();
        assert!(value.get("bookmark").is_none());
        assert!(value["bookmark_id"].as_str().is_some());
        assert!(value["segment_id"].as_str().is_some());
        assert_eq!(value["segment_content"], "当前正文");
        assert_eq!(value["language_id"], "zh");
    }
}
