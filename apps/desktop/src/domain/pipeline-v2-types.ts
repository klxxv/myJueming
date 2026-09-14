import type { ResearchRun, SchemaRef } from "./research-types";

export interface PortRefV2 { node_id: string; port: string }
export interface PlanNodeV2 {
  node_id: string;
  slot_id: string;
  operator_id: string;
  inputs: Record<string, PortRefV2[]>;
  config: Record<string, unknown>;
}
export interface PlanV2 { format_version: 2; nodes: PlanNodeV2[]; outputs: PortRefV2[] }
export interface PipelineMethodV2 { method_id: string; name: string; method_revision_id: string; plan: PlanV2 }
export interface ArtifactManifestV2 {
  handle: string; project_id: string; input_revision_id: string; run_id: string;
  schema: SchemaRef; sha256: string; bytes: number;
  provider_id: string; provider_release: string; dependencies: string[];
}
export interface GraphRunV2 extends ResearchRun { artifacts: ArtifactManifestV2[] }
export interface SchemaDefinitionV2 extends SchemaRef { schema: Record<string, unknown> }
