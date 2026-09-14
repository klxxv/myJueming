//! Real LocalAppHost + jueming-pipeline gateway acceptance tests.
//!
//! Host owner registration is intentionally explicit because this shared test
//! lives outside the application crate's exclusive source tree. Add this line
//! to `crates/jueming-application/tests/host.rs` while integrating the gateway:
//! `#[path = "../../../tests/agent-pipeline/pipeline_gateway.rs"] mod pipeline_gateway;`

use jueming_application::{AgentCall, LocalAppHost};
use jueming_pipeline::{PipelineMethodRevisionId, PipelineService, UpdatePipelineMethod};
use jueming_protocol::{
    CreateProjectRequest, Encoding, ImportProfile, ImportSideRequest, SegmentationMode, TextInput,
};
use serde_json::{Map, Value, json};

fn pipeline_request(path: std::path::PathBuf) -> CreateProjectRequest {
    pipeline_request_with_source(path, "我来到北京清华大学".into())
}

fn pipeline_request_with_source(
    path: std::path::PathBuf,
    source_text: String,
) -> CreateProjectRequest {
    let profile = ImportProfile::new(Encoding::Utf8, SegmentationMode::NonEmptyLine);
    CreateProjectRequest {
        additional_targets: Vec::new(),
        project_path: path.to_string_lossy().into_owned(),
        name: "pipeline".into(),
        source: ImportSideRequest {
            expected_sha256: None,
            language_id: "zh-CN".into(),
            title: "source".into(),
            input: TextInput::Paste {
                label: "source".into(),
                text: source_text,
            },
            profile: profile.clone(),
        },
        target: ImportSideRequest {
            expected_sha256: None,
            language_id: "en".into(),
            title: "target".into(),
            input: TextInput::Paste {
                label: "target".into(),
                text: "I came to Tsinghua University in Beijing".into(),
            },
            profile,
        },
    }
}

fn object_params(value: Value) -> Map<String, Value> {
    value
        .as_object()
        .cloned()
        .expect("object request parameters")
}

fn call(id: &str, method: &str, params: Value, binding_id: Option<String>) -> AgentCall {
    AgentCall {
        request_id: id.into(),
        method: method.into(),
        params: object_params(params),
        binding_id,
    }
}

fn bind(host: &LocalAppHost) -> String {
    host.dispatch_native(call("bind", "app.bind_session", json!({}), None))
        .unwrap()
        .data["binding_id"]
        .as_str()
        .unwrap()
        .to_owned()
}

