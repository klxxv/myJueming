use crate::{
    CancellationToken, ChineseTokenizerConfig, CreatePipelineMethod, DerivedToken,
    ExecutePipelineRequest, NormalizeConfig, PipelineArtifactId, PipelineArtifactSummary,
    PipelineError, PipelineExecution, PipelineMethod, PipelineMethodId, PipelineMethodRevision,
    PipelineMethodRevisionId, PipelineMethodRevisionSummary, PipelineMethodSummary, PipelineNode,
    PipelineNodeId, PipelinePlanSnapshot, TokenArtifact, TokenizerProvenance, UpdatePipelineMethod,
};
use jieba_rs::Jieba;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

const MANIFEST_VERSION: &str = "1.0";
const JIEBA_RS_VERSION: &str = "0.10.3";

/// Thread-safe, project-local service for non-canonical Pipeline data.
pub struct PipelineService {
    root: PathBuf,
    state: Mutex<PipelineManifest>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct PipelineManifest {
    format_version: String,
    #[serde(default)]
    methods: BTreeMap<PipelineMethodId, MethodRecord>,
    #[serde(default)]
    artifacts: BTreeMap<PipelineArtifactId, ArtifactRecord>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct MethodRecord {
    name: String,
    current_method_revision_id: PipelineMethodRevisionId,
    revisions: Vec<PipelineMethodRevisionId>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ArtifactRecord {
    method_id: PipelineMethodId,
    method_revision_id: PipelineMethodRevisionId,
    #[serde(default)]
    input_revision_id: jueming_core::RevisionId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SlotType {
    Text,
    Tokens,
    Artifact,
}

impl SlotType {
    const fn label(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Tokens => "tokens",
            Self::Artifact => "artifact",
        }
    }
}

impl PipelineService {
    pub fn open(project_root: impl AsRef<Path>) -> Result<Self, PipelineError> {
        let root = project_root.as_ref().to_path_buf();
        let valid_extension = root
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("jm"));
        if !valid_extension {
            return Err(PipelineError::InvalidProjectRoot(root));
        }

        let manifest_path = manifest_path(&root);
        let state = if manifest_path.exists() {
            let manifest: PipelineManifest = read_json(&manifest_path)?;
            if manifest.format_version != MANIFEST_VERSION {
                return Err(PipelineError::UnsupportedManifest(manifest.format_version));
            }
            manifest
        } else {
            PipelineManifest {
                format_version: MANIFEST_VERSION.into(),
                ..PipelineManifest::default()
            }
        };
        Ok(Self {
            root,
            state: Mutex::new(state),
        })
    }

    pub fn list_methods(&self) -> Result<Vec<PipelineMethodSummary>, PipelineError> {
        let state = self.state.lock().expect("pipeline state lock poisoned");
        Ok(state
            .methods
            .iter()
            .map(|(method_id, record)| PipelineMethodSummary {
                method_id: *method_id,
                name: record.name.clone(),
                current_method_revision_id: record.current_method_revision_id,
                revision_count: record.revisions.len() as u32,
            })
            .collect())
    }

    pub fn load_method(
        &self,
        method_id: PipelineMethodId,
    ) -> Result<PipelineMethod, PipelineError> {
        let state = self.state.lock().expect("pipeline state lock poisoned");
        let record = state
            .methods
            .get(&method_id)
            .ok_or(PipelineError::MethodNotFound(method_id))?;
        let current =
            read_method_revision(&self.root, method_id, record.current_method_revision_id)?;
        let revisions = record
            .revisions
            .iter()
            .map(|revision_id| {
                let revision = read_method_revision(&self.root, method_id, *revision_id)?;
                Ok(PipelineMethodRevisionSummary {
                    method_revision_id: revision.method_revision_id,
                    parent_method_revision_id: revision.parent_method_revision_id,
                })
            })
            .collect::<Result<Vec<_>, PipelineError>>()?;
        Ok(PipelineMethod {
            method_id,
            name: record.name.clone(),
            current,
            revisions,
        })
    }

    /// Loads one immutable method revision, including revisions that are no
    /// longer current.  Hosts use this to reconcile a durably reserved commit
    /// ID after an interrupted proposal approval.
    pub fn load_method_revision(
        &self,
        method_id: PipelineMethodId,
        method_revision_id: PipelineMethodRevisionId,
    ) -> Result<PipelineMethodRevision, PipelineError> {
        let state = self.state.lock().expect("pipeline state lock poisoned");
        let record = state
            .methods
            .get(&method_id)
            .ok_or(PipelineError::MethodNotFound(method_id))?;
        if !record.revisions.contains(&method_revision_id) {
            return Err(PipelineError::MethodRevisionNotFound(method_revision_id));
        }
        read_method_revision(&self.root, method_id, method_revision_id)
    }

    pub fn create_method(
        &self,
        request: CreatePipelineMethod,
    ) -> Result<PipelineMethod, PipelineError> {
        let name = validate_name(request.name)?;
        let method_id = PipelineMethodId::new();
        let method_revision_id = PipelineMethodRevisionId::new();
        let plan = PipelinePlanSnapshot {
            method_id,
            method_revision_id,
            nodes: request.plan.nodes,
            output_node_id: request.plan.output_node_id,
        };
        validate_plan(&plan)?;
        let revision = PipelineMethodRevision {
            method_id,
            method_revision_id,
            parent_method_revision_id: None,
            method_name: name.clone(),
            plan,
        };

        let mut state = self.state.lock().expect("pipeline state lock poisoned");
        write_method_revision(&self.root, &revision)?;
        let mut next_state = state.clone();
        next_state.methods.insert(
            method_id,
            MethodRecord {
                name: name.clone(),
                current_method_revision_id: method_revision_id,
                revisions: vec![method_revision_id],
            },
        );
        persist_manifest(&self.root, &next_state)?;
        *state = next_state;
        Ok(PipelineMethod {
            method_id,
            name,
            current: revision.clone(),
            revisions: vec![PipelineMethodRevisionSummary {
                method_revision_id,
                parent_method_revision_id: None,
            }],
        })
    }

    pub fn update_method(
        &self,
        request: UpdatePipelineMethod,
    ) -> Result<PipelineMethod, PipelineError> {
        self.update_method_with_revision_id(request, PipelineMethodRevisionId::new())
    }

    /// Appends an update under a host-reserved UUIDv7 revision ID.  This is
    /// intentionally a service API rather than a caller-supplied gateway
    /// field: a trusted host persists the ID before beginning an approval so a
    /// crash can be reconciled without inferring intent from matching content.
    pub fn update_method_with_revision_id(
        &self,
        request: UpdatePipelineMethod,
        method_revision_id: PipelineMethodRevisionId,
    ) -> Result<PipelineMethod, PipelineError> {
        if method_revision_id.as_uuid().get_version_num() != 7 {
            return Err(PipelineError::InvalidUuidV7 {
                kind: "method revision",
                value: method_revision_id.to_string(),
            });
        }
        let mut state = self.state.lock().expect("pipeline state lock poisoned");
        let record = state
            .methods
            .get(&request.method_id)
            .ok_or(PipelineError::MethodNotFound(request.method_id))?
            .clone();
        if record.revisions.contains(&method_revision_id) {
            return Err(PipelineError::MethodRevisionAlreadyExists(
                method_revision_id,
            ));
        }
        if record.current_method_revision_id != request.base_method_revision_id {
            return Err(PipelineError::StaleMethodRevision {
                method_id: request.method_id,
                expected: request.base_method_revision_id,
                actual: record.current_method_revision_id,
            });
        }
        let plan = PipelinePlanSnapshot {
            method_id: request.method_id,
            method_revision_id,
            nodes: request.plan.nodes,
            output_node_id: request.plan.output_node_id,
        };
        validate_plan(&plan)?;
        let new_name = match request.name {
            Some(name) => validate_name(name)?,
            None => record.name.clone(),
        };
        let revision = PipelineMethodRevision {
            method_id: request.method_id,
            method_revision_id,
            parent_method_revision_id: Some(record.current_method_revision_id),
            method_name: new_name.clone(),
            plan,
        };
        write_method_revision(&self.root, &revision)?;
        let mut next_state = state.clone();
        next_state.methods.insert(
            request.method_id,
            MethodRecord {
                name: new_name.clone(),
                current_method_revision_id: method_revision_id,
                revisions: record
                    .revisions
                    .into_iter()
                    .chain(std::iter::once(method_revision_id))
                    .collect(),
            },
        );
        persist_manifest(&self.root, &next_state)?;
        *state = next_state;
        drop(state);
        self.load_method(request.method_id)
    }

    pub fn execute(
        &self,
        request: ExecutePipelineRequest,
        cancellation: &CancellationToken,
    ) -> Result<PipelineExecution, PipelineError> {
        check_cancelled(cancellation)?;
        let revision = {
            let state = self.state.lock().expect("pipeline state lock poisoned");
            let record = state
                .methods
                .get(&request.method_id)
                .ok_or(PipelineError::MethodNotFound(request.method_id))?;
            let revision_id = request
                .method_revision_id
                .unwrap_or(record.current_method_revision_id);
            if !record.revisions.contains(&revision_id) {
                return Err(PipelineError::MethodRevisionNotFound(revision_id));
            }
            read_method_revision(&self.root, request.method_id, revision_id)?
        };
        validate_plan(&revision.plan)?;
        let artifact = execute_plan(&revision, &request, cancellation)?;
        check_cancelled(cancellation)?;

        let mut state = self.state.lock().expect("pipeline state lock poisoned");
        write_artifact(&self.root, &artifact)?;
        // An artifact file without a manifest record is unreachable. Do not
        // publish it if cancellation races with the last atomic write.
        check_cancelled(cancellation)?;
        let mut next_state = state.clone();
        next_state.artifacts.insert(
            artifact.artifact_id,
            ArtifactRecord {
                method_id: artifact.method_id,
                method_revision_id: artifact.method_revision_id,
                input_revision_id: artifact.input_revision_id,
            },
        );
        persist_manifest(&self.root, &next_state)?;
        *state = next_state;
        Ok(PipelineExecution { artifact })
    }

    /// Finds derived artifacts associated with the selected canonical revision.
    pub fn list_artifacts_for_revision(
        &self,
        input_revision_id: jueming_core::RevisionId,
    ) -> Result<Vec<PipelineArtifactSummary>, PipelineError> {
        let state = self.state.lock().expect("pipeline state lock poisoned");
        let mut artifacts = Vec::new();
        for (artifact_id, record) in &state.artifacts {
            if record.input_revision_id != input_revision_id {
                continue;
            }
            let artifact: TokenArtifact = read_json(&artifact_path(&self.root, *artifact_id))?;
            if artifact.input_revision_id == input_revision_id {
                artifacts.push(PipelineArtifactSummary {
                    artifact_id: *artifact_id,
                    input_revision_id: artifact.input_revision_id,
                    segment_id: artifact.segment_id,
                    method_id: artifact.method_id,
                    method_revision_id: artifact.method_revision_id,
                });
            }
        }
        Ok(artifacts)
    }

    pub fn load_artifact(
        &self,
        artifact_id: PipelineArtifactId,
    ) -> Result<TokenArtifact, PipelineError> {
        let state = self.state.lock().expect("pipeline state lock poisoned");
        let record = state
            .artifacts
            .get(&artifact_id)
            .ok_or(PipelineError::ArtifactNotFound(artifact_id))?;
        let artifact: TokenArtifact = read_json(&artifact_path(&self.root, artifact_id))?;
        if artifact.method_id != record.method_id
            || artifact.method_revision_id != record.method_revision_id
        {
            return Err(PipelineError::ArtifactProvenanceMismatch {
                artifact_id,
                method_revision_id: record.method_revision_id,
            });
        }
        Ok(artifact)
    }
}

fn validate_name(name: String) -> Result<String, PipelineError> {
    let name = name.trim().to_owned();
    if name.is_empty() {
        Err(PipelineError::BlankMethodName)
    } else {
        Ok(name)
    }
}

fn validate_plan(plan: &PipelinePlanSnapshot) -> Result<(), PipelineError> {
    let mut by_id = HashMap::new();
    for node in &plan.nodes {
        if node.node_id.as_uuid().get_version_num() != 7 {
            return Err(PipelineError::InvalidUuidV7 {
                kind: "node",
                value: node.node_id.to_string(),
            });
        }
        if by_id.insert(node.node_id, node).is_some() {
            return Err(PipelineError::DuplicateNode(node.node_id));
        }
        operator_slot(node)?;
    }
    if plan.method_id.as_uuid().get_version_num() != 7
        || plan.method_revision_id.as_uuid().get_version_num() != 7
    {
        return Err(PipelineError::InvalidUuidV7 {
            kind: "method or method revision",
            value: format!("{}/{}", plan.method_id, plan.method_revision_id),
        });
    }
    for node in &plan.nodes {
        let (expected_input, output) = operator_slot(node)?;
        if node.inputs.len() != expected_input.is_some() as usize {
            return Err(PipelineError::InvalidInputCount {
                node_id: node.node_id,
                expected: expected_input.is_some() as usize,
                actual: node.inputs.len(),
            });
        }
        let _ = output;
        if let Some(expected) = expected_input {
            let upstream_id = node.inputs[0];
            let upstream = by_id.get(&upstream_id).ok_or(PipelineError::UnknownNode {
                node_id: upstream_id,
                from_node_id: node.node_id,
            })?;
            let (_, actual) = operator_slot(upstream)?;
            if actual != expected {
                return Err(PipelineError::SlotMismatch {
                    node_id: node.node_id,
                    from_node_id: upstream_id,
                    expected: expected.label(),
                    actual: actual.label(),
                });
            }
        }
    }
    if operator_slot(
        by_id
            .get(&plan.output_node_id)
            .ok_or(PipelineError::UnknownNode {
                node_id: plan.output_node_id,
                from_node_id: plan.output_node_id,
            })?,
    )?
    .1 != SlotType::Artifact
    {
        return Err(PipelineError::InvalidOutputNode(plan.output_node_id));
    }
    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    for node in &plan.nodes {
        visit(node.node_id, &by_id, &mut visiting, &mut visited)?;
    }
    Ok(())
}

fn visit(
    node_id: PipelineNodeId,
    by_id: &HashMap<PipelineNodeId, &PipelineNode>,
    visiting: &mut HashSet<PipelineNodeId>,
    visited: &mut HashSet<PipelineNodeId>,
) -> Result<(), PipelineError> {
    if visited.contains(&node_id) {
        return Ok(());
    }
    if !visiting.insert(node_id) {
        return Err(PipelineError::Cycle(node_id));
    }
    let node = by_id[&node_id];
    for input in &node.inputs {
        visit(*input, by_id, visiting, visited)?;
    }
    visiting.remove(&node_id);
    visited.insert(node_id);
    Ok(())
}

fn operator_slot(node: &PipelineNode) -> Result<(Option<SlotType>, SlotType), PipelineError> {
    match node.operator.as_str() {
        "source" => Ok((None, SlotType::Text)),
        "normalize" => {
            parse_config::<NormalizeConfig>(node)?;
            Ok((Some(SlotType::Text), SlotType::Text))
        }
        "chinese_tokenize" => {
            parse_config::<ChineseTokenizerConfig>(node)?;
            Ok((Some(SlotType::Text), SlotType::Tokens))
        }
        "artifact" => {
            if !node.config.is_object() {
                return Err(invalid_config(node, "artifact config must be an object"));
            }
            Ok((Some(SlotType::Tokens), SlotType::Artifact))
        }
        operator => Err(PipelineError::UnknownOperator {
            node_id: node.node_id,
            operator: operator.into(),
        }),
    }
}

fn parse_config<T: DeserializeOwned>(node: &PipelineNode) -> Result<T, PipelineError> {
    serde_json::from_value(node.config.clone())
        .map_err(|error| invalid_config(node, error.to_string()))
}

fn invalid_config(node: &PipelineNode, message: impl Into<String>) -> PipelineError {
    PipelineError::InvalidOperatorConfig {
        node_id: node.node_id,
        operator: node.operator.clone(),
        message: message.into(),
    }
}

fn execute_plan(
    revision: &PipelineMethodRevision,
    request: &ExecutePipelineRequest,
    cancellation: &CancellationToken,
) -> Result<TokenArtifact, PipelineError> {
    let by_id: HashMap<_, _> = revision
        .plan
        .nodes
        .iter()
        .map(|node| (node.node_id, node))
        .collect();
    let mut values = HashMap::<PipelineNodeId, RuntimeValue>::new();
    let mut remaining = revision.plan.nodes.len();
    while values.len() < revision.plan.nodes.len() {
        check_cancelled(cancellation)?;
        if remaining == 0 {
            return Err(PipelineError::Cycle(revision.plan.output_node_id));
        }
        remaining -= 1;
        for node in &revision.plan.nodes {
            if values.contains_key(&node.node_id)
                || node.inputs.iter().any(|id| !values.contains_key(id))
            {
                continue;
            }
            let value = match node.operator.as_str() {
                "source" => RuntimeValue::Text(request.source_content.clone()),
                "normalize" => {
                    let config: NormalizeConfig = parse_config(node)?;
                    let input = values[&node.inputs[0]].text()?.to_owned();
                    RuntimeValue::Text(normalize(input, &config))
                }
                "chinese_tokenize" => {
                    let config: ChineseTokenizerConfig = parse_config(node)?;
                    let input = values[&node.inputs[0]].text()?.to_owned();
                    RuntimeValue::Tokens {
                        tokens: tokenize(&input, &config, cancellation)?,
                        normalized_content: input,
                    }
                }
                "artifact" => RuntimeValue::Artifact,
                _ => {
                    return Err(PipelineError::UnknownOperator {
                        node_id: node.node_id,
                        operator: node.operator.clone(),
                    });
                }
            };
            values.insert(node.node_id, value);
        }
    }
    let artifact_node = by_id[&revision.plan.output_node_id];
    let token_node_id = artifact_node.inputs[0];
    let (tokens, normalized_content) = values
        .remove(&token_node_id)
        .expect("validated token input")
        .tokens()?;
    let tokenizer_node = by_id[&token_node_id];
    let tokenizer_config: ChineseTokenizerConfig = parse_config(tokenizer_node)?;
    Ok(TokenArtifact {
        artifact_id: PipelineArtifactId::new(),
        artifact_format: "jueming.pipeline.token-artifact.v1".into(),
        input_revision_id: request.input_revision_id,
        segment_id: request.segment_id,
        source_content_sha256: digest(&request.source_content),
        normalized_content: normalized_content.clone(),
        normalized_content_sha256: digest(&normalized_content),
        method_id: revision.method_id,
        method_revision_id: revision.method_revision_id,
        tokenizer: TokenizerProvenance {
            implementation: "jieba-rs".into(),
            version: JIEBA_RS_VERSION.into(),
            hmm: tokenizer_config.hmm,
            custom_dictionary_sha256: dictionary_digest(&tokenizer_config),
        },
        tokens,
    })
}

enum RuntimeValue {
    Text(String),
    Tokens {
        tokens: Vec<DerivedToken>,
        normalized_content: String,
    },
    Artifact,
}
impl RuntimeValue {
    fn text(&self) -> Result<&str, PipelineError> {
        if let Self::Text(value) = self {
            Ok(value)
        } else {
            Err(PipelineError::InvalidInput {
                revision_id: jueming_core::RevisionId::new(0),
                segment_id: jueming_core::SegmentId::new(),
                message: "expected text pipeline value".into(),
            })
        }
    }
    fn tokens(self) -> Result<(Vec<DerivedToken>, String), PipelineError> {
        if let Self::Tokens {
            tokens,
            normalized_content,
        } = self
        {
            Ok((tokens, normalized_content))
        } else {
            Err(PipelineError::InvalidInput {
                revision_id: jueming_core::RevisionId::new(0),
                segment_id: jueming_core::SegmentId::new(),
                message: "expected token pipeline value".into(),
            })
        }
    }
}

fn normalize(value: String, config: &NormalizeConfig) -> String {
    let mut value = value;
    if config.trim {
        value = value.trim().to_owned();
    }
    if config.collapse_whitespace {
        value = value.split_whitespace().collect::<Vec<_>>().join(" ");
    }
    value
}

fn tokenize(
    value: &str,
    config: &ChineseTokenizerConfig,
    cancellation: &CancellationToken,
) -> Result<Vec<DerivedToken>, PipelineError> {
    let mut tokenizer = Jieba::new();
    let mut entries = config.custom_dictionary.clone();
    entries.sort_by(|left, right| {
        (left.word.as_str(), left.frequency, left.tag.as_deref()).cmp(&(
            right.word.as_str(),
            right.frequency,
            right.tag.as_deref(),
        ))
    });
    let mut words = HashSet::new();
    for entry in &entries {
        check_cancelled(cancellation)?;
        if entry.word.trim().is_empty() || !words.insert(entry.word.as_str()) {
            return Err(PipelineError::InvalidInput {
                revision_id: jueming_core::RevisionId::new(0),
                segment_id: jueming_core::SegmentId::new(),
                message: "custom dictionary words must be nonblank and unique".into(),
            });
        }
        tokenizer.add_word(&entry.word, entry.frequency, entry.tag.as_deref());
    }
    let mut tokens = Vec::new();
    for word in tokenizer.cut(value, config.hmm) {
        check_cancelled(cancellation)?;
        tokens.push(DerivedToken {
            text: word.word.into(),
            start_utf8: word.byte_start as u64,
            end_utf8: word.byte_end as u64,
        });
    }
    Ok(tokens)
}

fn dictionary_digest(config: &ChineseTokenizerConfig) -> String {
    let mut entries = config.custom_dictionary.clone();
    entries.sort_by(|left, right| {
        (left.word.as_str(), left.frequency, left.tag.as_deref()).cmp(&(
            right.word.as_str(),
            right.frequency,
            right.tag.as_deref(),
        ))
    });
    digest(&serde_json::to_string(&entries).expect("dictionary entries are serializable"))
}

fn digest(value: &str) -> String {
    Sha256::digest(value.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn check_cancelled(cancellation: &CancellationToken) -> Result<(), PipelineError> {
    if cancellation.is_cancelled() {
        Err(PipelineError::Cancelled)
    } else {
        Ok(())
    }
}

fn extensions_dir(root: &Path) -> PathBuf {
    root.join("extensions").join("pipeline")
}
fn manifest_path(root: &Path) -> PathBuf {
    extensions_dir(root).join("manifest-v1.json")
}
fn method_path(
    root: &Path,
    method_id: PipelineMethodId,
    revision_id: PipelineMethodRevisionId,
) -> PathBuf {
    extensions_dir(root)
        .join("methods")
        .join(method_id.to_string())
        .join(format!("{revision_id}.json"))
}
fn artifact_path(root: &Path, artifact_id: PipelineArtifactId) -> PathBuf {
    extensions_dir(root)
        .join("artifacts")
        .join(format!("{artifact_id}.json"))
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, PipelineError> {
    let text = fs::read_to_string(path).map_err(|source| PipelineError::Io {
        path: path.into(),
        source,
    })?;
    serde_json::from_str(&text).map_err(|source| PipelineError::Json {
        path: path.into(),
        source,
    })
}
fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), PipelineError> {
    let bytes = serde_json::to_vec_pretty(value).expect("pipeline values are serializable");
    jueming_storage::write_bytes_atomic(path, &bytes).map_err(PipelineError::from)
}
fn write_method_revision(
    root: &Path,
    revision: &PipelineMethodRevision,
) -> Result<(), PipelineError> {
    write_json(
        &method_path(root, revision.method_id, revision.method_revision_id),
        revision,
    )
}
fn read_method_revision(
    root: &Path,
    method_id: PipelineMethodId,
    revision_id: PipelineMethodRevisionId,
) -> Result<PipelineMethodRevision, PipelineError> {
    let path = method_path(root, method_id, revision_id);
    if !path.exists() {
        return Err(PipelineError::MethodRevisionNotFound(revision_id));
    }
    read_json(&path)
}
fn write_artifact(root: &Path, artifact: &TokenArtifact) -> Result<(), PipelineError> {
    write_json(&artifact_path(root, artifact.artifact_id), artifact)
}
fn persist_manifest(root: &Path, state: &PipelineManifest) -> Result<(), PipelineError> {
    write_json(&manifest_path(root), state)
}
