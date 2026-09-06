import { computed, ref } from "vue";
import { defineStore } from "pinia";
import type {
  AgentAnnotationScope,
  AgentChatMessage,
  ImmutableContextSnapshot,
  AgentProjection,
  AgentProposal,
  AgentProjectScope,
  AppEvent,
  ContextSnapshot,
} from "../domain/agent-types";

const EMPTY_SCOPE = (): AgentProjectScope => ({
  chat: [],
  annotationScope: { segmentIds: [], alignmentIds: [] },
  drafts: {},
});

const immutableContext = (context: ContextSnapshot): ImmutableContextSnapshot => Object.freeze({
  ...context,
  segment_ids: Object.freeze([...context.segment_ids]),
  alignment_ids: Object.freeze([...context.alignment_ids]),
  text_range: context.text_range ? Object.freeze({ ...context.text_range }) : null,
});

export const useAgentWorkspaceStore = defineStore("agent-workspace", () => {
  const activity = ref<AppEvent[]>([]);
  const proposals = ref<AgentProposal[]>([]);
  const bindingId = ref<string | null>(null);
  const latestContext = ref<ContextSnapshot | null>(null);
  const projectScopes = ref<Record<string, AgentProjectScope>>({});

  const activeProjectId = computed(() => latestContext.value?.project_id ?? null);

  const ensureProjectScope = (projectId: string): AgentProjectScope => {
    const existing = projectScopes.value[projectId];
    if (existing) return existing;
    const scope = EMPTY_SCOPE();
    projectScopes.value = { ...projectScopes.value, [projectId]: scope };
    return scope;
  };

  const applyProjection = (projection: AgentProjection) => {
    bindingId.value = projection.binding_id;
    latestContext.value = projection.context;
    proposals.value = [...projection.proposals];
    const projectId = projection.project?.project.project_id ?? projection.context?.project_id;
    if (projectId) ensureProjectScope(projectId);
  };

  const recordActivity = (event: AppEvent) => {
    activity.value = [...activity.value, event].slice(-200);
  };

  const setBinding = (binding: string | null) => {
    bindingId.value = binding;
  };

  const setContext = (context: ContextSnapshot) => {
    latestContext.value = context;
    if (context.project_id) ensureProjectScope(context.project_id);
  };

  const setAnnotationScope = (projectId: string, scope: AgentAnnotationScope) => {
    const project = ensureProjectScope(projectId);
    project.annotationScope = {
      segmentIds: [...new Set(scope.segmentIds)],
      alignmentIds: [...new Set(scope.alignmentIds)],
    };
  };

  const setDraft = (projectId: string, draftKey: string, value: string) => {
    const project = ensureProjectScope(projectId);
    project.drafts = { ...project.drafts, [draftKey]: value };
  };

  const removeDraft = (projectId: string, draftKey: string) => {
    const project = ensureProjectScope(projectId);
    const { [draftKey]: _removed, ...remaining } = project.drafts;
    project.drafts = remaining;
  };

  const recordOutboundMessage = (projectId: string, message: Omit<AgentChatMessage, "role" | "context" | "error">, context: ContextSnapshot) => {
    const project = ensureProjectScope(projectId);
    const immutable = immutableContext(context);
    project.chat = [...project.chat, {
      ...message,
      role: "user",
      context: immutable,
      error: null,
    }];
  };

  const markMessageSent = (projectId: string, messageId: string) => {
    const project = ensureProjectScope(projectId);
    project.chat = project.chat.map((message) => message.id === messageId
      ? { ...message, status: "sent" }
      : message);
  };

  const markMessageFailed = (projectId: string, messageId: string, error: string) => {
    const project = ensureProjectScope(projectId);
    project.chat = project.chat.map((message) => message.id === messageId
      ? { ...message, status: "failed", error }
      : message);
  };

  return {
    activity,
    proposals,
    bindingId,
    latestContext,
    projectScopes,
    activeProjectId,
    ensureProjectScope,
    applyProjection,
    recordActivity,
    setBinding,
    setContext,
    setAnnotationScope,
    setDraft,
    removeDraft,
    recordOutboundMessage,
    markMessageSent,
    markMessageFailed,
  };
});
