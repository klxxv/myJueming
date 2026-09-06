//! Sidecar DTOs for the versioned Kernel boundary.

use jueming_core::{AlignmentId, AnnotationId, BookmarkId, ProjectId, SegmentId};
use serde::{Deserialize, Serialize};

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
