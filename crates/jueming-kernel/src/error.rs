//! Kernel command/query errors, independent of transport.

use jueming_core::{CoreError, RevisionId, SegmentId};
use jueming_storage::StorageError;
use std::io;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum KernelError {
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("both sides must produce at least one segment")]
    EmptyImport,
    #[error("pasted Unicode text must use UTF-8")]
    InvalidPasteEncoding,
    #[error("invalid project snapshot: {0}")]
    InvalidSnapshot(String),
    #[error("segment {0} belongs to more than one active alignment")]
    DuplicateActiveAlignment(jueming_core::SegmentId),
    #[error("segment {0} does not exist in the current project")]
    SegmentNotFound(SegmentId),
    #[error("segment move anchor is not valid for this document")]
    InvalidMoveAnchor,
    #[error("alignment {0} does not exist in the current project")]
    AlignmentNotFound(jueming_core::AlignmentId),
    #[error("an alignment selection must contain unique, non-empty source and target segments")]
    InvalidAlignmentSelection,
    #[error(
        "one or more selected segments are already aligned; confirm replacement before linking"
    )]
    AlignmentSelectionConflict,
    #[error("segment {0} is on the wrong alignment side")]
    WrongAlignmentSide(SegmentId),
    #[error("language {0} is not in the supported left-to-right language catalogue")]
    UnsupportedLanguage(String),
    #[error("segment merge requires at least two distinct segments")]
    MergeSegmentsRequiresMultiple,
    #[error("segments can only be merged within one document")]
    MergeSegmentsDifferentDocuments,
    #[error("segments can only be merged when they are consecutive in SegmentOrder")]
    MergeSegmentsNotConsecutive,
    #[error("segments must all be unlinked or all belong to one active alignment before merging")]
    MergeSegmentsAlignmentConflict,
    #[error("a segment split requires two or more non-empty parts")]
    InvalidSplitParts,
    #[error("split parts must concatenate exactly to the original segment content")]
    SplitContentMismatch,
    #[error("merge requires at least two distinct alignments or unlinked segments")]
    MergeRequiresMultiple,
    #[error("segment {0} is already part of an active alignment and cannot be merged as unlinked")]
    MergeSegmentAlreadyAligned(SegmentId),
    #[error("segment {0} must belong to an active alignment before a gap can be inserted")]
    AlignmentGapRequiresLinkedSegment(SegmentId),
    #[error("an alignment gap requires at least one later source/target pair")]
    AlignmentGapOutOfRange,
    #[error(
        "ungroup requires at least two non-empty paired groups that partition the original alignment"
    )]
    SplitGroupsMismatch,
    #[error("revision {0} is not available")]
    RevisionNotFound(RevisionId),
    #[error("revision belongs to a different project")]
    RevisionProjectMismatch,
    #[error("no earlier revision is available for undo")]
    NoUndoAvailable,
    #[error("no redo branch is available from the current revision")]
    NoRedoAvailable,
    #[error("project ID does not match the current project")]
    ProjectMismatch,
    #[error("stale revision: expected {expected}, provided {provided}")]
    StaleRevision {
        expected: RevisionId,
        provided: RevisionId,
    },
    #[error("invalid search regular expression: {0}")]
    InvalidSearchRegex(String),
    #[error("replace selection does not belong to the preview")]
    ReplaceSelectionMismatch,
    #[error("bookmark {0} does not exist")]
    BookmarkNotFound(jueming_core::BookmarkId),
    #[error("annotation {0} does not exist")]
    AnnotationNotFound(jueming_core::AnnotationId),
    #[error("annotation links must contain unique existing segments")]
    InvalidAnnotationLinks,
    #[error("alignment anchor does not include the bookmarked/annotated segment")]
    AnchorMismatch,
    #[error("export JSON failed: {0}")]
    ExportJson(String),
}
