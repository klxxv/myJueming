use jueming_application::{AgentCall, LocalAppHost};
use jueming_protocol::{
    CreateProjectRequest, Encoding, ImportProfile, ImportSideRequest, SegmentationMode, TextInput,
};
use serde_json::{Map, Value, json};

#[path = "../../../tests/agent-pipeline/pipeline_gateway.rs"]
mod pipeline_gateway;

fn request(
    path: std::path::PathBuf,
    name: &str,
    source: &str,
    target: &str,
) -> CreateProjectRequest {
    let profile = ImportProfile::new(Encoding::Utf8, SegmentationMode::NonEmptyLine);
    CreateProjectRequest {
        additional_targets: Vec::new(),
        project_path: path.to_string_lossy().into_owned(),
        name: name.into(),
        source: ImportSideRequest {
            expected_sha256: None,
            language_id: "zh-CN".into(),
            title: "source".into(),
            input: TextInput::Paste {
                label: "source".into(),
                text: source.into(),
            },
            profile: profile.clone(),
        },
        target: ImportSideRequest {
            expected_sha256: None,
            language_id: "en".into(),
            title: "target".into(),
            input: TextInput::Paste {
                label: "target".into(),
                text: target.into(),
            },
            profile,
        },
    }
}

fn object_params(value: Value) -> Map<String, Value> {
    value.as_object().cloned().expect("object")
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
fn project_switch_invalidates_bindings_and_projection_echoes_binding() {
    let temp = tempfile::tempdir().unwrap();
    let host = LocalAppHost::new();
    host.create_project(&request(temp.path().join("one.jm"), "one", "甲", "a"))
        .unwrap();
    let binding = bind(&host);
    let projection = host
        .dispatch_native(call(
            "projection",
            "app.get_projection",
            json!({}),
            Some(binding.clone()),
        ))
        .unwrap();
    assert_eq!(projection.data["binding_id"], binding);
    host.create_project(&request(temp.path().join("two.jm"), "two", "乙", "b"))
        .unwrap();
    let error = host
        .dispatch(call(
            "summary",
            "project.get_summary",
            json!({}),
            Some(binding),
        ))
        .unwrap_err();
    assert_eq!(error.code, "stale_binding");
}

#[test]
fn external_dispatch_cannot_publish_native_context_or_read_native_projection() {
    let temp = tempfile::tempdir().unwrap();
    let host = LocalAppHost::new();
    host.create_project(&request(
        temp.path().join("native-context.jm"),
        "native-context",
        "甲",
        "a",
    ))
    .unwrap();
    let binding = bind(&host);
    let context = json!({"tab":"review", "mode":null, "project_id":null, "revision_id":null, "binding_id":null, "window_focused":true, "focused_control":null, "selected_text":"", "segment_ids":[], "alignment_ids":[], "text_range":null, "captured_at":"ignored"});
    assert_eq!(
        host.dispatch(call(
            "context",
            "ui.publish_context",
            context,
            Some(binding.clone())
        ))
        .unwrap_err()
        .code,
        "native_ui_required"
    );
    assert_eq!(
        host.dispatch(call(
            "projection-external",
            "app.get_projection",
            json!({}),
            Some(binding)
        ))
        .unwrap_err()
        .code,
        "native_ui_required"
    );
}

#[test]
fn loading_state_keeps_only_null_project_context_and_project_settings_navigation() {
    let host = LocalAppHost::new();
    let context = json!({"tab":"project", "mode":null, "project_id":null, "revision_id":null, "binding_id":null, "window_focused":true, "focused_control":"open", "selected_text":"", "segment_ids":[], "alignment_ids":[], "text_range":null, "captured_at":"ignored"});
    host.dispatch_native(call("loading-context", "ui.publish_context", context, None))
        .unwrap();
    let read = host
        .dispatch(call("read-loading", "ui.get_context", json!({}), None))
        .unwrap();
    assert!(read.data["project_id"].is_null());
    let navigation = host
        .dispatch(call(
            "loading-settings",
            "ui.navigate",
            json!({"tab":"settings"}),
            None,
        ))
        .unwrap();
    assert_eq!(navigation.data["status"], "accepted");
    assert_eq!(
        host.dispatch(call(
            "loading-review",
            "ui.navigate",
            json!({"tab":"review"}),
            None
        ))
        .unwrap_err()
        .code,
        "project_unavailable"
    );
}

#[test]
fn search_is_real_paginated_and_regex_is_validated_by_rust() {
    let temp = tempfile::tempdir().unwrap();
    let host = LocalAppHost::new();
    host.create_project(&request(
        temp.path().join("search.jm"),
        "search",
        "甲\n乙\n丙",
        "one\ntwo\nthree",
    ))
    .unwrap();
    let binding = bind(&host);
    host.dispatch_native(call(
        "spec",
        "search.set_spec",
        json!({"query":"[甲乙]", "regex":true, "case_sensitive":true, "language_id":null}),
        Some(binding.clone()),
    ))
    .unwrap();
    let first = host
        .dispatch(call(
            "execute",
            "search.execute",
            json!({"page_size":1}),
            Some(binding.clone()),
        ))
        .unwrap();
    assert_eq!(first.data["hits"].as_array().unwrap().len(), 1);
    let session = first.data["session_id"].as_str().unwrap();
    let next = host
        .dispatch(call(
            "next",
            "search.get_results",
            json!({"session_id":session, "cursor":1, "page_size":1}),
            Some(binding.clone()),
        ))
        .unwrap();
    assert_eq!(next.data["hits"].as_array().unwrap().len(), 1);
    let invalid = host
        .dispatch(call(
            "regex",
            "search.validate_regex",
            json!({"query":"(", "regex":true}),
            Some(binding),
        ))
        .unwrap_err();
    assert_eq!(invalid.code, "invalid_regex");
}

#[test]
fn replace_requires_native_approval_and_replays_without_new_revision() {
    let temp = tempfile::tempdir().unwrap();
    let host = LocalAppHost::new();
    host.create_project(&request(
        temp.path().join("replace.jm"),
        "replace",
        "old\nold",
        "old",
    ))
    .unwrap();
    let binding = bind(&host);
    let proposal = host
        .dispatch(call(
            "preview",
            "proposal.preview_replace",
            json!({"query":"old", "replacement":"new"}),
            Some(binding.clone()),
        ))
        .unwrap();
    assert_eq!(
        proposal.data["preview"]["items"].as_array().unwrap().len(),
        3
    );
    let proposal_id = proposal.data["proposal_id"].as_str().unwrap();
    let external = host
        .dispatch(call(
            "external-approve",
            "proposal.approve",
            json!({"proposal_id":proposal_id}),
            Some(binding.clone()),
        ))
        .unwrap_err();
    assert_eq!(external.code, "native_approval_required");
    let approved = host
        .dispatch_native(call(
            "approve-once",
            "proposal.approve",
            json!({"proposal_id":proposal_id}),
            Some(binding.clone()),
        ))
        .unwrap();
    let revision = approved.data["applied_revision_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let replay = host
        .dispatch_native(call(
            "approve-once",
            "proposal.approve",
            json!({"proposal_id":proposal_id}),
            Some(binding),
        ))
        .unwrap();
    assert_eq!(replay.data["applied_revision_id"], revision);
    assert_eq!(
        host.current_snapshot()
            .unwrap()
            .project
            .current_revision_id
            .to_string(),
        revision
    );
}

#[test]
fn stale_replace_proposal_cannot_apply_after_a_native_write() {
    let temp = tempfile::tempdir().unwrap();
    let host = LocalAppHost::new();
    host.create_project(&request(
        temp.path().join("stale.jm"),
        "stale",
        "old",
        "old",
    ))
    .unwrap();
    let binding = bind(&host);
    let proposal = host
        .dispatch(call(
            "preview-stale",
            "proposal.preview_replace",
            json!({"query":"old", "replacement":"new"}),
            Some(binding.clone()),
        ))
        .unwrap();
    let segment = host.current_snapshot().unwrap().segments[0].segment_id;
    host.mutate("native", |kernel, snapshot, path| {
        kernel.update_segment(path, snapshot, segment, "changed")
    })
    .unwrap();
    let error = host
        .dispatch_native(call(
            "approve-stale",
            "proposal.approve",
            json!({"proposal_id":proposal.data["proposal_id"]}),
            Some(binding),
        ))
        .unwrap_err();
    assert_eq!(error.code, "stale_revision");
}

#[test]
fn navigation_is_acknowledged_by_exact_operation_and_request_and_retries_are_fingerprinted() {
    let temp = tempfile::tempdir().unwrap();
    let host = LocalAppHost::new();
    host.create_project(&request(
        temp.path().join("navigation.jm"),
        "navigation",
        "甲",
        "a",
    ))
    .unwrap();
    let binding = bind(&host);
    let action = host
        .dispatch(call(
            "navigate",
            "ui.navigate",
            json!({"tab":"history"}),
            Some(binding.clone()),
        ))
        .unwrap();
    let operation_id = action.data["operation_id"].as_str().unwrap().to_owned();
    assert_eq!(action.data["payload"]["request_id"], "navigate");
    let conflict = host
        .dispatch(call(
            "navigate",
            "ui.navigate",
            json!({"tab":"review"}),
            Some(binding.clone()),
        ))
        .unwrap_err();
    assert_eq!(conflict.code, "request_id_conflict");
    let mismatch = host
        .dispatch_native(call(
            "ack-wrong",
            "ui.ack",
            json!({"operation_id":operation_id, "request_id":"wrong"}),
            Some(binding.clone()),
        ))
        .unwrap_err();
    assert_eq!(mismatch.code, "ack_mismatch");
    let ack = host
        .dispatch_native(call(
            "ack",
            "ui.ack",
            json!({"operation_id":operation_id, "request_id":"navigate", "status":"ui_applied"}),
            Some(binding.clone()),
        ))
        .unwrap();
    assert_eq!(ack.data["operation"]["status"], "ui_applied");
    let status = host
        .dispatch(call(
            "status",
            "operation.get",
            json!({"operation_id":operation_id}),
            Some(binding.clone()),
        ))
        .unwrap();
    assert_eq!(status.data["status"], "ui_applied");
    assert_eq!(
        host.dispatch(call(
            "missing",
            "operation.get",
            json!({"operation_id":"missing"}),
            Some(binding)
        ))
        .unwrap_err()
        .code,
        "operation_not_found"
    );
}

#[test]
fn native_binding_can_ack_external_navigation_only_in_the_same_project_epoch() {
    let temp = tempfile::tempdir().unwrap();
    let host = LocalAppHost::new();
    host.create_project(&request(
        temp.path().join("two-clients.jm"),
        "two-clients",
        "甲",
        "a",
    ))
    .unwrap();
    let external_binding = host
        .dispatch(call("external-bind", "app.bind_session", json!({}), None))
        .unwrap()
        .data["binding_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let native_binding = bind(&host);
    let action = host
        .dispatch(call(
            "external-navigate",
            "ui.navigate",
            json!({"tab":"history"}),
            Some(external_binding.clone()),
        ))
        .unwrap();
    let operation_id = action.data["operation_id"].as_str().unwrap().to_owned();
    assert_eq!(
        host.dispatch_native(call(
            "native-read-external",
            "operation.get",
            json!({"operation_id":operation_id}),
            Some(native_binding.clone())
        ))
        .unwrap_err()
        .code,
        "operation_forbidden"
    );
    assert_eq!(
        host.dispatch(call(
            "external-read-own",
            "operation.get",
            json!({"operation_id":operation_id}),
            Some(external_binding.clone())
        ))
        .unwrap()
        .data["status"],
        "accepted"
    );
    host.dispatch_native(call(
        "native-ack",
        "ui.ack",
        json!({"operation_id":operation_id, "request_id":"external-navigate"}),
        Some(native_binding.clone()),
    ))
    .unwrap();
    assert_eq!(
        host.dispatch_native(call(
            "late-ack",
            "ui.ack",
            json!({"operation_id":operation_id, "request_id":"external-navigate"}),
            Some(native_binding.clone())
        ))
        .unwrap_err()
        .code,
        "operation_terminal"
    );

    let external_binding = host
        .dispatch(call("external-bind-2", "app.bind_session", json!({}), None))
        .unwrap()
        .data["binding_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let stale = host
        .dispatch(call(
            "external-navigate-2",
            "ui.navigate",
            json!({"tab":"history"}),
            Some(external_binding),
        ))
        .unwrap();
    let stale_operation = stale.data["operation_id"].as_str().unwrap().to_owned();
    let segment = host.current_snapshot().unwrap().segments[0].segment_id;
    host.mutate("native", |kernel, snapshot, path| {
        kernel.update_segment(path, snapshot, segment, "changed")
    })
    .unwrap();
    assert_eq!(
        host.dispatch_native(call(
            "stale-epoch-ack",
            "ui.ack",
            json!({"operation_id":stale_operation, "request_id":"external-navigate-2"}),
            Some(native_binding)
        ))
        .unwrap_err()
        .code,
        "stale_operation_epoch"
    );
}

#[test]
fn execute_search_accepts_atomic_spec_and_expected_revision() {
    let temp = tempfile::tempdir().unwrap();
    let host = LocalAppHost::new();
    host.create_project(&request(
        temp.path().join("atomic-search.jm"),
        "atomic-search",
        "甲\n乙",
        "one\ntwo",
    ))
    .unwrap();
    let binding = bind(&host);
    let revision = host
        .current_snapshot()
        .unwrap()
        .project
        .current_revision_id
        .to_string();
    let result = host.dispatch(call("atomic-search", "search.execute", json!({"spec":{"query":"甲", "regex":false, "case_sensitive":true, "language_id":null}, "expected_revision_id":revision, "page_size":1}), Some(binding.clone()))).unwrap();
    assert_eq!(result.data["hits"].as_array().unwrap().len(), 1);
    assert_eq!(host.dispatch(call("stale-search", "search.execute", json!({"spec":{"query":"甲", "regex":false, "case_sensitive":true, "language_id":null}, "expected_revision_id":"999", "page_size":1}), Some(binding))).unwrap_err().code, "stale_revision");
}

#[test]
fn writer_lock_rejects_second_host_and_is_released_on_project_switch_and_drop() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("exclusive.jm");
    let host = LocalAppHost::new();
    let initial = host
        .create_project(&request(path.clone(), "exclusive", "甲", "a"))
        .unwrap();
    let second = LocalAppHost::new();
    assert_eq!(
        second.open_project(&path).unwrap_err().code,
        "project_locked"
    );
    assert_eq!(host.open_project(&path).unwrap(), initial);
    assert_eq!(
        host.create_project(&request(path.clone(), "overwrite", "乙", "b"))
            .unwrap_err()
            .code,
        "project_exists"
    );
    assert_eq!(host.current_snapshot().unwrap(), initial);
    host.create_project(&request(temp.path().join("other.jm"), "other", "乙", "b"))
        .unwrap();
    assert_eq!(second.open_project(&path).unwrap(), initial);
    drop(second);
    assert_eq!(LocalAppHost::new().open_project(&path).unwrap(), initial);
}

#[test]
fn native_commands_validate_scope_and_replay_after_restart_and_restore() {
    use jueming_protocol::{CommandEnvelope, CommandId, CommandKind, ProjectId, RevisionId};
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("commands.jm");
    let host = LocalAppHost::new();
    let initial = host
        .create_project(&request(path.clone(), "commands", "甲", "a"))
        .unwrap();
    let id = initial.segments[0].segment_id;
    let command = CommandEnvelope::new(
        CommandId::new(),
        initial.project.project_id,
        RevisionId::new(1),
        CommandKind::UpdateSegment,
        json!({"segment_id": id, "content": "changed"}),
    );
    let result = host.execute_native_command(command.clone()).unwrap();
    assert_eq!(result.committed_revision_id, RevisionId::new(2));
    assert_eq!(
        host.execute_native_command(command.clone()).unwrap().status,
        "duplicate"
    );
    let mut conflicting = command.clone();
    conflicting.payload["content"] = json!("different");
    assert_eq!(
        host.execute_native_command(conflicting).unwrap_err().code,
        "command_id_conflict"
    );
    let mut stale = command.clone();
    stale.command_id = CommandId::new();
    assert_eq!(
        host.execute_native_command(stale.clone()).unwrap_err().code,
        "stale_revision"
    );
    stale.project_id = ProjectId::new();
    assert_eq!(
        host.execute_native_command(stale).unwrap_err().code,
        "project_mismatch"
    );
    let revision_bytes = std::fs::read(path.join("revisions/2.json")).unwrap();
    let restore = CommandEnvelope::new(
        CommandId::new(),
        initial.project.project_id,
        RevisionId::new(2),
        CommandKind::RestoreRevision,
        json!({"target_revision_id": "1"}),
    );
    host.execute_native_command(restore.clone()).unwrap();
    assert_eq!(
        host.current_snapshot()
            .unwrap()
            .revisions
            .last()
            .unwrap()
            .parent_revision_id,
        Some(RevisionId::new(2))
    );
    drop(host);
    let restarted = LocalAppHost::new();
    restarted.open_project(&path).unwrap();
    assert_eq!(
        restarted
            .execute_native_command(command)
            .unwrap()
            .committed_revision_id,
        RevisionId::new(2)
    );
    assert_eq!(
        restarted.execute_native_command(restore).unwrap().status,
        "duplicate"
    );
    assert_eq!(
        restarted
            .current_snapshot()
            .unwrap()
            .project
            .current_revision_id,
        RevisionId::new(3)
    );
    assert_eq!(
        std::fs::read(path.join("revisions/2.json")).unwrap(),
        revision_bytes
    );
}

#[test]
fn stale_kernel_snapshot_cannot_overwrite_committed_revision() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("stale-kernel.jm");
    let kernel = jueming_kernel::KernelService;
    let initial = kernel
        .create_project(&request(path.clone(), "stale", "甲", "a"))
        .unwrap();
    let first = kernel
        .update_segment(
            &path,
            &initial,
            initial.segments[0].segment_id,
            "first writer",
        )
        .unwrap();
    let bytes = std::fs::read(path.join("revisions/2.json")).unwrap();
    assert!(
        kernel
            .update_segment(
                &path,
                &initial,
                initial.segments[1].segment_id,
                "stale writer"
            )
            .is_err()
    );
    assert!(kernel.save_project(&path, &initial).is_err());
    assert_eq!(kernel.open_project(&path).unwrap(), first);
    assert_eq!(std::fs::read(path.join("revisions/2.json")).unwrap(), bytes);
}

#[test]
fn workspace_projection_excludes_bodies_and_slices_validate_revision_and_bounds() {
    use jueming_protocol::{ParallelSliceRequest, RevisionId};
    let temp = tempfile::tempdir().unwrap();
    let host = LocalAppHost::new();
    let source = (0..600)
        .map(|index| format!("body-{index}"))
        .collect::<Vec<_>>()
        .join("\n");
    let initial = host
        .create_project(&request(
            temp.path().join("slices.jm"),
            "slices",
            &source,
            "target",
        ))
        .unwrap();
    let view = host.read(|k, s, _| k.workspace_project(s)).unwrap();
    let serialized = serde_json::to_string(&view).unwrap();
    assert!(!serialized.contains("body-599"));
    assert!(!serialized.contains("content_ref"));
    assert!(
        host.projection().unwrap().data["project"]
            .get("segments")
            .is_none()
    );
    let mut query = ParallelSliceRequest {
        project_id: initial.project.project_id,
        source_document_id: initial.documents[0].document_id,
        target_document_id: initial.documents[1].document_id,
        anchor_segment_id: None,
        anchor_alignment_id: None,
        halo: 0,
        revision_id: RevisionId::new(1),
        segment_ids: vec![initial.segments[599].segment_id],
    };
    let slice = host
        .read(|k, s, _| k.load_parallel_slice(s, &query))
        .unwrap();
    assert_eq!(slice.segments.len(), 1);
    assert_eq!(slice.segments[0].content, "body-599");
    query.segment_ids.clear();
    query.anchor_segment_id = Some(initial.segments[599].segment_id);
    assert!(
        !host
            .read(|k, s, _| k.load_parallel_slice(s, &query))
            .unwrap()
            .segments
            .is_empty()
    );
    query.anchor_segment_id = Some(jueming_protocol::SegmentId::new());
    assert!(
        host.read(|k, s, _| k.load_parallel_slice(s, &query))
            .is_err()
    );
    query.anchor_segment_id = None;
    query.segment_ids = initial
        .segments
        .iter()
        .take(201)
        .map(|s| s.segment_id)
        .collect();
    assert!(
        host.read(|k, s, _| k.load_parallel_slice(s, &query))
            .is_err()
    );
    query.segment_ids = vec![initial.segments[0].segment_id];
    query.revision_id = RevisionId::new(0);
    assert_eq!(
        host.read(|k, s, _| k.load_parallel_slice(s, &query))
            .unwrap_err()
            .code,
        "stale_revision"
    );
}
