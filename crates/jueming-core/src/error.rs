//! Errors raised by deterministic domain and import validation.

use crate::import::Encoding;
use thiserror::Error;

#[derive(Debug, Error, Clone, Eq, PartialEq)]
pub enum CoreError {
    #[error("text is not valid {encoding:?}: {detail}")]
    Decode { encoding: Encoding, detail: String },
    #[error("GB18030 decoding is unavailable on this platform")]
    UnsupportedGb18030,
    #[error("alignment must contain at least one segment on each side")]
    EmptyAlignment,
    #[error("an alignment contains duplicate segment references")]
    DuplicateAlignmentReference,
    #[error("segment order has duplicate or missing segment references")]
    InvalidSegmentOrder,
    #[error("segment belongs to the wrong document")]
    WrongDocument,
    #[error("alignment contains a segment from the wrong project/document")]
    InvalidAlignmentReference,
    #[error("project contains duplicate or missing document references")]
    InvalidProject,
}
