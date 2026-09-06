use jueming_core::{RevisionId, SegmentId};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use uuid::Uuid;

macro_rules! pipeline_id {
    ($name:ident) => {
        #[derive(
            Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }
            pub fn as_uuid(self) -> Uuid {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

pipeline_id!(PipelineMethodId);
pipeline_id!(PipelineMethodRevisionId);
pipeline_id!(PipelineNodeId);
pipeline_id!(PipelineArtifactId);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PipelineMethodSummary {
    pub method_id: PipelineMethodId,
    pub name: String,
    pub current_method_revision_id: PipelineMethodRevisionId,
    pub revision_count: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PipelineMethod {
    pub method_id: PipelineMethodId,
    pub name: String,
    pub current: PipelineMethodRevision,
    pub revisions: Vec<PipelineMethodRevisionSummary>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PipelineMethodRevisionSummary {
    pub method_revision_id: PipelineMethodRevisionId,
    pub parent_method_revision_id: Option<PipelineMethodRevisionId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PipelineMethodRevision {
    pub method_id: PipelineMethodId,
    pub method_revision_id: PipelineMethodRevisionId,
    pub parent_method_revision_id: Option<PipelineMethodRevisionId>,
    /// The method name at this immutable revision.  Keeping it with the plan
    /// makes an explicitly reserved update independently verifiable after a
    /// later method revision changes the manifest's current name.
    #[serde(default)]
    pub method_name: String,
    pub plan: PipelinePlanSnapshot,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PipelinePlanSnapshot {
    pub method_id: PipelineMethodId,
    pub method_revision_id: PipelineMethodRevisionId,
    pub nodes: Vec<PipelineNode>,
    pub output_node_id: PipelineNodeId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PipelineNode {
    pub node_id: PipelineNodeId,
    /// Closed operator vocabulary. Kept as a string so a persisted unknown value
    /// can receive a precise validation error instead of silently deserializing.
    pub operator: String,
    #[serde(default)]
    pub inputs: Vec<PipelineNodeId>,
    #[serde(default)]
    pub config: Value,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CreatePipelineMethod {
    pub name: String,
    pub plan: PipelinePlanSnapshot,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct UpdatePipelineMethod {
    pub method_id: PipelineMethodId,
    pub base_method_revision_id: PipelineMethodRevisionId,
    pub name: Option<String>,
    pub plan: PipelinePlanSnapshot,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExecutePipelineRequest {
    pub method_id: PipelineMethodId,
    pub method_revision_id: Option<PipelineMethodRevisionId>,
    pub input_revision_id: RevisionId,
    pub segment_id: SegmentId,
    pub source_content: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PipelineExecution {
    pub artifact: TokenArtifact,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PipelineArtifactSummary {
    pub artifact_id: PipelineArtifactId,
    pub input_revision_id: RevisionId,
    pub segment_id: SegmentId,
    pub method_id: PipelineMethodId,
    pub method_revision_id: PipelineMethodRevisionId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TokenArtifact {
    pub artifact_id: PipelineArtifactId,
    pub artifact_format: String,
    pub input_revision_id: RevisionId,
    pub segment_id: SegmentId,
    pub source_content_sha256: String,
    /// Derived text after the explicit normalize node. Token offsets refer here.
    pub normalized_content: String,
    pub normalized_content_sha256: String,
    pub method_id: PipelineMethodId,
    pub method_revision_id: PipelineMethodRevisionId,
    pub tokenizer: TokenizerProvenance,
    pub tokens: Vec<DerivedToken>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TokenizerProvenance {
    pub implementation: String,
    pub version: String,
    pub hmm: bool,
    pub custom_dictionary_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DerivedToken {
    pub text: String,
    pub start_utf8: u64,
    pub end_utf8: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct NormalizeConfig {
    #[serde(default)]
    pub trim: bool,
    #[serde(default)]
    pub collapse_whitespace: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ChineseTokenizerConfig {
    #[serde(default = "default_hmm")]
    pub hmm: bool,
    #[serde(default)]
    pub custom_dictionary: Vec<CustomDictionaryEntry>,
}

fn default_hmm() -> bool {
    true
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CustomDictionaryEntry {
    pub word: String,
    pub frequency: Option<usize>,
    pub tag: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}
