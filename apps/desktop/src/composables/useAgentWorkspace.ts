import { computed, ref, toValue, watch, type MaybeRefOrGetter } from "vue";
import type { ProjectIdentity } from "../domain/kernel-client";
import { agentClient, type AgentClient } from "../domain/agent-client";
import type {
  AgentClientUpdate,
  AgentNavigationRequest,
  AgentProjection,
  PendingUiAction,
  AgentRevealRequest,
  AgentSessionBinding,
  AgentSubscription,
  AppEvent,
  ContextSnapshot,
  SearchSpec,
} from "../domain/agent-types";
import { useAgentWorkspaceStore } from "../stores/agent-workspace";
import { t, type LocalizedMessage } from "../i18n";
import { formatError } from "../i18n/kernel-messages";

export interface AgentWorkspaceSelection {
  segmentIds: string[];
  alignmentIds: string[];
}

export interface AgentWorkspaceSearchState {
  query: string;
  regex: boolean;
  caseSensitive: boolean;
  languageId: string | null;
}

export interface AgentWorkspaceOptions {
  tab: MaybeRefOrGetter<string>;
  mode: MaybeRefOrGetter<string | null>;
  projectSnapshot: MaybeRefOrGetter<ProjectIdentity | null>;
  selection: MaybeRefOrGetter<AgentWorkspaceSelection>;
  searchState: MaybeRefOrGetter<AgentWorkspaceSearchState>;
  selectionSharingEnabled: MaybeRefOrGetter<boolean>;
  canLeaveDraft: () => boolean;
  navigate: (tab: string) => void | Promise<void>;
  reveal: (request: Omit<AgentRevealRequest, "request_id">) => void | Promise<void>;
  onProjectSnapshot: (snapshot: ProjectIdentity) => void | Promise<void>;
  onSearchState: (state: { spec: SearchSpec | null; results: unknown }) => void | Promise<void>;
  onAppEvent?: (event: AppEvent) => void | Promise<void>;
  /** Reconcile app-scoped state after initial subscription and stream resnapshot. */
  onProjection?: (projection: AgentProjection) => void | Promise<void>;
  client?: AgentClient;
}

interface DomSelectionCapture {
  selectedText: string;
  segmentIds: string[];
  textRange: ContextSnapshot["text_range"];
}

const privateSelector = [
  "[data-agent-context='exclude']",
  "[data-agent-context='private']",
  "[data-agent-private]",
  "input[type='password']",
  "input[autocomplete='current-password']",
  "input[autocomplete='new-password']",
].join(",");

const unique = (ids: string[]) => [...new Set(ids.filter(Boolean))];

const belongsTo = (root: HTMLElement, node: Node) => root === node || root.contains(node.nodeType === Node.ELEMENT_NODE ? node : node.parentNode);

const offsetWithin = (root: HTMLElement, container: Node, offset: number) => {
  const range = document.createRange();
  range.selectNodeContents(root);
  range.setEnd(container, offset);
  return utf16CodeUnitLength(range.toString());
};

/** DOM Range text and Tauri's context contract both use JavaScript UTF-16 offsets. */
export const utf16CodeUnitLength = (text: string) => text.length;

const segmentForNode = (node: Node | null) => {
  const element = node instanceof Element ? node : node?.parentElement;
  return element?.closest<HTMLElement>("[data-segment-id]") ?? null;
};

export const captureDomSelection = (): DomSelectionCapture | null => {
  if (typeof document === "undefined") return null;
  const selection = window.getSelection();
  if (!selection || selection.rangeCount === 0 || selection.isCollapsed) return null;
  const range = selection.getRangeAt(0);
  const startSegment = segmentForNode(range.startContainer);
  const endSegment = segmentForNode(range.endContainer);
  if (!startSegment || !endSegment) return null;
  const startContent = startSegment.querySelector<HTMLElement>("[data-segment-content]");
  const endContent = endSegment.querySelector<HTMLElement>("[data-segment-content]");

  if (startSegment === endSegment && startContent && startContent === endContent
    && belongsTo(startContent, range.startContainer)
    && belongsTo(startContent, range.endContainer)) {
    return {
      selectedText: selection.toString(),
      segmentIds: [startSegment.dataset.segmentId ?? ""].filter(Boolean),
      textRange: {
        start_utf16: offsetWithin(startContent, range.startContainer, range.startOffset),
        end_utf16: offsetWithin(startContent, range.endContainer, range.endOffset),
      },
    };
  }

  const segmentIds = [...document.querySelectorAll<HTMLElement>("[data-segment-content]")]
    .filter((element) => range.intersectsNode(element))
    .map((element) => element.closest<HTMLElement>("[data-segment-id]")?.dataset.segmentId ?? "");
  return { selectedText: "", segmentIds: unique(segmentIds), textRange: null };
};