#[test]
fn pipeline_gateway_uses_host_snapshot_and_requires_native_mutation_authority() {
    let temp = tempfile::tempdir().unwrap();
    let host = LocalAppHost::new();
    let project_path = temp.path().join("gateway.jm");
    host.create_project(&pipeline_request(project_path.clone()))
        .unwrap();
    let binding = bind(&host);
    let external_create = host
        .dispatch(call(
            "external-create",
            "pipeline.create_default",
            json!({"name":"default"}),
            Some(binding.clone()),
        ))
        .unwrap_err();
    assert_eq!(external_create.code, "native_approval_required");

    let method = host
        .dispatch_native(call(
            "native-create",
            "pipeline.create_default",
            json!({"name":"中文分词"}),
            Some(binding.clone()),
        ))
        .unwrap()
        .data;
    let operators: Vec<_> = method["current"]["plan"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|node| node["operator"].as_str().unwrap())
        .collect();
    assert_eq!(
        operators,
        ["source", "normalize", "chinese_tokenize", "artifact"]
    );

    let request = json!({
        "method_id": method["method_id"],
        "base_method_revision_id": method["current"]["method_revision_id"],
        "name": "审校分词",
        "plan": method["current"]["plan"],
    });
    let proposal = host
        .dispatch(call(
            "preview",
            "pipeline.preview_update",
            json!({"request": request.clone()}),
            Some(binding.clone()),
        ))
        .unwrap()
        .data;
    let external_approval = host
        .dispatch(call(
            "external-approval",
            "pipeline.approve_update",
            json!({"proposal_id": proposal["proposal_id"]}),
            Some(binding.clone()),
        ))
        .unwrap_err();
    assert_eq!(external_approval.code, "native_approval_required");

    // Model the crash window after the reserved service commit but before the
    // journal finalization. A later ordinary edit must not hide the historical
    // reserved revision during recovery.
    let reserved: PipelineMethodRevisionId =
        serde_json::from_value(proposal["reserved_method_revision_id"].clone()).unwrap();
    let stored_request: UpdatePipelineMethod = serde_json::from_value(request.clone()).unwrap();
    let committed_before_finalization = PipelineService::open(&project_path)
        .unwrap()
        .update_method_with_revision_id(stored_request, reserved)
        .unwrap();
    let later_manual_update = host
        .dispatch_native(call(
            "later-manual-update",
            "pipeline.update",
            json!({"request": {
                "method_id": method["method_id"],
                "base_method_revision_id": reserved,
                "name": "later ordinary update",
                "plan": method["current"]["plan"],
            }}),
            Some(binding.clone()),
        ))
        .unwrap()
        .data;
    let approved = host
        .dispatch_native(call(
            "native-approval",
            "pipeline.approve_update",
            json!({"proposal_id": proposal["proposal_id"]}),
            Some(binding.clone()),
        ))
        .unwrap()
        .data;
    assert_eq!(approved["status"], "approved");
    assert_eq!(
        approved["approved_method_revision_id"],
        json!(committed_before_finalization.current.method_revision_id)
    );
    let retry = host
        .dispatch_native(call(
            "native-approval-retry",
            "pipeline.approve_update",
            json!({"proposal_id": proposal["proposal_id"]}),
            Some(binding.clone()),
        ))
        .unwrap()
        .data;
    assert_eq!(
        retry["approved_method_revision_id"],
        approved["approved_method_revision_id"]
    );
    let recovered_method = host
        .dispatch(call(
            "get-recovered-method",
            "pipeline.get",
            json!({"method_id": method["method_id"]}),
            Some(binding.clone()),
        ))
        .unwrap()
        .data;
    assert_eq!(recovered_method["revisions"].as_array().unwrap().len(), 3);
    assert_eq!(
        recovered_method["current"]["method_revision_id"],
        later_manual_update["current"]["method_revision_id"]
    );

    // A direct native update with an identical request must not cause the
    // still-unapproved proposal to be inferred as approved.
    let identical_request = json!({
        "method_id": method["method_id"],
        "base_method_revision_id": later_manual_update["current"]["method_revision_id"],
        "name": "identical manual update",
        "plan": later_manual_update["current"]["plan"],
    });
    let identical_proposal = host
        .dispatch(call(
            "preview-identical",
            "pipeline.preview_update",
            json!({"request": identical_request.clone()}),
            Some(binding.clone()),
        ))
        .unwrap()
        .data;
    host.dispatch_native(call(
        "manual-identical-update",
        "pipeline.update",
        json!({"request": identical_request}),
        Some(binding.clone()),
    ))
    .unwrap();
    let proposals = host
        .dispatch(call(
            "list-identical-proposals",
            "pipeline.list_proposals",
            json!({}),
            Some(binding.clone()),
        ))
        .unwrap()
        .data;
    let identical_after_manual = proposals
        .as_array()
        .unwrap()
        .iter()
        .find(|candidate| candidate["proposal_id"] == identical_proposal["proposal_id"])
        .unwrap();
    assert_eq!(identical_after_manual["status"], "pending");
    assert_eq!(
        host.dispatch_native(call(
            "approve-identical-stale-proposal",
            "pipeline.approve_update",
            json!({"proposal_id": identical_proposal["proposal_id"]}),
            Some(binding.clone()),
        ))
        .unwrap_err()
        .code,
        "stale_method_revision"
    );

    let source_before = host.current_snapshot().unwrap().segments[0].content.clone();
    let snapshot = host.current_snapshot().unwrap();
    let execution = host
        .dispatch(call(
            "execute",
            "pipeline.execute",
            json!({
                "method_id": method["method_id"],
                "segment_id": snapshot.segments[0].segment_id,
                "base_revision_id": snapshot.project.current_revision_id,
                "operation_id": "pipeline-gateway-test-operation",
                "source_content": "forged caller content must never be used",
            }),
            Some(binding.clone()),
        ))
        .unwrap()
        .data;
    assert_eq!(
        execution["artifact"]["input_revision_id"],
        json!(snapshot.project.current_revision_id.to_string())
    );
    assert_eq!(
        execution["artifact"]["normalized_content"],
        json!(source_before.clone())
    );
    assert_eq!(
        host.current_snapshot().unwrap().segments[0].content,
        source_before
    );

    let artifact = host
        .dispatch(call(
            "artifact",
            "pipeline.artifacts",
            json!({"artifact_id": execution["artifact"]["artifact_id"]}),
            Some(binding.clone()),
        ))
        .unwrap()
        .data;
    assert_eq!(
        artifact["artifact_id"],
        execution["artifact"]["artifact_id"]
    );
    let history = host
        .dispatch(call(
            "history-artifacts",
            "pipeline.artifacts",
            json!({"input_revision_id": snapshot.project.current_revision_id}),
            Some(binding),
        ))
        .unwrap()
        .data;
    assert_eq!(history.as_array().unwrap().len(), 1);
}

