//! Read-only workspace structure. Text and storage references are excluded.
use crate::{Bookmark, HumanAnnotation, ProjectSummary};
use jueming_core::{
    Alignment, Document, DocumentId, Project, ProjectId, Revision, RevisionId, SegmentId,
    SegmentOrder,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SegmentDescriptor {
    pub segment_id: SegmentId,
    pub document_id: DocumentId,
    pub content_hash: String,
    pub content_length: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkspaceProject {
    pub contract_version: String,
    pub project: Project,
    pub documents: Vec<Document>,
    pub segments: Vec<SegmentDescriptor>,
    pub segment_orders: Vec<SegmentOrder>,
    pub alignments: Vec<Alignment>,
    pub revisions: Vec<Revision>,
    pub bookmarks: Vec<Bookmark>,
    pub annotations: Vec<HumanAnnotation>,
    pub summary: ProjectSummary,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SegmentText {
    pub segment_id: SegmentId,
    pub content: String,
    pub content_hash: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParallelSlice {
    pub project_id: ProjectId,
    pub revision_id: RevisionId,
    pub segments: Vec<SegmentText>,
}
