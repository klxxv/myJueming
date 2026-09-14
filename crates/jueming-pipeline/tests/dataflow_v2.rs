use jueming_pipeline::{
    builtins::{self, Builtins},
    executor::{self, Provider, RunContext, Values},
    pool::{ArtifactManifest, DataPool},
    registry::Registry,
};
use jueming_protocol::*;
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::atomic::AtomicBool};
struct NoPlugins;
impl Provider for NoPlugins {
    fn execute(
        &self,
        _: &str,
        _: Values,
        _: &Value,
        _: &AtomicBool,
    ) -> Result<BTreeMap<String, Value>, String> {
        Err("no plugin bound".into())
    }
}
fn registry() -> Registry {
    let mut r = Registry::standard().unwrap();
    builtins::register(&mut r).unwrap();
    r
}
fn input(schema: &str, handle: &str) -> PlanNodeV2 {
    PlanNodeV2 {
        node_id: CommandId::new().to_string(),
        slot_id: format!("runtime.input.{schema}"),
        operator_id: format!("host.input.{schema}"),
        inputs: BTreeMap::new(),
        config: json!({"handle":handle}),
    }
}
fn edge(node: &PlanNodeV2, port: &str) -> PortRef {
    PortRef {
        node_id: node.node_id.clone(),
        port: port.into(),
    }
}
fn seed(r: &Registry, pool: &DataPool, schema: &str, value: Value) -> ArtifactManifest {
    pool.publish(
        r,
        ArtifactManifest {
            handle: String::new(),
            project_id: "project".into(),
            input_revision_id: "1".into(),
            run_id: "run".into(),
            schema: SchemaRef {
                name: schema.into(),
                version: 1,
            },
            sha256: String::new(),
            bytes: 0,
            provider_id: "test".into(),
            provider_release: "1".into(),
            dependencies: vec![],
        },
        &value,
    )
    .unwrap()
}
#[test]
fn all_profile_slots_are_registered_and_incompatible_binding_is_rejected() {
    let mut r = Registry::standard().unwrap();
    assert_eq!(r.slots().len(), 33);
    for slot in r.slots() {
        for p in slot.inputs.into_iter().chain(slot.outputs) {
            for schema in p.schemas {
                assert!(r.schema(&schema).is_ok());
            }
        }
    }
    builtins::register(&mut r).unwrap();
    let mut incompatible = r.operator("builtin.kwic").unwrap().clone();
    incompatible.operator_id = "bad".into();
    incompatible.inputs.remove(0);
    assert!(r.register_operator(incompatible).is_err());
    let mut replacement = r.operator("builtin.kwic").unwrap().clone();
    replacement.operator_id = "alternative.kwic".into();
    r.register_operator(replacement).unwrap();
    r.unregister_operator("builtin.kwic");
    assert_eq!(
        r.slot("analysis.kwic").unwrap().providers,
        vec!["alternative.kwic"]
    );
}
#[test]
fn named_inputs_execute_against_immutable_handles_and_reject_cross_schema_edges() {
    let temp = tempfile::tempdir().unwrap();
    let pool = DataPool::new(temp.path(), "project".into());
    let r = registry();
    let segments = seed(
        &r,
        &pool,
        "SegmentView",
        json!({"segments":[{"segment_id":"segment","text":"Heavy sudden rain and heavy rain","language":"en"}]}),
    );
    let query = seed(
        &r,
        &pool,
        "QuerySpec",
        json!({"text":"heavy rain","fuzzy":false,"max_gap":1,"similarity_operator":"fuzzy.edit_distance"}),
    );
    let a = input("SegmentView", &segments.handle);
    let b = input("QuerySpec", &query.handle);
    let search = PlanNodeV2 {
        node_id: CommandId::new().to_string(),
        slot_id: "analysis.basic_search".into(),
        operator_id: "builtin.search".into(),
        inputs: BTreeMap::from([
            ("segments".into(), vec![edge(&a, "value")]),
            ("query".into(), vec![edge(&b, "value")]),
        ]),
        config: json!({}),
    };
    let mut plan = PlanV2 {
        format_version: 2,
        outputs: vec![edge(&search, "occurrences")],
        nodes: vec![search, a, b],
    };
    let cancel = AtomicBool::new(false);
    let output = executor::execute(
        &r,
        &pool,
        &Builtins {
            plugins: &NoPlugins,
        },
        &plan,
        RunContext {
            run_id: "run",
            project_id: "project",
            revision_id: "1",
            cancelled: &cancel,
        },
    )
    .unwrap();
    let value = pool.read(&output[0].handle, "1").unwrap();
    assert_eq!(value["occurrences"].as_array().unwrap().len(), 2);
    assert_eq!(
        value["occurrences"][0]["matched_ranges"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert!(pool.read(&output[0].handle, "2").is_err());
    let incompatible = edge(&plan.nodes[1], "value");
    plan.nodes[0]
        .inputs
        .insert("query".into(), vec![incompatible]);
    assert!(r.compile(&plan).is_err());
    let file = temp
        .path()
        .join("cache/pipeline-v2/payloads")
        .join(&output[0].handle)
        .join("payload.json");
    std::fs::write(file, b"{}").unwrap();
    assert!(pool.read(&output[0].handle, "1").is_err());
}
#[test]
fn invalid_offsets_payload_shape_and_unbound_providers_are_rejected() {
    let r = registry();
    assert!(
        r.validate(
            &SchemaRef {
                name: "TextMap".into(),
                version: 1
            },
            &json!({"coordinate_unit":"utf16","ranges":[]})
        )
        .is_err()
    );
    let mut n = input("TextView", &CommandId::new().to_string());
    n.operator_id = "missing".into();
    let plan = PlanV2 {
        format_version: 2,
        outputs: vec![edge(&n, "value")],
        nodes: vec![n],
    };
    assert!(r.compile(&plan).is_err());
}

#[test]
fn indexes_keep_original_ranges_and_search_matches_scan() {
    let r = registry();
    let provider = Builtins {
        plugins: &NoPlugins,
    };
    let cancelled = AtomicBool::new(false);
    let segments = json!({"segments":[{"segment_id":"a","text":"雨 Heavy sudden rain heavy rain","language":"en"},{"segment_id":"b","text":"clear sky","language":"en"}]});
    let index = provider
        .execute(
            "builtin.basic_index",
            BTreeMap::from([("segments".into(), vec![segments.clone()])]),
            &json!({}),
            &cancelled,
        )
        .unwrap()["index"]
        .clone();
    r.validate(
        &SchemaRef {
            name: "BasicIndexRef".into(),
            version: 1,
        },
        &index,
    )
    .unwrap();
    assert_eq!(
        index["postings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["term"] == "rain")
            .unwrap()["segment_ids"],
        json!(["a"])
    );
    let search = |source| {
        provider.execute("builtin.search",BTreeMap::from([("segments".into(),vec![source]),("query".into(),vec![json!({"text":"heavy rain","fuzzy":false,"max_gap":1,"similarity_operator":"fuzzy.edit_distance"})])]),&json!({}),&cancelled).unwrap()["occurrences"]["occurrences"].as_array().unwrap().iter().map(|o|o["matched_ranges"].clone()).collect::<Vec<_>>()
    };
    assert_eq!(search(segments.clone()), search(index));
    assert_eq!(search(segments.clone())[0][0]["start_utf8"], 4);
    let tokenized = provider
        .execute(
            "builtin.tokenize",
            BTreeMap::from([("segments".into(), vec![segments])]),
            &json!({}),
            &cancelled,
        )
        .unwrap();
    let lexical = provider
        .execute(
            "builtin.lexical_index",
            BTreeMap::from([("tokens".into(), vec![tokenized["tokens"].clone()])]),
            &json!({}),
            &cancelled,
        )
        .unwrap()["index"]
        .clone();
    r.validate(
        &SchemaRef {
            name: "LexicalIndexRef".into(),
            version: 1,
        },
        &lexical,
    )
    .unwrap();
    let rain = lexical["postings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["term"] == "rain")
        .unwrap();
    assert_eq!(rain["occurrences"].as_array().unwrap().len(), 2);
    assert!(
        rain["occurrences"][0]["position"].as_u64().unwrap()
            < rain["occurrences"][1]["position"].as_u64().unwrap()
    );
    assert_eq!(lexical["tokenizer_id"], tokenized["tokens"]["tokenizer_id"]);
}

#[test]
fn identical_runs_reuse_computation_but_keep_distinct_run_provenance() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct Counting(AtomicUsize);
    impl Provider for Counting {
        fn execute(
            &self,
            op: &str,
            inputs: Values,
            config: &Value,
            cancelled: &AtomicBool,
        ) -> Result<BTreeMap<String, Value>, String> {
            self.0.fetch_add(1, Ordering::Relaxed);
            Builtins {
                plugins: &NoPlugins,
            }
            .execute(op, inputs, config, cancelled)
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let pool = DataPool::new(temp.path(), "project".into());
    let r = registry();
    let value = seed(
        &r,
        &pool,
        "SegmentView",
        json!({"segments":[{"segment_id":"s","text":"雨 rain","language":"en"}]}),
    );
    let source = input("SegmentView", &value.handle);
    let index = PlanNodeV2 {
        node_id: CommandId::new().to_string(),
        slot_id: "index.basic_string".into(),
        operator_id: "builtin.basic_index".into(),
        inputs: BTreeMap::from([("segments".into(), vec![edge(&source, "value")])]),
        config: json!({}),
    };
    let plan = PlanV2 {
        format_version: 2,
        outputs: vec![edge(&index, "index")],
        nodes: vec![source, index],
    };
    let provider = Counting(AtomicUsize::new(0));
    let cancel = AtomicBool::new(false);
    let run = |id| {
        executor::execute(
            &r,
            &pool,
            &provider,
            &plan,
            RunContext {
                run_id: id,
                project_id: "project",
                revision_id: "1",
                cancelled: &cancel,
            },
        )
        .unwrap()
    };
    let first = run("first");
    let second = run("second");
    assert_eq!(provider.0.load(Ordering::Relaxed), 1);
    assert_ne!(first[0].handle, second[0].handle);
    assert_eq!(second[0].run_id, "second");
    assert_eq!(first[0].sha256, second[0].sha256);
    pool.collect(&Default::default()).unwrap();
    // GC clears rebuildable outputs; references become cache misses, never false hits.
    assert!(pool.cached_outputs(&"0".repeat(64), "1").is_none());
}

#[test]
fn input_views_and_export_side_have_explicit_contracts() {
    let r = registry();
    let mut source = input("QuerySpec", "unused");
    source.config = json!({"view":"source"});
    assert!(
        r.compile(&PlanV2 {
            format_version: 2,
            outputs: vec![edge(&source, "value")],
            nodes: vec![source]
        })
        .is_err()
    );
    let result=Builtins{plugins:&NoPlugins}.execute("builtin.export_txt",BTreeMap::from([("view".into(),vec![json!({"project_id":"p","revision_id":"1","segments":[{"segment_id":"s","text":"rain","language":"en","side":"source"},{"segment_id":"t","text":"雨","language":"zh","side":"target"}],"alignments":[]})]),("spec".into(),vec![json!({"side":"target"})])]),&json!({}),&AtomicBool::new(false)).unwrap();
    assert_eq!(result["export"]["content"], "雨");
}

#[test]
fn candidate_projection_preserves_partial_coverage_even_when_no_links_exist() {
    let inputs = BTreeMap::from([
        (
            "occurrences".into(),
            vec![
                json!({"occurrences":[{"occurrence_id":"o","source":{"segment_id":"s","start_utf8":0,"end_utf8":4},"matched_ranges":[{"segment_id":"s","start_utf8":0,"end_utf8":4}],"text":"rain"}]}),
            ],
        ),
        (
            "contexts".into(),
            vec![
                json!({"contexts":[{"context_id":"c","sources":[{"segment_id":"s","text":"rain","language":"en"}],"targets":[{"segment_id":"t","text":"雨","language":"zh"}]}]}),
            ],
        ),
        (
            "alignments".into(),
            vec![
                json!({"contexts":[{"context_id":"c","coverage":"partial","edges":[],"score_kind":"cosine","provider_id":"test"}]}),
            ],
        ),
    ]);
    let value = Builtins {
        plugins: &NoPlugins,
    }
    .execute(
        "builtin.translation_candidates",
        inputs,
        &json!({}),
        &AtomicBool::new(false),
    )
    .unwrap()["candidates"]
        .clone();
    registry()
        .validate(
            &SchemaRef {
                name: "TranslationCandidateSet".into(),
                version: 1,
            },
            &value,
        )
        .unwrap();
    assert_eq!(value["items"][0]["alignment_coverage"], "partial");
    assert_eq!(value["items"][0]["candidates"], json!([]));
}
