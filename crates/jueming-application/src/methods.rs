//! One discovery/policy table for the local research control plane.
use serde::Serialize;
#[derive(Clone, Copy, Debug, Serialize)]
pub struct MethodDescriptor {
    pub method: &'static str,
    pub requires_binding: bool,
    pub native_only: bool,
    pub description: &'static str,
}
pub const RESEARCH_METHODS: &[MethodDescriptor] = &[
    MethodDescriptor {
        method: "plugins.remove_local",
        requires_binding: false,
        native_only: true,
        description: "Unregister a separately installed local package while retaining historical artifacts",
    },
    MethodDescriptor {
        method: "slots.migrations",
        requires_binding: false,
        native_only: false,
        description: "Discover explicit legacy aliases and names requiring a new provider choice",
    },
    MethodDescriptor {
        method: "pool.read_index_partition",
        requires_binding: true,
        native_only: false,
        description: "Read at most 200 term postings from a snapshot-bound index artifact",
    },
    MethodDescriptor {
        method: "pool.collect_cache",
        requires_binding: true,
        native_only: true,
        description: "Collect unreachable rebuildable payloads while preserving view, method and run references",
    },
    MethodDescriptor {
        method: "pool.open_view",
        requires_binding: true,
        native_only: false,
        description: "Open a scoped view pinned to a canonical revision",
    },
    MethodDescriptor {
        method: "pool.read_batch",
        requires_binding: true,
        native_only: false,
        description: "Pull at most 200 rows and publish a typed immutable batch",
    },
    MethodDescriptor {
        method: "pool.read_slice",
        requires_binding: true,
        native_only: false,
        description: "Read a bounded snapshot slice",
    },
    MethodDescriptor {
        method: "pool.read_segment",
        requires_binding: true,
        native_only: false,
        description: "Read a stable Segment from a frozen view",
    },
    MethodDescriptor {
        method: "pool.read_alignment",
        requires_binding: true,
        native_only: false,
        description: "Read an n:m alignment from a frozen view",
    },
    MethodDescriptor {
        method: "pool.read_annotation",
        requires_binding: true,
        native_only: false,
        description: "Read a human annotation from a frozen view",
    },
    MethodDescriptor {
        method: "pool.close_view",
        requires_binding: true,
        native_only: false,
        description: "Close a scoped view and release its lease",
    },
    MethodDescriptor {
        method: "plugins.install_local",
        requires_binding: false,
        native_only: true,
        description: "Install a verified local algorithm package directory",
    },
    MethodDescriptor {
        method: "research.summary",
        requires_binding: true,
        native_only: false,
        description: "Read full-run group distribution and review coverage",
    },
    MethodDescriptor {
        method: "research.merge_groups",
        requires_binding: true,
        native_only: true,
        description: "Merge human surface-form groups in one canonical revision",
    },
    MethodDescriptor {
        method: "pipeline.list_methods_v2",
        requires_binding: true,
        native_only: false,
        description: "List versioned named-port methods",
    },
    MethodDescriptor {
        method: "pipeline.save_method_v2",
        requires_binding: true,
        native_only: true,
        description: "Save a validated immutable method revision",
    },
    MethodDescriptor {
        method: "pipeline.start",
        requires_binding: true,
        native_only: false,
        description: "Start asynchronous v2 graph execution",
    },
    MethodDescriptor {
        method: "pipeline.get_graph_run",
        requires_binding: true,
        native_only: false,
        description: "Read generic graph status and output manifests",
    },
    MethodDescriptor {
        method: "pipeline.cancel_graph_run",
        requires_binding: true,
        native_only: false,
        description: "Cancel generic graph execution",
    },
    MethodDescriptor {
        method: "capabilities.get",
        requires_binding: false,
        native_only: false,
        description: "Read live feature, slot and operator availability",
    },
    MethodDescriptor {
        method: "slots.list",
        requires_binding: false,
        native_only: false,
        description: "List registered slot contracts",
    },
    MethodDescriptor {
        method: "schemas.list",
        requires_binding: false,
        native_only: false,
        description: "List registered JSON Schemas",
    },
    MethodDescriptor {
        method: "schemas.get",
        requires_binding: false,
        native_only: false,
        description: "Read immutable schema by name and version",
    },
    MethodDescriptor {
        method: "operators.list",
        requires_binding: false,
        native_only: false,
        description: "List currently bound operators",
    },
    MethodDescriptor {
        method: "research.start",
        requires_binding: true,
        native_only: false,
        description: "Start asynchronous frozen-snapshot translation research",
    },
    MethodDescriptor {
        method: "pipeline.get_run",
        requires_binding: true,
        native_only: false,
        description: "Read research run status",
    },
    MethodDescriptor {
        method: "pipeline.list_runs",
        requires_binding: true,
        native_only: false,
        description: "List durable project research runs",
    },
    MethodDescriptor {
        method: "pipeline.cancel_run",
        requires_binding: true,
        native_only: false,
        description: "Cancel a research run",
    },
    MethodDescriptor {
        method: "pipeline.read_result",
        requires_binding: true,
        native_only: false,
        description: "Read at most 200 occurrences at a stable cursor",
    },
    MethodDescriptor {
        method: "pipeline.validate_plan",
        requires_binding: true,
        native_only: false,
        description: "Compile typed v2 named-port graph without execution",
    },
    MethodDescriptor {
        method: "pipeline.list_artifacts",
        requires_binding: true,
        native_only: false,
        description: "Discover published schema-checked artifacts",
    },
    MethodDescriptor {
        method: "pipeline.read_artifact",
        requires_binding: true,
        native_only: false,
        description: "Read immutable artifact by opaque handle",
    },
    MethodDescriptor {
        method: "features.enable",
        requires_binding: false,
        native_only: true,
        description: "Prepare official resources and enable feature",
    },
    MethodDescriptor {
        method: "features.disable",
        requires_binding: false,
        native_only: true,
        description: "Stop feature, retain resources and project records",
    },
    MethodDescriptor {
        method: "features.cancel_prepare",
        requires_binding: false,
        native_only: true,
        description: "Cancel resource preparation",
    },
    MethodDescriptor {
        method: "features.retry_prepare",
        requires_binding: false,
        native_only: true,
        description: "Retry resource preparation",
    },
    MethodDescriptor {
        method: "features.update_preferences",
        requires_binding: false,
        native_only: true,
        description: "Save local feature defaults",
    },
    MethodDescriptor {
        method: "research.confirm",
        requires_binding: true,
        native_only: true,
        description: "Commit human judgement through canonical command envelope",
    },
];
pub fn research_method(method: &str) -> Option<&'static MethodDescriptor> {
    RESEARCH_METHODS.iter().find(|d| d.method == method)
}
