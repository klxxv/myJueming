//! Search DTOs for the versioned Kernel boundary.

use jueming_core::{AlignmentId, DocumentId, ProjectId, RevisionId, SegmentId};
use serde::{Deserialize, Serialize};

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
