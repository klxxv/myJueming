//! Shared deterministic import fixture for Kernel API tests.

use jueming_core::{Encoding, ImportProfile, SegmentationMode};
use jueming_protocol::{CreateProjectRequest, ImportSideRequest, TextInput};
use std::path::Path;

pub fn government_request(project_path: &Path) -> CreateProjectRequest {
    CreateProjectRequest {
        additional_targets: Vec::new(),
        project_path: project_path.to_string_lossy().into_owned(),
        name: "2024 government report".into(),
        source: ImportSideRequest {
            expected_sha256: None,
            language_id: "zh-CN".into(),
            title: "中文（原文）".into(),
            input: TextInput::Paste {
                label: "report_zh.txt".into(),
                text: include_str!("../../../../tests/fixtures/government-report/report_zh.txt")
                    .into(),
            },
            profile: ImportProfile::new(Encoding::Utf8, SegmentationMode::NonEmptyLine),
        },
        target: ImportSideRequest {
            expected_sha256: None,
            language_id: "en".into(),
            title: "English（译文）".into(),
            input: TextInput::Paste {
                label: "report_en.txt".into(),
                text: include_str!("../../../../tests/fixtures/government-report/report_en.txt")
                    .into(),
            },
            profile: ImportProfile::new(Encoding::Utf8, SegmentationMode::NonEmptyLine),
        },
    }
}
