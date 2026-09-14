use crate::{PipelineArtifactId, PipelineMethodId, PipelineMethodRevisionId, PipelineNodeId};
use jueming_core::{RevisionId, SegmentId};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PipelineError {
    #[error("project directory must use the .jm extension: {0}")]
    InvalidProjectRoot(PathBuf),
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid pipeline JSON at {path}: {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("unsupported pipeline manifest format {0}")]
    UnsupportedManifest(String),
    #[error("pipeline method not found: {0}")]
    MethodNotFound(PipelineMethodId),
    #[error("pipeline method revision not found: {0}")]
    MethodRevisionNotFound(PipelineMethodRevisionId),
    #[error("pipeline method revision ID already exists: {0}")]
    MethodRevisionAlreadyExists(PipelineMethodRevisionId),
    #[error("pipeline artifact not found: {0}")]
    ArtifactNotFound(PipelineArtifactId),
    #[error(
        "method {method_id} changed; expected revision {expected}, current revision is {actual}"
    )]
    StaleMethodRevision {
        method_id: PipelineMethodId,
        expected: PipelineMethodRevisionId,
        actual: PipelineMethodRevisionId,
    },
    #[error("pipeline method name must not be blank")]
    BlankMethodName,
    #[error("pipeline graph has duplicate node ID {0}")]
    DuplicateNode(PipelineNodeId),
    #[error("pipeline {kind} ID must be UUIDv7: {value}")]
    InvalidUuidV7 { kind: &'static str, value: String },
    #[error("pipeline graph refers to unknown node {node_id} from {from_node_id}")]
    UnknownNode {
        node_id: PipelineNodeId,
        from_node_id: PipelineNodeId,
    },
    #[error("pipeline graph has an unknown operator {operator} at node {node_id}")]
    UnknownOperator {
        node_id: PipelineNodeId,
        operator: String,
    },
    #[error("pipeline node {node_id} has invalid inputs: expected {expected}, found {actual}")]
    InvalidInputCount {
        node_id: PipelineNodeId,
        expected: usize,
        actual: usize,
    },
    #[error(
        "pipeline slot mismatch from {from_node_id} to {node_id}: expected {expected}, found {actual}"
    )]
    SlotMismatch {
        node_id: PipelineNodeId,
        from_node_id: PipelineNodeId,
        expected: &'static str,
        actual: &'static str,
    },
    #[error("pipeline graph contains a cycle at node {0}")]
    Cycle(PipelineNodeId),
    #[error("pipeline output node {0} does not emit an artifact")]
    InvalidOutputNode(PipelineNodeId),
    #[error("invalid configuration for {operator} at node {node_id}: {message}")]
    InvalidOperatorConfig {
        node_id: PipelineNodeId,
        operator: String,
        message: String,
    },
    #[error("execution was cancelled")]
    Cancelled,
    #[error(
        "input content for segment {segment_id} at revision {revision_id} is invalid: {message}"
    )]
    InvalidInput {
        revision_id: RevisionId,
        segment_id: SegmentId,
        message: String,
    },
    #[error("artifact {artifact_id} is not declared by method revision {method_revision_id}")]
    ArtifactProvenanceMismatch {
        artifact_id: PipelineArtifactId,
        method_revision_id: PipelineMethodRevisionId,
    },
}

impl From<jueming_storage::StorageError> for PipelineError {
    fn from(value: jueming_storage::StorageError) -> Self {
        match value {
            jueming_storage::StorageError::ProjectLocked(path) => Self::Io {
                path,
                source: std::io::Error::other("project is locked"),
            },
            jueming_storage::StorageError::Io { path, source } => Self::Io { path, source },
            jueming_storage::StorageError::Json { path, source } => Self::Json { path, source },
            jueming_storage::StorageError::InvalidProjectRoot(path) => {
                Self::InvalidProjectRoot(path)
            }
            jueming_storage::StorageError::MissingParent(path) => Self::Io {
                path,
                source: std::io::Error::other("pipeline path has no parent"),
            },
        }
    }
}
