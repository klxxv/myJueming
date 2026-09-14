//! Versioned, serde-friendly DTOs for the local `KernelClient` boundary.
//!
//! The DTO layer reuses core's typed IDs and canonical objects; it does not
//! introduce a second representation of Segment or Alignment identity.

mod command;
mod event;
mod export;
mod history;
mod import;
mod languages;
mod project;
mod research;
mod search;
mod sidecar;
mod slice;
mod workspace;

pub use research::*;

pub const CONTRACT_VERSION: &str = "1.0";

// Preserve the established crate-root API. Internal DTO modules import core
// types directly, so these compatibility exports are not internal dependencies.
pub use jueming_core::{
    AlignmentId, AnnotationId, AssetId, BookmarkId, CommandId, DocumentId, Encoding, ImportProfile,
    OperationId, ProjectId, RevisionId, SegmentId, SegmentOrderId, SegmentationMode,
};

pub use jueming_core::{
    Alignment, AlignmentProducer, Cardinality, ChangeSetSummary, ContentRef, DecodedText, Document,
    InitialLayout, PositionKey, Project, Revision, RevisionState, Segment, SegmentKind,
    SegmentOrder, SegmentPreview, SegmentationPreview,
};
pub use jueming_core::{
    CoreError, build_provisional_layout, decode_bytes, provisional_alignment_layout, segment_text,
    validate_alignment, validate_segment_order,
};

pub use command::{
    AlignmentGapEdge, AlignmentRefPayload, AlignmentSelectionPayload, ApplySegmentationPayload,
    CommandEnvelope, CommandKind, CommandResult, GroupAlignmentPayload, LinkSegmentsPayload,
    MergeSegmentsPayload, MoveSegmentPayload, SplitAlignmentPayload, SplitSegmentPayload,
    UngroupAlignmentPayload, UpdateSegmentPayload,
};
pub use event::KernelEvent;
pub use export::{ExportFormat, ExportRequest};
pub use history::{
    AlignmentRevisionDiff, OrderRevisionDiff, RevisionComparison, RevisionListResponse,
    SaveRevisionRequest, SegmentRevisionDiff,
};
pub use import::{
    CreateProjectRequest, ImportPreviewResponse, ImportSideRequest, PreviewImportRequest,
    SourceAssetRecord, TextInput,
};
pub use languages::{COMMON_LTR_LANGUAGES, SupportedLanguage, SupportedLanguageId};
pub use project::{CommandReceipt, ProjectSnapshot, ProjectSummary};
pub use search::{
    ReplaceApplyRequest, ReplacePreviewItem, ReplacePreviewRequest, ReplacePreviewResponse,
    SearchSegmentsRequest, SearchSegmentsResponse, SegmentSearchHit,
};
pub use sidecar::{
    AnnotationCreateRequest, AnnotationStatus, AnnotationUpdateRequest, Bookmark,
    BookmarkCreateRequest, BookmarkPreview, BookmarkUpdateRequest, HumanAnnotation,
};
pub use slice::ParallelSliceRequest;
pub use workspace::{ParallelSlice, SegmentDescriptor, SegmentText, WorkspaceProject};
