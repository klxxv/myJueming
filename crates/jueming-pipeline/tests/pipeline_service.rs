use jueming_core::{RevisionId, SegmentId};
use jueming_pipeline::{
    CancellationToken, CreatePipelineMethod, ExecutePipelineRequest, PipelineError,
    PipelineMethodRevisionId, PipelineNode, PipelineNodeId, PipelinePlanSnapshot, PipelineService,
    UpdatePipelineMethod,
};
use serde_json::json;
use tempfile::TempDir;

fn project_root(temp: &TempDir) -> std::path::PathBuf {
    temp.path().join("pipeline-test.jm")
}

fn plan(custom_dictionary: serde_json::Value) -> PipelinePlanSnapshot {
    let source = PipelineNodeId::new();
    let normalize = PipelineNodeId::new();
    let tokenize = PipelineNodeId::new();
    let artifact = PipelineNodeId::new();
    PipelinePlanSnapshot {
        method_id: jueming_pipeline::PipelineMethodId::new(),
        method_revision_id: jueming_pipeline::PipelineMethodRevisionId::new(),
        nodes: vec![
            PipelineNode {
                node_id: source,
                operator: "source".into(),
                inputs: vec![],
                config: json!({}),
            },
            PipelineNode {
                node_id: normalize,
                operator: "normalize".into(),
                inputs: vec![source],
                config: json!({ "trim": true }),
            },
            PipelineNode {
                node_id: tokenize,
                operator: "chinese_tokenize".into(),
                inputs: vec![normalize],
                config: json!({ "hmm": false, "custom_dictionary": custom_dictionary }),
            },
            PipelineNode {
                node_id: artifact,
                operator: "artifact".into(),
                inputs: vec![tokenize],
                config: json!({}),
            },
        ],
        output_node_id: artifact,
    }
}

fn create(service: &PipelineService) -> jueming_pipeline::PipelineMethod {
    service
        .create_method(CreatePipelineMethod {
            name: "中文分词".into(),
            plan: plan(json!([{ "word": "清华大学", "frequency": 3000000, "tag": "nt" }])),
        })
        .unwrap()
}

fn execute(
    service: &PipelineService,
    method_id: jueming_pipeline::PipelineMethodId,
    source: &str,
) -> jueming_pipeline::PipelineExecution {
    service
        .execute(
            ExecutePipelineRequest {
                method_id,
                method_revision_id: None,
                input_revision_id: RevisionId::new(7),
                segment_id: SegmentId::new(),
                source_content: source.into(),
            },
            &CancellationToken::new(),
        )
        .unwrap()
}

#[test]
fn persists_method_and_artifact_across_restart_without_changing_source_input() {
    let temp = TempDir::new().unwrap();
    let root = project_root(&temp);
    let service = PipelineService::open(&root).unwrap();
    let method = create(&service);
    let original = "  我来到北京清华大学  ".to_owned();
    let execution = execute(&service, method.method_id, &original);

    assert_eq!(original, "  我来到北京清华大学  ");
    assert_eq!(execution.artifact.input_revision_id, RevisionId::new(7));
    assert!(
        execution
            .artifact
            .tokens
            .iter()
            .any(|token| token.text == "清华大学")
    );

    drop(service);
    let reopened = PipelineService::open(&root).unwrap();
    let reloaded_method = reopened.load_method(method.method_id).unwrap();
    assert_eq!(
        reloaded_method.current.method_revision_id,
        method.current.method_revision_id
    );
    let reloaded_artifact = reopened
        .load_artifact(execution.artifact.artifact_id)
        .unwrap();
    assert_eq!(reloaded_artifact, execution.artifact);
    assert_eq!(
        reopened
            .list_artifacts_for_revision(RevisionId::new(7))
            .unwrap(),
        vec![jueming_pipeline::PipelineArtifactSummary {
            artifact_id: execution.artifact.artifact_id,
            input_revision_id: RevisionId::new(7),
            segment_id: execution.artifact.segment_id,
            method_id: method.method_id,
            method_revision_id: method.current.method_revision_id,
        }]
    );
    assert!(
        root.join("extensions")
            .join("pipeline")
            .join("manifest-v1.json")
            .exists()
    );
}

