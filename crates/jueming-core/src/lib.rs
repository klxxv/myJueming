//! Canonical Jueming domain objects and deterministic import primitives.
//!
//! This crate deliberately has no filesystem, Tauri, or UI dependencies. It
//! owns semantic identity and the validations that protect it during reorder.

mod alignment;
mod error;
mod ids;
mod import;
mod order;
mod project;
mod revision;
mod segment;

pub use alignment::{
    Alignment, AlignmentProducer, Cardinality, InitialLayout, build_provisional_layout,
    provisional_alignment_layout, validate_alignment,
};
pub use error::CoreError;
pub use ids::{
    AlignmentId, AnnotationId, AssetId, BookmarkId, CommandId, DocumentId, OperationId, ProjectId,
    RevisionId, SegmentId, SegmentOrderId,
};
pub use import::{
    DecodedText, Encoding, EncodingDetection, ImportCleanup, ImportProfile, SegmentBoundary,
    SegmentPreview, SegmentationMode, SegmentationPreview, decode_bytes, detect_encoding,
    segment_text,
};
pub use order::{OrderedSegmentRef, PositionKey, SegmentOrder, validate_segment_order};
pub use project::{ComparisonSet, Document, LanguageId, Project, Timestamp, validate_project};
pub use revision::{ChangeSetSummary, Revision, RevisionState};
pub use segment::{ContentRef, Segment, SegmentKind};
