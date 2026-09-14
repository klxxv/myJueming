//! Deterministic, non-canonical pipeline methods and derived token artifacts.
//!
//! This crate never opens or modifies canonical project snapshots. Callers must
//! provide revision-bound source content obtained through the Kernel boundary.

pub mod builtins;
mod error;
pub mod executor;
mod model;
pub mod plugin;
pub mod pool;
pub mod registry;
mod service;

pub use error::PipelineError;
pub use model::{
    CancellationToken, ChineseTokenizerConfig, CreatePipelineMethod, CustomDictionaryEntry,
    DerivedToken, ExecutePipelineRequest, NormalizeConfig, PipelineArtifactId,
    PipelineArtifactSummary, PipelineExecution, PipelineMethod, PipelineMethodId,
    PipelineMethodRevision, PipelineMethodRevisionId, PipelineMethodRevisionSummary,
    PipelineMethodSummary, PipelineNode, PipelineNodeId, PipelinePlanSnapshot, TokenArtifact,
    TokenizerProvenance, UpdatePipelineMethod,
};
pub use service::PipelineService;
