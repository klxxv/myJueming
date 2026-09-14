use jueming_application::{AgentCall, LocalAppHost};
use jueming_protocol::*;
use serde_json::{Value, json};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
fn call(host: &LocalAppHost, method: &str, params: Value, binding: Option<&str>) -> Value {
    host.dispatch_native(AgentCall {
        request_id: CommandId::new().to_string(),
        method: method.into(),
        params: params.as_object().unwrap().clone(),
        binding_id: binding.map(str::to_owned),
    })
    .unwrap()
    .data
}
#[test]
fn catalog_is_available_without_project_and_feature_activation_is_native_only() {
    let host = LocalAppHost::new();
    let value = call(&host, "capabilities.get", json!({}), None);
    assert!(
        value["slots"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["slot_id"] == "analysis.kwic" && s["state"] == "bound")
    );
    assert!(
        value["slots"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["slot_id"] == "relation.word_alignment" && s["state"] == "unbound")
    );
    let error = host
        .dispatch(AgentCall {
            request_id: "external".into(),
            method: "features.enable".into(),
            params: Default::default(),
            binding_id: None,
        })
        .unwrap_err();
    assert_eq!(error.code, "native_ui_required");
}

#[test]
fn generic_graph_and_data_pool_keep_frozen_revision_and_method_history() {
    let temp = tempfile::tempdir().unwrap();
    let host = Arc::new(LocalAppHost::new());
    let profile = ImportProfile::new(Encoding::Utf8, SegmentationMode::NonEmptyLine);
    let side = |lang: &str, content: &str| ImportSideRequest {
        expected_sha256: None,
        language_id: lang.into(),
        title: lang.into(),
        input: TextInput::Paste {
            label: lang.into(),
            text: content.into(),
        },
        profile: profile.clone(),
    };
    let project = host
        .create_project(&CreateProjectRequest {
            additional_targets: Vec::new(),
            project_path: temp.path().join("flow.jm").to_string_lossy().into_owned(),
            name: "flow".into(),
            source: side("en", "heavy rain"),
            target: side("zh-CN", "大雨"),
        })
        .unwrap();
    let binding = call(&host, "app.bind_session", json!({}), None)["binding_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let lease = call(
        &host,
        "pool.open_view",
        json!({"schema":{"name":"SegmentView","version":1},"side":"source"}),
        Some(&binding),
    );
    let first = call(
        &host,
        "pool.read_batch",
        json!({"view_id":lease["view_id"]}),
        Some(&binding),
    );
    assert_eq!(first["data"]["segments"][0]["text"], "heavy rain");
    let source_id = CommandId::new().to_string();
    let query_id = CommandId::new().to_string();
    let search_id = CommandId::new().to_string();
    let plan = json!({"format_version":2,"nodes":[{"node_id":source_id,"slot_id":"runtime.input.SegmentView","operator_id":"host.input.SegmentView","inputs":{},"config":{"view":"source"}},{"node_id":query_id,"slot_id":"runtime.input.QuerySpec","operator_id":"host.input.QuerySpec","inputs":{},"config":{"value":{"text":"heavy rain","fuzzy":false,"max_gap":0,"similarity_operator":"fuzzy.edit_distance"}}},{"node_id":search_id,"slot_id":"analysis.basic_search","operator_id":"builtin.search","inputs":{"segments":[{"node_id":source_id,"port":"value"}],"query":[{"node_id":query_id,"port":"value"}]},"config":{}}],"outputs":[{"node_id":search_id,"port":"occurrences"}]});
    assert_eq!(
        call(
            &host,
            "pipeline.validate_plan",
            plan.clone(),
            Some(&binding)
        )
        .as_array()
        .unwrap()
        .len(),
        3
    );
    let method = call(
        &host,
        "pipeline.save_method_v2",
        json!({"name":"Search","plan":plan}),
        Some(&binding),
    );
    let run = call(
        &host,
        "pipeline.start",
        json!({"method_id":method["method_id"],"method_revision_id":method["method_revision_id"]}),
        Some(&binding),
    );
    let segment_id = project
        .segments
        .iter()
        .find(|s| s.document_id == project.documents[0].document_id)
        .unwrap()
        .segment_id;
    host.execute_native_command(CommandEnvelope::new(
        CommandId::new(),
        project.project.project_id,
        project.project.current_revision_id,
        CommandKind::UpdateSegment,
        json!({"segment_id":segment_id,"content":"clear sky"}),
    ))
    .unwrap();
    let frozen = call(
        &host,
        "pool.read_segment",
        json!({"view_id":lease["view_id"],"segment_id":segment_id}),
        Some(&binding),
    );
    assert_eq!(frozen["content"], "heavy rain");
    let start = Instant::now();
    let finished = loop {
        let result = call(
            &host,
            "pipeline.get_graph_run",
            json!({"run_id":run["run_id"]}),
            Some(&binding),
        );
        if result["status"] == "completed" {
            break result;
        }
        assert_ne!(result["status"], "failed", "{result}");
        assert!(start.elapsed() < Duration::from_secs(30));
        std::thread::sleep(Duration::from_millis(20));
    };
    let artifact = call(
        &host,
        "pipeline.read_artifact",
        json!({"handle":finished["artifacts"][0]["handle"]}),
        Some(&binding),
    );
    assert_eq!(artifact["occurrences"].as_array().unwrap().len(), 1);
    let newer = call(
        &host,
        "pipeline.save_method_v2",
        json!({"method_id":method["method_id"],"name":"Renamed","base_method_revision_id":method["method_revision_id"],"plan":plan}),
        Some(&binding),
    );
    assert_ne!(method["method_revision_id"], newer["method_revision_id"]);
    call(
        &host,
        "pool.close_view",
        json!({"view_id":lease["view_id"]}),
        Some(&binding),
    );
    call(&host, "pool.collect_cache", json!({}), Some(&binding));
    assert_eq!(
        call(
            &host,
            "pipeline.read_artifact",
            json!({"handle":finished["artifacts"][0]["handle"]}),
            Some(&binding)
        )["occurrences"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn local_package_install_validates_slot_and_file_integrity_without_running_code() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("package");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::write(source.join("worker.py"), "# inert test fixture\n").unwrap();
    let manifest = json!({"protocol_version":1,"package_id":"test-provider","release":"1.0.0","runtime":"python-3.12","entrypoint":"worker.py","operators":["test.similarity"],"slots":{"test.similarity":"text.similarity"},"files":[{"path":"worker.py","bytes":21,"sha256":jueming_pipeline::pool::digest(b"# inert test fixture\n"),"executable":false,"url":null}]});
    // Compute the fixture length explicitly so the installer verifies the complete file.
    let mut manifest = manifest;
    manifest["files"][0]["bytes"] =
        json!(std::fs::metadata(source.join("worker.py")).unwrap().len());
    std::fs::write(
        source.join("plugin.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    let host = Arc::new(LocalAppHost::new());
    host.configure_research(temp.path().join("device"), temp.path().join("bundle"))
        .unwrap();
    assert_eq!(
        call(
            &host,
            "plugins.install_local",
            json!({"package_path":source}),
            None
        )["package_id"],
        "test-provider"
    );
    std::fs::write(source.join("worker.py"), "tampered").unwrap();
    assert!(
        host.dispatch_native(AgentCall {
            request_id: CommandId::new().to_string(),
            method: "plugins.install_local".into(),
            params: json!({"package_path":source}).as_object().unwrap().clone(),
            binding_id: None
        })
        .is_err()
    );
}
#[test]
#[ignore = "requires JUEMING_RESEARCH_BUNDLE containing real managed Python and pinned XLM-R weights"]
fn real_offline_workers_run_both_fuzzy_providers_and_commit_history() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("research.jm");
    let host = Arc::new(LocalAppHost::new());
    host.configure_research(
        temp.path().join("device"),
        std::env::var_os("JUEMING_RESEARCH_BUNDLE")
            .expect("bundle path")
            .into(),
    )
    .unwrap();
    let profile = ImportProfile::new(Encoding::Utf8, SegmentationMode::NonEmptyLine);
    let side = |language: &str, content: &str| ImportSideRequest {
        expected_sha256: None,
        language_id: language.into(),
        title: language.into(),
        input: TextInput::Paste {
            label: language.into(),
            text: content.into(),
        },
        profile: profile.clone(),
    };
    host.create_project(&CreateProjectRequest {
        additional_targets: Vec::new(),
        project_path: path.to_string_lossy().into_owned(),
        name: "research".into(),
        source: side(
            "en",
            "The heavy rain kept them indoors.\nAfter a night of heavy rain, the river rose.",
        ),
        target: side(
            "zh-CN",
            "大雨下个不停，他们整个下午都没出门。\n一夜暴雨过后，河水涨了起来。",
        ),
    })
    .unwrap();
    let binding = call(&host, "app.bind_session", json!({}), None)["binding_id"]
        .as_str()
        .unwrap()
        .to_owned();
    call(&host, "features.enable", json!({}), None);
    let start = Instant::now();
    loop {
        let cap = call(&host, "capabilities.get", json!({}), None);
        let feature = &cap["features"][0];
        if feature["status"] == "ready" {
            break;
        }
        assert_ne!(feature["status"], "failed", "{feature}");
        assert!(start.elapsed() < Duration::from_secs(300));
        std::thread::sleep(Duration::from_millis(200));
    }
    // A shared input branches to two independently bound operators in the generic executor.
    let pair_id = CommandId::new().to_string();
    let edit_id = CommandId::new().to_string();
    let gram_id = CommandId::new().to_string();
    let plan = json!({"format_version":2,"nodes":[{"node_id":pair_id,"slot_id":"runtime.input.TextPairBatch","operator_id":"host.input.TextPairBatch","inputs":{},"config":{"value":{"pairs":[{"pair_id":"near-match","left":"kitten","right":"sitting"}]}}},{"node_id":edit_id,"slot_id":"text.similarity","operator_id":"fuzzy.edit_distance","inputs":{"pairs":[{"node_id":pair_id,"port":"value"}]},"config":{}},{"node_id":gram_id,"slot_id":"text.similarity","operator_id":"fuzzy.char_ngram","inputs":{"pairs":[{"node_id":pair_id,"port":"value"}]},"config":{}}],"outputs":[{"node_id":edit_id,"port":"scores"},{"node_id":gram_id,"port":"scores"}]});
    let method = call(
        &host,
        "pipeline.save_method_v2",
        json!({"name":"Compare real fuzzy providers","plan":plan}),
        Some(&binding),
    );
    let graph = call(
        &host,
        "pipeline.start",
        json!({"method_id":method["method_id"],"method_revision_id":method["method_revision_id"]}),
        Some(&binding),
    );
    let start = Instant::now();
    let result = loop {
        let value = call(
            &host,
            "pipeline.get_graph_run",
            json!({"run_id":graph["run_id"]}),
            Some(&binding),
        );
        if value["status"] == "completed" {
            break value;
        }
        assert_ne!(value["status"], "failed", "{value}");
        assert!(start.elapsed() < Duration::from_secs(30));
        std::thread::sleep(Duration::from_millis(25));
    };
    assert_eq!(result["completed"], 3);
    let scores: Vec<_> = result["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| {
            call(
                &host,
                "pipeline.read_artifact",
                json!({"handle":m["handle"]}),
                Some(&binding),
            )["items"][0]["score"]
                .as_f64()
                .unwrap()
        })
        .collect();
    assert_ne!(scores[0], scores[1]);
    for provider in ["fuzzy.edit_distance", "fuzzy.char_ngram"] {
        let run = call(
            &host,
            "research.start",
            json!({"query":"heavy rain","fuzzy":true,"similarity_operator":provider,"auto_locate":true,"max_gap":2}),
            Some(&binding),
        );
        let id = run["run_id"].as_str().unwrap();
        let start = Instant::now();
        loop {
            let state = call(
                &host,
                "pipeline.get_run",
                json!({"run_id":id}),
                Some(&binding),
            );
            if state["status"] == "completed" {
                break;
            }
            assert_ne!(state["status"], "failed", "{state}");
            assert!(start.elapsed() < Duration::from_secs(300));
            std::thread::sleep(Duration::from_millis(200));
        }
        let page = call(
            &host,
            "pipeline.read_result",
            json!({"run_id":id,"cursor":0,"limit":20}),
            Some(&binding),
        );
        assert_eq!(page["total"], 2);
        let item = &page["items"][0];
        let candidate = &item["candidates"][0];
        assert!(
            candidate["ranges"]
                .as_array()
                .is_some_and(|ranges| !ranges.is_empty()),
            "{page}"
        );
        eprintln!("{provider}: {}", candidate["text"]);
        let confirmed = call(
            &host,
            "research.confirm",
            json!({"run_id":id,"occurrence_id":item["occurrence_id"],"kind":"translation","target_ranges":candidate["ranges"],"group_name":"大雨","strategy":"直译"}),
            Some(&binding),
        );
        assert!(confirmed["committed_revision_id"].is_string());
        let refreshed = call(
            &host,
            "pipeline.read_result",
            json!({"run_id":id}),
            Some(&binding),
        );
        assert_eq!(refreshed["items"][0]["status"], "confirmed");
        assert_eq!(
            call(
                &host,
                "research.summary",
                json!({"run_id":id}),
                Some(&binding)
            )["confirmed"],
            1
        );
        call(
            &host,
            "research.merge_groups",
            json!({"run_id":id,"from_group":"大雨","to_group":"强降雨"}),
            Some(&binding),
        );
        assert_eq!(
            call(
                &host,
                "pipeline.read_result",
                json!({"run_id":id,"group_name":"强降雨"}),
                Some(&binding)
            )["total"],
            1
        );
        let current = host.current_snapshot().unwrap();
        let source: SegmentId =
            serde_json::from_value(item["source"]["segment_id"].clone()).unwrap();
        let alignment = current
            .alignments
            .iter()
            .find(|a| a.source_segment_ids.contains(&source))
            .unwrap();
        host.execute_native_command(CommandEnvelope::new(
            CommandId::new(),
            current.project.project_id,
            current.project.current_revision_id,
            CommandKind::UnlinkAlignment,
            json!({"alignment_id":alignment.alignment_id}),
        ))
        .unwrap();
        assert_eq!(
            call(
                &host,
                "pipeline.read_result",
                json!({"run_id":id}),
                Some(&binding)
            )["items"][0]["status"],
            "needs_review"
        );
        assert_eq!(
            call(
                &host,
                "research.summary",
                json!({"run_id":id}),
                Some(&binding)
            )["confirmed"],
            0
        );
        let current = host.current_snapshot().unwrap();
        host.execute_native_command(CommandEnvelope::new(
            CommandId::new(),
            current.project.project_id,
            current.project.current_revision_id,
            CommandKind::Undo,
            json!({}),
        ))
        .unwrap();
        assert_eq!(
            call(
                &host,
                "pipeline.read_result",
                json!({"run_id":id}),
                Some(&binding)
            )["items"][0]["status"],
            "confirmed"
        );
    }
    call(&host, "features.disable", json!({}), None);
    assert_eq!(
        call(&host, "pipeline.list_runs", json!({}), Some(&binding))
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let kernel = jueming_kernel::KernelService;
    let saved = kernel.open_project(&path).unwrap();
    assert_eq!(saved.research_records.len(), 2);
    assert_eq!(saved.contract_version, "1.1");
}

#[test]
#[ignore = "requires JUEMING_RESEARCH_BUNDLE managed Python"]
fn real_offline_workers_cancel_blocked_input_and_restart_after_cancellation() {
    use jueming_pipeline::plugin::PluginWorker;
    use std::sync::atomic::AtomicBool;
    let root =
        std::path::PathBuf::from(std::env::var_os("JUEMING_RESEARCH_BUNDLE").expect("bundle"));
    let platform = root.join(format!("bundle.{}.json", std::env::consts::ARCH));
    let manifest: Value = serde_json::from_slice(
        &std::fs::read(if platform.is_file() {
            platform
        } else {
            root.join("bundle.json")
        })
        .unwrap(),
    )
    .unwrap();
    let python = root.join(manifest["python"].as_str().unwrap());
    let packages = root.join(manifest["packages"].as_str().unwrap());
    let model = root.join(manifest["model"].as_str().unwrap());
    let temp = tempfile::tempdir().unwrap();
    let script = temp.path().join("faults.py");
    std::fs::write(
        &script,
        r#"import json,sys,time
for line in sys.stdin:
    request=json.loads(line)
    if request['method']=='hello': data={'protocol_version':1,'operators':['fixture.echo']}
    else:
        if request['config'].get('sleep'): time.sleep(60)
        data={'result':{'ok':True}}
    print(json.dumps({'request_id':request['request_id'],'data':data}),flush=True)
"#,
    )
    .unwrap();
    let mut worker = PluginWorker::start(&python, &packages, &script, &model).unwrap();
    worker
        .handshake(&["fixture.echo".into()], &AtomicBool::new(false))
        .unwrap();
    let cancelled = Arc::new(AtomicBool::new(false));
    let flag = cancelled.clone();
    let interrupt = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(100));
        flag.store(true, std::sync::atomic::Ordering::Release);
    });
    let start = Instant::now();
    assert_eq!(
        worker
            .execute(
                "fixture.echo",
                Default::default(),
                &json!({"sleep":true}),
                &cancelled
            )
            .unwrap_err(),
        "cancelled"
    );
    assert!(start.elapsed() < Duration::from_secs(2));
    interrupt.join().unwrap();
    assert_eq!(
        worker
            .execute(
                "fixture.echo",
                Default::default(),
                &json!({}),
                &AtomicBool::new(false)
            )
            .unwrap()["result"]["ok"],
        true
    );
    drop(worker);
    // Stop reading after hello. A frame larger than the OS pipe must remain cancellable.
    std::fs::write(&script,r#"import json,sys,time
request=json.loads(sys.stdin.readline())
print(json.dumps({'request_id':request['request_id'],'data':{'protocol_version':1,'operators':['fixture.echo']}}),flush=True)
time.sleep(60)
"#).unwrap();
    let mut worker = PluginWorker::start(&python, &packages, &script, &model).unwrap();
    worker
        .handshake(&["fixture.echo".into()], &AtomicBool::new(false))
        .unwrap();
    let cancelled = Arc::new(AtomicBool::new(false));
    let flag = cancelled.clone();
    let interrupt = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(100));
        flag.store(true, std::sync::atomic::Ordering::Release);
    });
    let start = Instant::now();
    assert_eq!(
        worker
            .execute(
                "fixture.echo",
                Default::default(),
                &json!({"large":"x".repeat(1024*1024)}),
                &cancelled
            )
            .unwrap_err(),
        "cancelled"
    );
    assert!(start.elapsed() < Duration::from_secs(2));
    interrupt.join().unwrap();
}