#[test]
fn pipeline_approval_requires_matching_canonical_and_method_revisions() {
    let temp = tempfile::tempdir().unwrap();
    let host = LocalAppHost::new();
    host.create_project(&pipeline_request(temp.path().join("proposal-versions.jm")))
        .unwrap();
    let binding = bind(&host);
    let method = host
        .dispatch_native(call(
            "create-method",
            "pipeline.create_default",
            json!({"name":"versioned"}),
            Some(binding.clone()),
        ))
        .unwrap()
        .data;

    let canonical_request = json!({
        "method_id": method["method_id"],
        "base_method_revision_id": method["current"]["method_revision_id"],
        "name": "canonical proposal",
        "plan": method["current"]["plan"],
    });
    let canonical_proposal = host
        .dispatch(call(
            "preview-canonical",
            "pipeline.preview_update",
            json!({"request": canonical_request}),
            Some(binding.clone()),
        ))
        .unwrap()
        .data;
    let segment = host.current_snapshot().unwrap().segments[0].segment_id;
    host.mutate("native", |kernel, snapshot, path| {
        kernel.update_segment(path, snapshot, segment, "canonical revision advanced")
    })
    .unwrap();
    assert_eq!(
        host.dispatch_native(call(
            "approve-stale-canonical",
            "pipeline.approve_update",
            json!({"proposal_id": canonical_proposal["proposal_id"]}),
            Some(binding.clone()),
        ))
        .unwrap_err()
        .code,
        "stale_revision"
    );

    let current = host
        .dispatch(call(
            "get-current",
            "pipeline.get",
            json!({"method_id": method["method_id"]}),
            Some(binding.clone()),
        ))
        .unwrap()
        .data;
    let proposal_request = json!({
        "method_id": current["method_id"],
        "base_method_revision_id": current["current"]["method_revision_id"],
        "name": "proposal name",
        "plan": current["current"]["plan"],
    });
    let method_proposal = host
        .dispatch(call(
            "preview-method",
            "pipeline.preview_update",
            json!({"request": proposal_request.clone()}),
            Some(binding.clone()),
        ))
        .unwrap()
        .data;
    let manual_request = json!({
        "method_id": current["method_id"],
        "base_method_revision_id": current["current"]["method_revision_id"],
        "name": "manual name",
        "plan": current["current"]["plan"],
    });
    host.dispatch_native(call(
        "manual-update",
        "pipeline.update",
        json!({"request": manual_request}),
        Some(binding.clone()),
    ))
    .unwrap();
    assert_eq!(
        host.dispatch_native(call(
            "approve-stale-method",
            "pipeline.approve_update",
            json!({"proposal_id": method_proposal["proposal_id"]}),
            Some(binding),
        ))
        .unwrap_err()
        .code,
        "stale_method_revision"
    );
}

