import type {
  ContextProvider,
  ExtensionCapabilities,
  ExtensionCapabilityRegistry,
  ExtensionCapabilitySnapshot,
  ExtensionGeneration,
  ExtensionRegistration,
  NavigationContribution,
  OperatorProvider,
  ToolProvider,
} from "./types";

type CapabilityKind = keyof ExtensionCapabilities;
type Entry = { owner: string; generation: number; kind: CapabilityKind; id: string; value: NavigationContribution | ContextProvider | OperatorProvider | ToolProvider };

const kinds: CapabilityKind[] = ["navigation", "context", "operators", "tools"];

const cloneValue = <T extends Entry["value"]>(value: T): T => {
  if ("fields" in value) return { ...value, fields: [...value.fields] } as T;
  return { ...value } as T;
};

const sortByOrderThenId = <T extends { id: string; order?: number }>(items: T[]) => [...items]
  .sort((left, right) => (left.order ?? 0) - (right.order ?? 0) || left.id.localeCompare(right.id));

export const createExtensionCapabilityRegistry = (): ExtensionCapabilityRegistry => {
  const entries = new Map<string, Entry>();
  const activeGenerations = new Map<string, ExtensionGeneration>();
  const subscribers = new Set<(snapshot: ExtensionCapabilitySnapshot) => void>();
  let serial = 0;
  let disposed = false;

  const snapshot = (): ExtensionCapabilitySnapshot => {
    const values = (kind: CapabilityKind) => [...entries.values()]
      .filter((entry) => entry.kind === kind)
      .map((entry) => cloneValue(entry.value));
    return Object.freeze({
      navigation: Object.freeze(sortByOrderThenId(values("navigation") as NavigationContribution[])),
      context: Object.freeze(sortByOrderThenId(values("context") as ContextProvider[])),
      operators: Object.freeze(sortByOrderThenId(values("operators") as OperatorProvider[])),
      tools: Object.freeze(sortByOrderThenId(values("tools") as ToolProvider[])),
    });
  };
  const publish = () => {
    const next = snapshot();
    for (const listener of subscribers) listener(next);
  };
  const removeGeneration = (owner: string, generation: number) => {
    let changed = false;
    for (const [key, entry] of entries) {
      if (entry.owner === owner && entry.generation === generation) {
        entries.delete(key);
        changed = true;
      }
    }
    if (changed) publish();
  };

  return {
    beginGeneration(owner) {
      if (disposed) throw new Error("Extension capability registry has been disposed");
      if (!owner) throw new Error("Extension generation owner is required");
      activeGenerations.get(owner)?.dispose();
      const generation = ++serial;
      let generationDisposed = false;
      const registrations = new Set<ExtensionRegistration>();
      const current: ExtensionGeneration = {
        owner,
        generation,
        register(capabilities) {
          if (disposed || generationDisposed) throw new Error("Extension generation is not active");
          const pending: Array<{ key: string; kind: CapabilityKind; raw: Entry["value"] }> = [];
          const pendingKeys = new Set<string>();
          for (const kind of kinds) {
            for (const raw of capabilities[kind] ?? []) {
              const key = `${kind}:${raw.id}`;
              if (!raw.id) throw new Error(`Extension ${kind} contribution needs an id`);
              if (entries.has(key) || pendingKeys.has(key)) throw new Error(`Extension contribution already registered: ${key}`);
              pendingKeys.add(key);
              pending.push({ key, kind, raw });
            }
          }
          const inserted: string[] = [];
          for (const { key, kind, raw } of pending) {
            entries.set(key, { owner, generation, kind, id: raw.id, value: cloneValue(raw) });
            inserted.push(key);
          }
          if (inserted.length) publish();
          let registrationDisposed = false;
          const registration: ExtensionRegistration = {
            dispose() {
              if (registrationDisposed) return;
              registrationDisposed = true;
              let changed = false;
              for (const key of inserted) changed = entries.delete(key) || changed;
              registrations.delete(registration);
              if (changed) publish();
            },
          };
          registrations.add(registration);
          return registration;
        },
        dispose() {
          if (generationDisposed) return;
          generationDisposed = true;
          for (const registration of [...registrations]) registration.dispose();
          removeGeneration(owner, generation);
          if (activeGenerations.get(owner) === current) activeGenerations.delete(owner);
        },
      };
      activeGenerations.set(owner, current);
      return current;
    },
    snapshot,
    subscribe(listener) {
      if (disposed) return () => undefined;
      subscribers.add(listener);
      listener(snapshot());
      return () => subscribers.delete(listener);
    },
    dispose() {
      if (disposed) return;
      disposed = true;
      subscribers.clear();
      for (const generation of [...activeGenerations.values()]) generation.dispose();
      activeGenerations.clear();
      entries.clear();
    },
    get activeSubscriberCount() { return subscribers.size; },
  };
};
