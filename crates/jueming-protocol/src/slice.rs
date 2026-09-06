//! Slice DTOs for the versioned Kernel boundary.

use jueming_core::{AlignmentId, DocumentId, ProjectId, RevisionId, SegmentId};
use serde::{Deserialize, Serialize};

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
