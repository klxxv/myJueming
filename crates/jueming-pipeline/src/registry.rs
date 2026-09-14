//! Registered contracts and provider bindings. No filesystem or network schema resolution.
use jueming_protocol::{OperatorDescriptor, PlanV2, PortDescriptor, SchemaRef, SlotState};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SchemaDefinition {
    pub name: String,
    pub version: u32,
    pub schema: Value,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SlotMigration {
    pub legacy_name: String,
    pub canonical_slot: Option<String>,
    pub action: String,
    pub reason: String,
}
#[derive(Deserialize)]
struct Catalog {
    slots: Vec<SlotState>,
    schemas: Vec<SchemaDefinition>,
}

pub struct Registry {
    slots: BTreeMap<String, SlotState>,
    schemas: BTreeMap<(String, u32), SchemaDefinition>,
    validators: BTreeMap<(String, u32), jsonschema::Validator>,
    operators: BTreeMap<String, OperatorDescriptor>,
}
impl Registry {
    pub fn migrations() -> Result<Vec<SlotMigration>, String> {
        serde_json::from_str(include_str!(
            "../../../schemas/pipeline/slot-migrations-v2.json"
        ))
        .map_err(|e| e.to_string())
    }
    pub fn resolve_slot(&self, id: &str) -> Result<String, String> {
        if self.slots.contains_key(id) {
            return Ok(id.into());
        }
        let migration = Self::migrations()?
            .into_iter()
            .find(|m| m.legacy_name == id)
            .ok_or_else(|| format!("unknown slot {id}"))?;
        if migration.action != "alias" {
            return Err(format!(
                "{}: {}; suggested slot: {}",
                migration.action,
                migration.reason,
                migration.canonical_slot.as_deref().unwrap_or("none")
            ));
        }
        migration
            .canonical_slot
            .ok_or("missing alias destination".into())
    }
    pub fn standard() -> Result<Self, String> {
        let catalog: Catalog =
            serde_json::from_str(include_str!("../../../schemas/pipeline/catalog-v2.json"))
                .map_err(|e| e.to_string())?;
        let mut registry = Self {
            slots: BTreeMap::new(),
            schemas: BTreeMap::new(),
            validators: BTreeMap::new(),
            operators: BTreeMap::new(),
        };
        for schema in catalog.schemas {
            registry.register_schema(schema)?;
        }
        for slot in catalog.slots {
            registry.check_ports(&slot.inputs)?;
            registry.check_ports(&slot.outputs)?;
            if registry.slots.insert(slot.slot_id.clone(), slot).is_some() {
                return Err("duplicate slot".into());
            }
        }
        Ok(registry)
    }
    fn check_ports(&self, ports: &[PortDescriptor]) -> Result<(), String> {
        let mut names = BTreeSet::new();
        for p in ports {
            if p.name.is_empty() || !names.insert(&p.name) || p.schemas.is_empty() {
                return Err("invalid/duplicate port".into());
            }
            for s in &p.schemas {
                if !self.schemas.contains_key(&(s.name.clone(), s.version)) {
                    return Err(format!("unknown schema {}/{}", s.name, s.version));
                }
            }
        }
        Ok(())
    }
    pub fn register_schema(&mut self, definition: SchemaDefinition) -> Result<(), String> {
        let key = (definition.name.clone(), definition.version);
        if self.schemas.contains_key(&key) {
            return Err("schema identity is immutable".into());
        }
        let validator = jsonschema::validator_for(&definition.schema).map_err(|e| e.to_string())?;
        self.schemas.insert(key.clone(), definition);
        self.validators.insert(key, validator);
        Ok(())
    }
    pub fn schema(&self, s: &SchemaRef) -> Result<&SchemaDefinition, String> {
        self.schemas
            .get(&(s.name.clone(), s.version))
            .ok_or_else(|| "unknown schema".into())
    }
    pub fn validate(&self, s: &SchemaRef, value: &Value) -> Result<(), String> {
        self.validators
            .get(&(s.name.clone(), s.version))
            .ok_or("unknown schema")?
            .validate(value)
            .map_err(|e| e.to_string())
    }
    pub fn register_operator(&mut self, operator: OperatorDescriptor) -> Result<(), String> {
        if self.operators.contains_key(&operator.operator_id) {
            return Err("operator ID already registered".into());
        }
        self.check_ports(&operator.inputs)?;
        self.check_ports(&operator.outputs)?;
        jsonschema::validator_for(&operator.config_schema).map_err(|e| e.to_string())?;
        for id in &operator.slots {
            let slot = self
                .slots
                .get(id)
                .ok_or_else(|| format!("unknown slot {id}"))?;
            if operator.inputs != slot.inputs || operator.outputs != slot.outputs {
                return Err(format!("incompatible ports for {id}"));
            }
        }
        for id in &operator.slots {
            let slot = self.slots.get_mut(id).expect("validated slot");
            slot.providers.push(operator.operator_id.clone());
            slot.state = "bound".into();
            slot.reason = None;
        }
        self.operators
            .insert(operator.operator_id.clone(), operator);
        Ok(())
    }
    pub fn unregister_operator(&mut self, id: &str) {
        self.operators.remove(id);
        for slot in self.slots.values_mut() {
            slot.providers.retain(|p| p != id);
            if slot.providers.is_empty() {
                slot.state = "unbound".into();
                slot.reason = Some("没有可用的算法实现".into());
            }
        }
    }
    pub fn slots(&self) -> Vec<SlotState> {
        self.slots.values().cloned().collect()
    }
    pub fn schemas(&self) -> Vec<SchemaDefinition> {
        self.schemas.values().cloned().collect()
    }
    pub fn register_input_adapters(&mut self) -> Result<(), String> {
        for schema in self.schemas() {
            let slot_id = format!("runtime.input.{}", schema.name);
            let outputs = vec![PortDescriptor {
                name: "value".into(),
                schemas: vec![SchemaRef {
                    name: schema.name.clone(),
                    version: schema.version,
                }],
                required: true,
                multiple: false,
            }];
            self.slots.insert(
                slot_id.clone(),
                SlotState {
                    slot_id: slot_id.clone(),
                    name: format!("Host {} input", schema.name),
                    state: "unbound".into(),
                    reason: None,
                    inputs: vec![],
                    outputs: outputs.clone(),
                    providers: vec![],
                },
            );
            let mut variants = vec![
                serde_json::json!({"type":"object","properties":{"handle":{"type":"string","minLength":1}},"required":["handle"],"additionalProperties":false}),
                serde_json::json!({"type":"object","properties":{"value":schema.schema},"required":["value"],"additionalProperties":false}),
            ];
            if matches!(schema.name.as_str(), "SegmentView" | "TextView") {
                variants.push(serde_json::json!({"type":"object","properties":{"view":{"enum":["source","target"]}},"required":["view"],"additionalProperties":false}));
            }
            self.register_operator(OperatorDescriptor {
                operator_id: format!("host.input.{}", schema.name),
                release: "1".into(),
                name: format!("Host {} input", schema.name),
                slots: vec![slot_id],
                inputs: vec![],
                outputs,
                config_schema: serde_json::json!({"oneOf":variants}),
            })?;
        }
        Ok(())
    }
    pub fn operators(&self) -> Vec<OperatorDescriptor> {
        self.operators.values().cloned().collect()
    }
    pub fn slot(&self, id: &str) -> Option<&SlotState> {
        self.slots.get(id)
    }
    pub fn operator(&self, id: &str) -> Option<&OperatorDescriptor> {
        self.operators.get(id)
    }

    /// Returns a deterministic topological order. All edge semantics are checked before execution.
    pub fn compile(&self, plan: &PlanV2) -> Result<Vec<String>, String> {
        if plan.format_version != 2
            || plan.nodes.is_empty()
            || plan.nodes.len() > 128
            || plan.outputs.is_empty()
        {
            return Err("unsupported or empty plan".into());
        }
        let nodes: BTreeMap<_, _> = plan.nodes.iter().map(|n| (n.node_id.as_str(), n)).collect();
        if nodes.len() != plan.nodes.len() {
            return Err("duplicate node".into());
        }
        for node in &plan.nodes {
            if uuid::Uuid::parse_str(&node.node_id).is_err() {
                return Err("node ID must be opaque UUID".into());
            }
            let op = self
                .operator(&node.operator_id)
                .ok_or_else(|| format!("unbound operator {}", node.operator_id))?;
            if !op.slots.contains(&self.resolve_slot(&node.slot_id)?) {
                return Err("operator does not provide requested slot".into());
            }
            jsonschema::validator_for(&op.config_schema)
                .map_err(|e| e.to_string())?
                .validate(&node.config)
                .map_err(|e| e.to_string())?;
            for name in node.inputs.keys() {
                if !op.inputs.iter().any(|p| &p.name == name) {
                    return Err(format!("unknown input {name}"));
                }
            }
            for port in &op.inputs {
                let edges = node
                    .inputs
                    .get(&port.name)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                if (port.required && edges.is_empty()) || (!port.multiple && edges.len() > 1) {
                    return Err(format!("invalid cardinality for {}", port.name));
                }
                for edge in edges {
                    let upstream = nodes
                        .get(edge.node_id.as_str())
                        .ok_or("unknown upstream node")?;
                    let provider = self
                        .operator(&upstream.operator_id)
                        .ok_or("unknown upstream operator")?;
                    let output = provider
                        .outputs
                        .iter()
                        .find(|p| p.name == edge.port)
                        .ok_or("unknown output port")?;
                    if !output.schemas.iter().all(|s| port.schemas.contains(s)) {
                        return Err("incompatible edge schema".into());
                    }
                }
            }
        }
        for out in &plan.outputs {
            let n = nodes
                .get(out.node_id.as_str())
                .ok_or("unknown plan output")?;
            if !self
                .operator(&n.operator_id)
                .expect("validated provider")
                .outputs
                .iter()
                .any(|p| p.name == out.port)
            {
                return Err("unknown plan output port".into());
            }
        }
        let mut reachable = BTreeSet::new();
        let mut pending: Vec<_> = plan.outputs.iter().map(|p| p.node_id.as_str()).collect();
        while let Some(id) = pending.pop() {
            if reachable.insert(id) {
                pending.extend(
                    nodes[id]
                        .inputs
                        .values()
                        .flatten()
                        .map(|edge| edge.node_id.as_str()),
                );
            }
        }
        if reachable.len() != nodes.len() {
            return Err("plan contains nodes unreachable from its outputs".into());
        }
        let mut order = Vec::new();
        let mut done = BTreeSet::new();
        while order.len() < nodes.len() {
            let before = order.len();
            for (id, n) in &nodes {
                if !done.contains(id)
                    && n.inputs
                        .values()
                        .flatten()
                        .all(|e| done.contains(e.node_id.as_str()))
                {
                    order.push((*id).to_owned());
                    done.insert(*id);
                }
            }
            if before == order.len() {
                return Err("cyclic dependency".into());
            }
        }
        Ok(order)
    }
}
