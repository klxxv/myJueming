//! History DTOs for the versioned Kernel boundary.

use jueming_core::{
    Alignment, AlignmentId, DocumentId, ProjectId, Revision, RevisionId, Segment, SegmentId,
    SegmentOrder,
};
use serde::{Deserialize, Serialize};

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
pub struct SaveRevisionRequest {
    pub project_id: ProjectId,
    pub base_revision_id: RevisionId,
    pub author_label: String,
    pub summary: String,
}