#[test]
fn pipeline_cancel_reaches_the_active_project_operation_and_releases_it() {
    let temp = tempfile::tempdir().unwrap();
    let host = std::sync::Arc::new(LocalAppHost::new());
    host.create_project(&pipeline_request_with_source(
        temp.path().join("cancel.jm"),
        "我来到北京清华大学".repeat(100_000),
    ))
    .unwrap();
    let binding = bind(&host);
    let method = host
        .dispatch_native(call(
            "create-cancel-method",
            "pipeline.create_default",
            json!({"name":"cancel"}),
            Some(binding.clone()),
        ))
        .unwrap()
        .data;
    let snapshot = host.current_snapshot().unwrap();
    let operation_id = "shared-cancel-operation".to_owned();
    let runner = {
        let host = host.clone();
        let binding = binding.clone();
        let method_id = method["method_id"].clone();
        let segment_id = json!(snapshot.segments[0].segment_id);
        let revision_id = json!(snapshot.project.current_revision_id);
        let operation_id = operation_id.clone();
        std::thread::spawn(move || {
            host.dispatch(call(
                "long-running-execute",
                "pipeline.execute",
                json!({
                    "method_id": method_id,
                    "segment_id": segment_id,
                    "base_revision_id": revision_id,
                    "operation_id": operation_id,
                }),
                Some(binding),
            ))
        })
    };
    let mut cancel_sent = false;
    let other_binding = host
        .dispatch(call("other-bind", "app.bind_session", json!({}), None))
        .unwrap()
        .data["binding_id"]
        .as_str()
        .unwrap()
        .to_owned();
    for _ in 0..10_000 {
        match host.dispatch(call(
            "cross-binding-cancel",
            "pipeline.cancel",
            json!({"operation_id": operation_id}),
            Some(other_binding.clone()),
        )) {
            Err(error) if error.code == "operation_not_found" && !runner.is_finished() => {
                std::thread::yield_now();
                continue;
            }
            Err(error) => assert_eq!(error.code, "operation_not_owner"),
            Ok(_) => panic!("a non-owner binding cancelled the Pipeline operation"),
        }
        match host.dispatch(call(
            "cancel-active-run",
            "pipeline.cancel",
            json!({"operation_id": operation_id}),
            Some(binding.clone()),
        )) {
            Ok(response) => {
                assert_eq!(response.data["status"], "cancelling");
                cancel_sent = true;
                break;
            }
            Err(error) if error.code == "operation_not_found" && !runner.is_finished() => {
                std::thread::yield_now();
            }
            Err(error) => panic!("unexpected Pipeline cancellation result: {error}"),
        }
    }
    assert!(cancel_sent, "Pipeline operation did not become cancellable");
    assert_eq!(
        runner.join().unwrap().unwrap_err().code,
        "operation_cancelled"
    );

    // The run registry is reclaimed on every terminal result. The same key is
    // therefore accepted for a new project without cross-project collision.
    let second = LocalAppHost::new();
    second
        .create_project(&pipeline_request(temp.path().join("cancel-second.jm")))
        .unwrap();
    let second_binding = bind(&second);
    let second_method = second
        .dispatch_native(call(
            "create-second-method",
            "pipeline.create_default",
            json!({"name":"second"}),
            Some(second_binding.clone()),
        ))
        .unwrap()
        .data;
    let second_snapshot = second.current_snapshot().unwrap();
    let second_execution = second.dispatch(call(
        "second-project-same-operation-id",
        "pipeline.execute",
        json!({
            "method_id": second_method["method_id"],
            "segment_id": second_snapshot.segments[0].segment_id,
            "base_revision_id": second_snapshot.project.current_revision_id,
            "operation_id": operation_id,
        }),
        Some(second_binding),
    ));
    assert!(second_execution.is_ok());
}