const focusedControlName = () => {
  if (typeof document === "undefined") return null;
  const active = document.activeElement;
  if (!(active instanceof HTMLElement) || active.matches(privateSelector) || active.closest(privateSelector)) return null;
  if (active === document.body || active === document.documentElement) return null;
  return active.getAttribute("data-agent-control")
    ?? active.getAttribute("aria-label")
    ?? active.id
    ?? active.tagName.toLowerCase();
};

const projectKey = (snapshot: ProjectIdentity | null) => snapshot
  ? `${snapshot.project.project_id}:${snapshot.project.current_revision_id}`
  : null;

export const useAgentWorkspace = (options: AgentWorkspaceOptions) => {
  const client = options.client ?? agentClient;
  const store = useAgentWorkspaceStore();
  const available = computed(() => client.available);
  const starting = ref(false);
  const error = ref<LocalizedMessage | null>(null);
  const binding = ref<AgentSessionBinding | null>(null);
  const context = ref<ContextSnapshot | null>(null);
  let subscription: AgentSubscription | null = null;
  let publishQueued = false;
  let lifecycle = 0;
  let lastAppliedSequence: string | null = null;
  let updateChain = Promise.resolve();
  let lastForwardedProject = projectKey(toValue(options.projectSnapshot));
  let lastContentSelection: DomSelectionCapture | null = null;
  const processedUiActions = new Set<string>();

  const snapshot = () => toValue(options.projectSnapshot);
  const selection = () => toValue(options.selection);
  const searchSpec = (): SearchSpec => {
    const state = toValue(options.searchState);
    return {
      query: state.query,
      regex: state.regex,
      case_sensitive: state.caseSensitive,
      language_id: state.languageId,
    };
  };

  const buildContext = (): ContextSnapshot => {
    const project = snapshot();
    const shareSelection = toValue(options.selectionSharingEnabled) && project !== null;
    const visibleSelection = selection();
    const domSelection = shareSelection ? lastContentSelection : null;
    return {
      tab: toValue(options.tab),
      mode: toValue(options.mode),
      project_id: project?.project.project_id ?? null,
      revision_id: project?.project.current_revision_id ?? null,
      binding_id: binding.value?.binding_id ?? null,
      window_focused: typeof document === "undefined" ? false : document.hasFocus(),
      focused_control: focusedControlName(),
      selected_text: domSelection?.selectedText ?? "",
      segment_ids: shareSelection ? unique([...visibleSelection.segmentIds, ...(domSelection?.segmentIds ?? [])]) : [],
      alignment_ids: shareSelection ? unique(visibleSelection.alignmentIds) : [],
      text_range: domSelection?.textRange ?? null,
      captured_at: new Date().toISOString(),
    };
  };

  const publishContext = async () => {
    publishQueued = false;
    if (!available.value) return;
    const next = buildContext();
    context.value = next;
    store.setContext(next);
    try {
      await client.call("ui.publish_context", { ...next }, binding.value?.binding_id ?? null);
    } catch (cause) {
      error.value = cause instanceof Error ? cause.message : String(cause);
    }
  };

  const schedulePublish = () => {
    if (publishQueued) return;
    publishQueued = true;
    queueMicrotask(() => { void publishContext(); });
  };

  const acknowledge = async (operationId: string, requestId: string, status: "ui_applied" | "failed" | "cancelled" = "ui_applied") => {
    await client.call("ui.ack", { operation_id: operationId, request_id: requestId, status }, binding.value?.binding_id ?? null);
  };

  const forwardProjectSnapshot = async (next: ProjectIdentity | null, epoch: number) => {
    if (!next) return true;
    const nextKey = projectKey(next);
    if (nextKey === lastForwardedProject) return lifecycle === epoch;
    await options.onProjectSnapshot(next);
    if (lifecycle !== epoch) return false;
    lastForwardedProject = nextKey;
    return true;
  };

  const compareSequence = (left: string, right: string) => {
    const leftValue = BigInt(left);
    const rightValue = BigInt(right);
    return leftValue === rightValue ? 0 : leftValue < rightValue ? -1 : 1;
  };

  const applyProjection = async (sequence: string, projection: Parameters<typeof store.applyProjection>[0]) => {
    if (lastAppliedSequence && compareSequence(sequence, lastAppliedSequence) < 0) return;
    const projectionEpoch = lifecycle;
    lastAppliedSequence = sequence;
    store.applyProjection(projection);
    const projectApplied = await forwardProjectSnapshot(projection.project, projectionEpoch);
    if (!projectApplied || lifecycle !== projectionEpoch) return;
    await options.onSearchState({ spec: projection.search_spec, results: projection.search_results });
    if (lifecycle !== projectionEpoch) return;
    await options.onProjection?.(projection);
    if (lifecycle !== projectionEpoch) return;
    await processPendingUiActions(projection.pending_ui_actions ?? []);
  };

  const refreshProjection = async () => {
    const projection = await client.projection();
    await applyProjection(projection.sequence, projection.data);
  };

  const currentProjectScope = () => ({
    projectId: snapshot()?.project.project_id ?? null,
    revisionId: snapshot()?.project.current_revision_id ?? null,
  });
  const appOnlyNavigationTabs = new Set(["project", "settings"]);
  const executeNavigation = async (request: AgentNavigationRequest) => {
    const scopeBefore = currentProjectScope();
    const lifecycleBefore = lifecycle;
    const isAppOnly = scopeBefore.projectId === null && scopeBefore.revisionId === null;
    if (!request.request_id || !request.operation_id || !request.tab || request.project_id !== scopeBefore.projectId || request.revision_id !== scopeBefore.revisionId || !options.canLeaveDraft()) return false;
    if (isAppOnly && !appOnlyNavigationTabs.has(request.tab)) return false;
    if (!isAppOnly && !binding.value) return false;
    try {
      await options.navigate(request.tab);
    } catch (cause) {
      error.value = cause instanceof Error ? cause.message : String(cause);
      return false;
    }
    const scopeAfter = currentProjectScope();
    if (lifecycle !== lifecycleBefore || scopeAfter.projectId !== scopeBefore.projectId || scopeAfter.revisionId !== scopeBefore.revisionId) return false;
    await acknowledge(request.operation_id, request.request_id);
    return true;
  };

  const executeReveal = async (request: AgentRevealRequest) => {
    const scopeBefore = currentProjectScope();
    const lifecycleBefore = lifecycle;
    if (!request.request_id || !request.operation_id || !scopeBefore.projectId || !binding.value || request.project_id !== scopeBefore.projectId || request.revision_id !== scopeBefore.revisionId || !options.canLeaveDraft()) return false;
    try {
      await options.reveal(request);
    } catch (cause) {
      error.value = cause instanceof Error ? cause.message : String(cause);
      return false;
    }
    const scopeAfter = currentProjectScope();
    if (lifecycle !== lifecycleBefore || scopeAfter.projectId !== scopeBefore.projectId || scopeAfter.revisionId !== scopeBefore.revisionId) return false;
    await acknowledge(request.operation_id, request.request_id);
    return true;
  };

  const processPendingUiActions = async (actions: PendingUiAction[]) => {
    for (const action of actions) {
      if (processedUiActions.has(action.operation_id) || action.status !== "accepted") continue;
      processedUiActions.add(action.operation_id);
      const payload = typeof action.payload === "object" && action.payload !== null ? action.payload as Record<string, unknown> : {};
      const request = {
        ...payload,
        operation_id: action.operation_id,
        request_id: action.request_id,
        project_id: action.project_id,
        revision_id: action.revision_id,
      };
      const completed = action.kind === "navigation_requested"
        ? await executeNavigation(request as AgentNavigationRequest)
        : await executeReveal(request as AgentRevealRequest);
      if (!completed) processedUiActions.delete(action.operation_id);
    }
  };

  const handleEvent = async (event: AppEvent) => {
    store.recordActivity(event);
    try {
      await options.onAppEvent?.(event);
    } catch (cause) {
      error.value = cause instanceof Error ? cause.message : String(cause);
    }
    if (event.kind === "navigation_requested") {
      const request = event.payload as AgentNavigationRequest;
      if (!request?.operation_id || processedUiActions.has(request.operation_id)) return;
      processedUiActions.add(request.operation_id);
      if (!await executeNavigation(request)) processedUiActions.delete(request.operation_id);
      return;
    }
    if (event.kind === "reveal_requested") {
      const request = event.payload as AgentRevealRequest;
      if (!request?.operation_id || processedUiActions.has(request.operation_id)) return;
      processedUiActions.add(request.operation_id);
      if (!await executeReveal(request)) processedUiActions.delete(request.operation_id);
      return;
    }
    if (["project_changed", "revision_advanced", "search_changed", "proposal_changed"].includes(event.kind)) {
      await refreshProjection();
    }
  };

  const handleUpdate = (update: AgentClientUpdate) => {
    updateChain = updateChain.then(async () => {
      try {
        if (update.type === "projection") {
          await applyProjection(update.projection.sequence, update.projection.data);
        } else if (update.type === "event") {
          await handleEvent(update.event);
        } else {
          error.value = update.error.message;
        }
      } catch (cause) {
        error.value = cause instanceof Error ? cause.message : String(cause);
      }
    });
  };

  const ensureBinding = async (token: number) => {
    const project = snapshot();
    if (!project || !available.value) return;
    const reply = await client.call<AgentSessionBinding>("app.bind_session", {
      project_id: project.project.project_id,
      revision_id: project.project.current_revision_id,
    });
    if (token !== lifecycle) return;
    binding.value = reply.data;
    store.setBinding(reply.data.binding_id);
    await refreshProjection();
    schedulePublish();
  };

  const start = async () => {
    if (starting.value || subscription) return;
    if (!available.value) {
      error.value = () => t("agentBrowserUnavailable");
      return;
    }
    starting.value = true;
    error.value = null;
    const token = ++lifecycle;
    try {
      subscription = await client.subscribe(handleUpdate);
      await updateChain;
      await ensureBinding(token);
      schedulePublish();
    } catch (cause) {
      error.value = cause instanceof Error ? cause.message : String(cause);
      subscription?.dispose();
      subscription = null;
    } finally {
      starting.value = false;
    }
  };

  const setProjectSnapshot = (next: ProjectIdentity | null) => {
    const previous = context.value?.project_id ?? binding.value?.project_id ?? null;
    const nextProjectId = next?.project.project_id ?? null;
    lastForwardedProject = projectKey(next);
    if (previous !== nextProjectId) {
      lastContentSelection = null;
      processedUiActions.clear();
      binding.value = null;
      store.setBinding(null);
      context.value = null;
      if (subscription && next) {
        const token = ++lifecycle;
        void ensureBinding(token).catch((cause: unknown) => {
          error.value = cause instanceof Error ? cause.message : String(cause);
        });
      }
    }
    schedulePublish();
  };

  const captureSelection = () => {
    const captured = captureDomSelection();
    if (captured) lastContentSelection = captured;
    schedulePublish();
  };

  const clearSelectionContext = () => {
    lastContentSelection = null;
    schedulePublish();
  };

  const recordOutboundMessage = (projectId: string, messageId: string, requestId: string, content: string) => {
    const immutable = buildContext();
    store.recordOutboundMessage(projectId, {
      id: messageId,
      requestId,
      content,
      sentAt: new Date().toISOString(),
      status: "sending",
    }, immutable);
    return immutable;
  };

  const dispose = () => {
    lifecycle += 1;
    subscription?.dispose();
    subscription = null;
    processedUiActions.clear();
    if (typeof document !== "undefined") {
      document.removeEventListener("selectionchange", captureSelection);
      document.removeEventListener("focusin", captureSelection);
    }
    if (typeof window !== "undefined") {
      window.removeEventListener("focus", captureSelection);
      window.removeEventListener("blur", captureSelection);
    }
  };

  watch(
    () => ({
      tab: toValue(options.tab),
      mode: toValue(options.mode),
      project: projectKey(snapshot()),
      selection: selection(),
      search: searchSpec(),
      sharing: toValue(options.selectionSharingEnabled),
    }),
    () => schedulePublish(),
    { deep: true, flush: "post" },
  );

  if (typeof document !== "undefined") {
    document.addEventListener("selectionchange", captureSelection);
    document.addEventListener("focusin", captureSelection);
    window.addEventListener("focus", captureSelection);
    window.addEventListener("blur", captureSelection);
  }

  return {
    available,
    starting,
    error: computed(() => typeof error.value === "function" ? error.value() : error.value === null ? null : formatError(error.value)),
    binding,
    context,
    store,
    start,
    dispose,
    publishContext,
    captureDomSelection: captureSelection,
    clearSelectionContext,
    setProjectSnapshot,
    recordOutboundMessage,
  };
};
