//! Research contract v2; original-text coordinates and independent run identities.
use jueming_core::{ProjectId, RevisionId, SegmentId};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextRange {
    pub segment_id: SegmentId,
    pub start_utf8: u64,
    pub end_utf8: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResearchJudgement {
    pub kind: JudgementKind,
    pub group_name: String,
    pub strategy: String,
    pub target_ranges: Vec<TextRange>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JudgementKind {
    Translation,
    Omission,
    Paraphrase,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResearchRecord {
    pub record_id: String,
    pub run_id: String,
    pub occurrence_id: String,
    pub input_revision_id: RevisionId,
    pub source_ranges: Vec<TextRange>,
    pub content_hashes: std::collections::BTreeMap<SegmentId, String>,
    #[serde(default)]
    pub context_fingerprint: String,
    pub judgement: ResearchJudgement,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchGroupMergeRequest {
    pub run_id: String,
    pub from_group: String,
    pub to_group: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResearchSource {
    pub segment_id: SegmentId,
    pub text: String,
    pub ranges: Vec<Utf8Range>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Utf8Range {
    pub start_utf8: u64,
    pub end_utf8: u64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResearchTarget {
    pub segment_id: SegmentId,
    pub text: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TranslationCandidate {
    pub ranges: Vec<TextRange>,
    pub text: String,
    pub score: Option<f64>,
    pub score_kind: String,
    pub provider_id: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResearchOccurrence {
    pub occurrence_id: String,
    pub source: ResearchSource,
    pub targets: Vec<ResearchTarget>,
    pub candidates: Vec<TranslationCandidate>,
    #[serde(default)]
    pub alignment_coverage: Option<String>,
    pub status: String,
    pub judgement: Option<ResearchJudgement>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Queued,
    Running,
    Completed,
    Cancelled,
    Failed,
    Interrupted,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResearchRun {
    pub run_id: String,
    pub project_id: ProjectId,
    pub input_revision_id: RevisionId,
    pub status: RunStatus,
    pub error: Option<String>,
    pub total: u64,
    pub completed: u64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResearchResultPage {
    pub run: ResearchRun,
    pub items: Vec<ResearchOccurrence>,
    pub total: u64,
    pub next_cursor: Option<u64>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchQuery {
    pub query: String,
    pub fuzzy: bool,
    pub similarity_operator: String,
    #[serde(default)]
    pub minimum_similarity: Option<f64>,
    pub auto_locate: bool,
    #[serde(default)]
    pub max_gap: u32,
    #[serde(default)]
    pub method_id: Option<String>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FeatureReason {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FeatureSnapshot {
    pub feature_id: String,
    pub desired_enabled: bool,
    pub status: String,
    pub stage: Option<String>,
    pub generation: String,
    pub activation_id: Option<String>,
    pub completed_bytes: Option<String>,
    pub total_bytes: Option<String>,
    pub reason: Option<FeatureReason>,
    pub resources_ready: bool,
    pub worker_state: String,
    pub default_similarity: String,
    pub auto_locate: bool,
}
impl Default for FeatureSnapshot {
    fn default() -> Self {
        Self {
            feature_id: "translation_research".into(),
            desired_enabled: false,
            status: "disabled".into(),
            stage: None,
            generation: "0".into(),
            activation_id: None,
            completed_bytes: None,
            total_bytes: None,
            reason: None,
            resources_ready: false,
            worker_state: "stopped".into(),
            default_similarity: "fuzzy.edit_distance".into(),
            auto_locate: true,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SchemaRef {
    pub name: String,
    pub version: u32,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PortDescriptor {
    pub name: String,
    pub schemas: Vec<SchemaRef>,
    pub required: bool,
    pub multiple: bool,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OperatorDescriptor {
    pub operator_id: String,
    pub release: String,
    pub name: String,
    pub slots: Vec<String>,
    pub inputs: Vec<PortDescriptor>,
    pub outputs: Vec<PortDescriptor>,
    pub config_schema: Value,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SlotState {
    pub slot_id: String,
    pub name: String,
    pub state: String,
    pub reason: Option<String>,
    pub inputs: Vec<PortDescriptor>,
    pub outputs: Vec<PortDescriptor>,
    pub providers: Vec<String>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CapabilitySnapshot {
    pub contract_version: String,
    pub generation: String,
    pub features: Vec<FeatureSnapshot>,
    pub slots: Vec<SlotState>,
    pub operators: Vec<OperatorDescriptor>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanV2 {
    pub format_version: u32,
    pub nodes: Vec<PlanNodeV2>,
    pub outputs: Vec<PortRef>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanNodeV2 {
    pub node_id: String,
    pub slot_id: String,
    pub operator_id: String,
    pub inputs: std::collections::BTreeMap<String, Vec<PortRef>>,
    #[serde(default)]
    pub config: Value,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortRef {
    pub node_id: String,
    pub port: String,
}