#[test]
fn stale_method_edit_is_rejected_and_old_revision_remains_executable() {
    let temp = TempDir::new().unwrap();
    let service = PipelineService::open(project_root(&temp)).unwrap();
    let method = create(&service);
    let old_revision = method.current.method_revision_id;
    let updated = service
        .update_method(UpdatePipelineMethod {
            method_id: method.method_id,
            base_method_revision_id: old_revision,
            name: Some("分词 v2".into()),
            plan: plan(json!([])),
        })
        .unwrap();
    let stale = service.update_method(UpdatePipelineMethod {
        method_id: method.method_id,
        base_method_revision_id: old_revision,
        name: None,
        plan: plan(json!([])),
    });
    assert!(matches!(
        stale,
        Err(PipelineError::StaleMethodRevision { .. })
    ));

    let old_execution = service
        .execute(
            ExecutePipelineRequest {
                method_id: method.method_id,
                method_revision_id: Some(old_revision),
                input_revision_id: RevisionId::new(3),
                segment_id: SegmentId::new(),
                source_content: "北京清华大学".into(),
            },
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(old_execution.artifact.method_revision_id, old_revision);
    assert_ne!(updated.current.method_revision_id, old_revision);
}

#[test]
fn host_reserved_revision_id_is_exactly_once_and_normal_updates_stay_generated() {
    let temp = TempDir::new().unwrap();
    let service = PipelineService::open(project_root(&temp)).unwrap();
    let method = create(&service);
    let reserved = PipelineMethodRevisionId::new();
    let request = UpdatePipelineMethod {
        method_id: method.method_id,
        base_method_revision_id: method.current.method_revision_id,
        name: Some("reserved approval".into()),
        plan: plan(json!([])),
    };

    let committed = service
        .update_method_with_revision_id(request.clone(), reserved)
        .unwrap();
    assert_eq!(committed.current.method_revision_id, reserved);
    assert_eq!(
        service
            .load_method_revision(method.method_id, reserved)
            .unwrap()
            .method_name,
        "reserved approval"
    );
    assert!(matches!(
        service.update_method_with_revision_id(request, reserved),
        Err(PipelineError::MethodRevisionAlreadyExists(_))
    ));

    let generated = service
        .update_method(UpdatePipelineMethod {
            method_id: method.method_id,
            base_method_revision_id: reserved,
            name: None,
            plan: plan(json!([])),
        })
        .unwrap();
    assert_ne!(generated.current.method_revision_id, reserved);
}

#[test]
fn cancelled_execution_does_not_publish_an_artifact() {
    let temp = TempDir::new().unwrap();
    let root = project_root(&temp);
    let service = PipelineService::open(&root).unwrap();
    let method = create(&service);
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let result = service.execute(
        ExecutePipelineRequest {
            method_id: method.method_id,
            method_revision_id: None,
            input_revision_id: RevisionId::new(1),
            segment_id: SegmentId::new(),
            source_content: "取消前不能发布".into(),
        },
        &cancellation,
    );
    assert!(matches!(result, Err(PipelineError::Cancelled)));
    assert!(
        !root
            .join("extensions")
            .join("pipeline")
            .join("artifacts")
            .exists()
    );
}

#[test]
fn rejects_unknown_operator_and_cycles_before_method_publication() {
    let temp = TempDir::new().unwrap();
    let service = PipelineService::open(project_root(&temp)).unwrap();
    let node = PipelineNodeId::new();
    let unknown = PipelinePlanSnapshot {
        method_id: jueming_pipeline::PipelineMethodId::new(),
        method_revision_id: jueming_pipeline::PipelineMethodRevisionId::new(),
        nodes: vec![PipelineNode {
            node_id: node,
            operator: "shell".into(),
            inputs: vec![],
            config: json!({}),
        }],
        output_node_id: node,
    };
    assert!(matches!(
        service.create_method(CreatePipelineMethod {
            name: "bad".into(),
            plan: unknown
        }),
        Err(PipelineError::UnknownOperator { .. })
    ));

    let first = PipelineNodeId::new();
    let second = PipelineNodeId::new();
    let cycle = PipelinePlanSnapshot {
        method_id: jueming_pipeline::PipelineMethodId::new(),
        method_revision_id: jueming_pipeline::PipelineMethodRevisionId::new(),
        nodes: vec![
            PipelineNode {
                node_id: first,
                operator: "normalize".into(),
                inputs: vec![second],
                config: json!({}),
            },
            PipelineNode {
                node_id: second,
                operator: "normalize".into(),
                inputs: vec![first],
                config: json!({}),
            },
        ],
        output_node_id: first,
    };
    assert!(matches!(
        service.create_method(CreatePipelineMethod {
            name: "cycle".into(),
            plan: cycle
        }),
        Err(PipelineError::InvalidOutputNode(_)) | Err(PipelineError::Cycle(_))
    ));
}
