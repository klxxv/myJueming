//! Import DTOs for the versioned Kernel boundary.

use jueming_core::{AssetId, Encoding, EncodingDetection, ImportProfile, SegmentationPreview};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum TextInput {
    File { path: String },
    Paste { label: String, text: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PreviewImportRequest {
    #[serde(default)]
    pub auto_detect_encoding: bool,
    pub input: TextInput,
    pub profile: ImportProfile,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ImportPreviewResponse {
    #[serde(default)]
    pub encoding_detection: EncodingDetection,
    pub label: String,
    pub profile: ImportProfile,
    pub had_bom: bool,
    pub sha256: String,
    pub byte_length: u64,
    pub preview: SegmentationPreview,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ImportSideRequest {
    #[serde(default)]
    pub expected_sha256: Option<String>,
    pub language_id: String,
    pub title: String,
    pub input: TextInput,
    pub profile: ImportProfile,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CreateProjectRequest {
    #[serde(default)]
    pub additional_targets: Vec<ImportSideRequest>,
    pub project_path: String,
    pub name: String,
    pub source: ImportSideRequest,
    pub target: ImportSideRequest,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceAssetRecord {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub import_profile: Option<ImportProfile>,
    pub asset_id: AssetId,
    pub original_path: Option<String>,
    pub label: String,
    pub encoding: Encoding,
    pub sha256: String,
    pub byte_length: u64,
    pub imported_at: String,
}
