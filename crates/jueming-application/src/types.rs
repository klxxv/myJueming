use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub const APPLICATION_CONTRACT_VERSION: &str = "1.0";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AgentCall {
    pub request_id: String,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Map<String, Value>,
    #[serde(default)]
    pub binding_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AgentReply {
    pub request_id: String,
    pub data: Value,
    pub sequence: String,
}

#[derive(Clone, Debug, Error, Deserialize, Serialize)]
#[error("{code}: {message}")]
pub struct AppError {
    pub code: String,
    pub message: String,
}

impl AppError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new("invalid_request", message)
    }
    pub fn unavailable() -> Self {
        Self::new("project_unavailable", "No local project is open.")
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AppEvent {
    pub contract_version: String,
    pub sequence: String,
    pub kind: String,
    pub binding_id: Option<String>,
    pub origin: String,
    pub payload: Value,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct SearchSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_ids: Option<Vec<jueming_protocol::DocumentId>>,
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub regex: bool,
    #[serde(default)]
    pub case_sensitive: bool,
    #[serde(default)]
    pub language_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ContextSnapshot {
    pub tab: String,
    pub mode: Option<String>,
    pub project_id: Option<String>,
    pub revision_id: Option<String>,
    pub binding_id: Option<String>,
    pub window_focused: bool,
    pub focused_control: Option<String>,
    pub selected_text: String,
    pub segment_ids: Vec<String>,
    pub alignment_ids: Vec<String>,
    pub text_range: Option<TextRange>,
    pub captured_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TextRange {
    pub start_utf16: u64,
    pub end_utf16: u64,
}

impl Default for ContextSnapshot {
    fn default() -> Self {
        Self {
            tab: "review".into(),
            mode: None,
            project_id: None,
            revision_id: None,
            binding_id: None,
            window_focused: false,
            focused_control: None,
            selected_text: String::new(),
            segment_ids: Vec::new(),
            alignment_ids: Vec::new(),
            text_range: None,
            captured_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}
