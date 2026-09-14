//! Generic named-port executor; provider lookup is independent of graph compilation.
use crate::{
    pool::{ArtifactManifest, DataPool},
    registry::Registry,
};
use jueming_protocol::PlanV2;
use serde_json::Value;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};

pub type Values = BTreeMap<String, Vec<Value>>;
pub trait Provider: Send + Sync {
    fn execute(
        &self,
        operator: &str,
        inputs: Values,
        config: &Value,
        cancelled: &AtomicBool,
    ) -> Result<BTreeMap<String, Value>, String>;
}
pub struct RunContext<'a> {
    pub run_id: &'a str,
    pub project_id: &'a str,
    pub revision_id: &'a str,
    pub cancelled: &'a AtomicBool,
}
pub fn execute(
    registry: &Registry,
    pool: &DataPool,
    provider: &dyn Provider,
    plan: &PlanV2,
    context: RunContext<'_>,
) -> Result<Vec<ArtifactManifest>, String> {
    execute_observed(registry, pool, provider, plan, context, |_, _, _| Ok(()))
}
pub fn execute_observed(
    registry: &Registry,
    pool: &DataPool,
    provider: &dyn Provider,
    plan: &PlanV2,
    context: RunContext<'_>,
    mut observe: impl FnMut(&str, bool, &[ArtifactManifest]) -> Result<(), String>,
) -> Result<Vec<ArtifactManifest>, String> {
    let _pin = crate::pool::pin_execution()?;
    let order = registry.compile(plan)?;
    let mut outputs: BTreeMap<(String, String), ArtifactManifest> = BTreeMap::new();
    for id in order {
        if context.cancelled.load(Ordering::Acquire) {
            return Err("cancelled".into());
        }
        let node = plan
            .nodes
            .iter()
            .find(|n| n.node_id == id)
            .expect("compiled node");
        let op = registry
            .operator(&node.operator_id)
            .expect("compiled operator");
        observe(&id, false, &[])?;
        let mut inputs = Values::new();
        let mut input_keys = BTreeMap::new();
        let mut dependencies = Vec::new();
        for (port, edges) in &node.inputs {
            let mut values = Vec::new();
            let mut keys = Vec::new();
            for edge in edges {
                let m = outputs
                    .get(&(edge.node_id.clone(), edge.port.clone()))
                    .ok_or("upstream output missing")?;
                values.push(pool.read(&m.handle, context.revision_id)?);
                keys.push(serde_json::json!({"schema":m.schema,"sha256":m.sha256}));
                dependencies.push(m.handle.clone());
            }
            inputs.insert(port.clone(), values);
            input_keys.insert(port.clone(), keys);
        }
        let cache_key = crate::pool::digest(
            &serde_json::to_vec(&serde_json::json!({
                "contract":1,"project":context.project_id,"revision":context.revision_id,
                "operator":op,"config":node.config,"inputs":input_keys
            }))
            .map_err(|e| e.to_string())?,
        );
        let result = if node.operator_id.starts_with("host.input.") {
            let handle = node
                .config
                .get("handle")
                .and_then(Value::as_str)
                .ok_or("input handle required")?;
            let m = pool.manifest(handle)?;
            if m.schema != op.outputs[0].schemas[0] {
                return Err("input handle schema mismatch".into());
            }
            dependencies.push(handle.into());
            BTreeMap::from([("value".into(), pool.read(handle, context.revision_id)?)])
        } else if let Some(cached) = pool.cached_outputs(&cache_key, context.revision_id) {
            cached
        } else {
            provider.execute(&node.operator_id, inputs, &node.config, context.cancelled)?
        };
        if result.len() != op.outputs.len() {
            return Err("provider returned incorrect output ports".into());
        }
        let mut published = BTreeMap::new();
        let mut node_artifacts = Vec::new();
        for port in &op.outputs {
            let value = result
                .get(&port.name)
                .ok_or("provider omitted output port")?;
            let schema = port
                .schemas
                .iter()
                .find(|s| registry.validate(s, value).is_ok())
                .ok_or("provider output schema invalid")?
                .clone();
            if context.cancelled.load(Ordering::Acquire) {
                return Err("cancelled".into());
            }
            let m = pool.publish(
                registry,
                ArtifactManifest {
                    handle: String::new(),
                    project_id: context.project_id.into(),
                    input_revision_id: context.revision_id.into(),
                    run_id: context.run_id.into(),
                    schema,
                    sha256: String::new(),
                    bytes: 0,
                    provider_id: op.operator_id.clone(),
                    provider_release: op.release.clone(),
                    dependencies: dependencies.clone(),
                },
                value,
            )?;
            published.insert(port.name.clone(), m.handle.clone());
            node_artifacts.push(m.clone());
            outputs.insert((id.clone(), port.name.clone()), m);
        }
        if !node.operator_id.starts_with("host.input.") {
            pool.remember_outputs(&cache_key, &published)?;
        }
        observe(&id, true, &node_artifacts)?;
    }
    plan.outputs
        .iter()
        .map(|p| {
            outputs
                .get(&(p.node_id.clone(), p.port.clone()))
                .cloned()
                .ok_or("output missing".into())
        })
        .collect()
}
