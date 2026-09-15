//! Keep GC in its own integration-test process: the production gate protects
//! every active graph in the process, including graphs in unrelated test pools.
use jueming_pipeline::{
    pool::{ArtifactManifest, DataPool, pin_execution},
    registry::Registry,
};
use jueming_protocol::SchemaRef;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn collection_respects_execution_pins_and_invalidates_cached_outputs() {
    let temp = tempfile::tempdir().unwrap();
    let pool = DataPool::new(temp.path(), "project".into());
    let registry = Registry::standard().unwrap();
    let value = json!({"segments":[{"segment_id":"s","text":"雨 rain","language":"en"}]});
    let publish = |dependencies| {
        pool.publish(
            &registry,
            ArtifactManifest {
                handle: String::new(),
                project_id: "project".into(),
                input_revision_id: "1".into(),
                run_id: "run".into(),
                schema: SchemaRef {
                    name: "SegmentView".into(),
                    version: 1,
                },
                sha256: String::new(),
                bytes: 0,
                provider_id: "test".into(),
                provider_release: "1".into(),
                dependencies,
            },
            &value,
        )
        .unwrap()
    };
    let source = publish(vec![]);
    let output = publish(vec![source.handle.clone()]);
    let key = "0".repeat(64);
    pool.remember_outputs(
        &key,
        &BTreeMap::from([("value".into(), output.handle.clone())]),
    )
    .unwrap();
    assert_eq!(pool.cached_outputs(&key, "1").unwrap()["value"], value);

    let pin = pin_execution().unwrap();
    assert_eq!(
        pool.collect(&BTreeSet::new()).unwrap_err(),
        "active data views or graph executions pin the cache"
    );
    drop(pin);
    assert_eq!(pool.read(&output.handle, "1").unwrap(), value);

    // A view pin retains both the selected output and its transitive inputs.
    assert_eq!(
        pool.collect(&BTreeSet::from([output.handle.clone()]))
            .unwrap(),
        0
    );
    assert_eq!(pool.read(&source.handle, "1").unwrap(), value);
    assert!(pool.cached_outputs(&key, "1").is_some());

    assert_eq!(pool.collect(&BTreeSet::new()).unwrap(), 2);
    assert!(pool.list().unwrap().is_empty());
    assert!(pool.cached_outputs(&key, "1").is_none());
}
