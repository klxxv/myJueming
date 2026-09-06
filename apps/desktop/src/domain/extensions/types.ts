export type ExtensionScope = "application" | "project" | "selection";

export interface NavigationContribution {
  id: string;
  label: string;
  location: "primary" | "secondary" | "context";
  order: number;
  enabled?: boolean;
}

export interface ContextProvider {
  id: string;
  label: string;
  scope: ExtensionScope;
  fields: readonly string[];
}

export interface OperatorProvider {
  id: string;
  label: string;
  inputSchema: string;
  outputSchema?: string;
}

export interface ToolProvider {
  id: string;
  label: string;
  access: "read" | "propose";
  inputSchema?: string;
}

export interface ExtensionCapabilities {
  navigation?: readonly NavigationContribution[];
  context?: readonly ContextProvider[];
  operators?: readonly OperatorProvider[];
  tools?: readonly ToolProvider[];
}

export interface ExtensionCapabilitySnapshot {
  navigation: readonly NavigationContribution[];
  context: readonly ContextProvider[];
  operators: readonly OperatorProvider[];
  tools: readonly ToolProvider[];
}

export interface ExtensionRegistration {
  dispose(): void;
}

export interface ExtensionGeneration {
  readonly owner: string;
  readonly generation: number;
  register(capabilities: ExtensionCapabilities): ExtensionRegistration;
  dispose(): void;
}

export interface ExtensionCapabilityRegistry {
  beginGeneration(owner: string): ExtensionGeneration;
  snapshot(): ExtensionCapabilitySnapshot;
  subscribe(listener: (snapshot: ExtensionCapabilitySnapshot) => void): () => void;
  dispose(): void;
  readonly activeSubscriberCount: number;
}
