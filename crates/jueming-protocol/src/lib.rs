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
    MergeAlignments,
    SplitAlignment,
    CreateAlignment,
    DeleteAlignment,
    MergeAlignment,
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
pub struct LinkSegmentsPayload {
    pub source_segment_ids: Vec<SegmentId>,
    pub target_segment_ids: Vec<SegmentId>,
    /// Existing active relations are only removed after an explicit user
    /// confirmation. Omitting this field is equivalent to `false`.
    #[serde(default)]
    pub replace_existing: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SplitAlignmentPayload {
    pub alignment_id: AlignmentId,
    /// Groups are paired by index. Every group must be non-empty and the
    /// groups must partition the original alignment references.
    pub source_groups: Vec<Vec<SegmentId>>,
    pub target_groups: Vec<Vec<SegmentId>>,
}

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
}
