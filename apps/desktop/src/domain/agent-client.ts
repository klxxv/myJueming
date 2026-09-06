import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  AgentCall,
  AgentClientUpdate,
  AgentParams,
  AgentProjection,
  AgentReply,
  AgentSubscription,
  AppErrorShape,
  AppEvent,
} from "./agent-types";

export class AgentClientUnavailableError extends Error {
  constructor() {
    super("嵌入式 Agent 仅可在已打开本地工程的 Tauri 桌面应用中使用");
    this.name = "AgentClientUnavailableError";
  }
}

export interface AgentBridge {
  available(): boolean;
  invoke<T>(command: string, args?: Record<string, unknown>): Promise<T>;
  listen<T>(channel: string, listener: (payload: T) => void): Promise<() => void>;
}

const tauriBridge: AgentBridge = {
  available: () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window,
  invoke: <T>(command: string, args?: Record<string, unknown>) => invoke<T>(command, args),
  listen: async <T>(channel: string, listener: (payload: T) => void) => listen<T>(channel, (event) => listener(event.payload)),
};

const newRequestId = () => globalThis.crypto.randomUUID();

const compareSequence = (left: string, right: string) => {
  const leftValue = BigInt(left);
  const rightValue = BigInt(right);
  return leftValue === rightValue ? 0 : leftValue < rightValue ? -1 : 1;
};

const isNextSequence = (candidate: string, current: string) => BigInt(candidate) === BigInt(current) + 1n;

const errorShape = (error: unknown): AppErrorShape => ({
  code: error instanceof AgentClientUnavailableError ? "agent_unavailable" : "agent_stream_error",
  message: error instanceof Error ? error.message : String(error),
});

export interface AgentClient {
  readonly available: boolean;
  call<T>(method: string, params?: AgentParams, bindingId?: string | null): Promise<AgentReply<T>>;
  projection(): Promise<AgentReply<AgentProjection>>;
  subscribe(listener: (update: AgentClientUpdate) => void): Promise<AgentSubscription>;
}

export const createAgentClient = (bridge: AgentBridge = tauriBridge): AgentClient => {
  const requireRuntime = () => {
    if (!bridge.available()) throw new AgentClientUnavailableError();
  };

  const call = async <T>(method: string, params: AgentParams = {}, bindingId?: string | null) => {
    requireRuntime();
    const callEnvelope: AgentCall = {
      request_id: newRequestId(),
      method,
      params,
      binding_id: bindingId,
    };
    return bridge.invoke<AgentReply<T>>("agent_call", { call: callEnvelope });
  };

  const projection = async () => {
    requireRuntime();
    return bridge.invoke<AgentReply<AgentProjection>>("agent_projection");
  };

  const subscribe = async (listener: (update: AgentClientUpdate) => void): Promise<AgentSubscription> => {
    requireRuntime();
    let disposed = false;
    let snapshotReady = false;
    let lastSequence: string | null = null;
    let queued: AppEvent[] = [];
    let eventChain = Promise.resolve();

    const emitError = (error: unknown) => {
      if (!disposed) listener({ type: "error", error: errorShape(error) });
    };

    const acceptProjection = async () => {
      const next = await projection();
      if (disposed) return;
      lastSequence = next.sequence;
      snapshotReady = true;
      listener({ type: "projection", projection: next });
    };

    const consumeEvent = async (event: AppEvent, afterResnapshot = false): Promise<void> => {
      if (disposed || lastSequence === null) return;
      if (compareSequence(event.sequence, lastSequence) <= 0) return;
      if (!isNextSequence(event.sequence, lastSequence)) {
        if (afterResnapshot) {
          emitError(new Error(`Agent event sequence gap remains after resnapshot (${lastSequence} → ${event.sequence})`));
          return;
        }
        await acceptProjection();
        await consumeEvent(event, true);
        return;
      }
      lastSequence = event.sequence;
      listener({ type: "event", event });
    };

    const enqueue = (event: AppEvent) => {
      eventChain = eventChain
        .then(async () => {
          if (disposed) return;
          if (!snapshotReady) {
            queued.push(event);
            return;
          }
          await consumeEvent(event);
        })
        .catch(emitError);
    };

    let unlisten: (() => void) | null = null;
    try {
      unlisten = await bridge.listen<AppEvent>("agent_subscribe", enqueue);
      await acceptProjection();
      const initialEvents = queued;
      queued = [];
      for (const event of initialEvents.sort((left, right) => compareSequence(left.sequence, right.sequence))) {
        await consumeEvent(event);
      }
    } catch (error) {
      unlisten?.();
      throw error;
    }

    return {
      dispose() {
        disposed = true;
        queued = [];
        unlisten?.();
        unlisten = null;
      },
    };
  };

  return {
    get available() { return bridge.available(); },
    call,
    projection,
    subscribe,
  };
};

export const agentClient = createAgentClient();
