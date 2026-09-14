//! Command DTOs for the versioned Kernel boundary.

use crate::CONTRACT_VERSION;
use jueming_core::{
    AlignmentId, CommandId, DocumentId, Encoding, ImportProfile, ProjectId, RevisionId, SegmentId,
    SegmentationMode,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlignmentGapEdge {
    Before,
    After,
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
    ReorderSegments,
    InsertAlignmentGap,
    ApplyReplace,
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
    UpdateBookmark,
    DeleteBookmark,
    ResolveAnnotation,
    ConfirmResearch,
    MergeResearchGroups,
    SaveRevision,
    Undo,
    Redo,
    RestoreRevision,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CommandResult {
    pub command_id: CommandId,
    pub project_id: ProjectId,
    pub committed_revision_id: RevisionId,
    pub status: String,
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
