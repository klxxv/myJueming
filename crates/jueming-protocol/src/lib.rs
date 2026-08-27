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
    CreateAlignment,
    DeleteAlignment,
    MergeAlignment,
    SplitAlignment,
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
pub struct AlignmentRefPayload {
    pub alignment_id: AlignmentId,
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
pub struct SearchSegmentsRequest {
    pub project_id: ProjectId,
    pub query: String,
    pub regex: bool,
    pub case_sensitive: bool,
    pub language_id: Option<String>,
    pub base_revision_id: RevisionId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SearchSegmentsResponse {
    pub revision_id: RevisionId,
    pub hits: Vec<SegmentSearchHit>,
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
