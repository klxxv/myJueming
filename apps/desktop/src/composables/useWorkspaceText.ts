import { shallowRef } from "vue";
import type { KernelClient, SegmentText, WorkspaceProject } from "../domain/kernel-client";

/** Bounded, discardable body cache; the full workspace contains only structure. */
export function useWorkspaceText(client: Pick<KernelClient, "loadParallelSlice">) {
  const texts = shallowRef<ReadonlyMap<string, SegmentText>>(new Map());
  let project: WorkspaceProject | null = null;
  let epoch = 0;
  let visibleIds = new Set<string>();
  const pending = new Map<string, Promise<void>>();
  const selectProject = (next: WorkspaceProject) => {
    const sameProject = project?.project.project_id === next.project.project_id;
    if (sameProject && project?.project.current_revision_id === next.project.current_revision_id) { project = next; return; }
    epoch++;
    const hashes = new Map(next.segments.map(segment => [segment.segment_id, segment.content_hash]));
    texts.value = new Map([...texts.value].filter(([id, text]) => sameProject && hashes.get(id) === text.content_hash));
    project = next;
    pending.clear();
    if (!sameProject) visibleIds.clear();
  };
  const ensure = async (ids: string[]) => {
    const current = project;
    if (!current) return;
    const token = epoch;
    const missing = [...new Set(ids)].filter(id => !texts.value.has(id));
    for (let offset = 0; offset < missing.length; offset += 200) {
      const batch = missing.slice(offset, offset + 200);
      const key = `${token}:${batch.join(",")}`;
      let request = pending.get(key);
      if (!request) {
        request = (async () => {
          const slice = await client.loadParallelSlice(current, batch);
          if (token !== epoch || slice.project_id !== current.project.project_id || slice.revision_id !== current.project.current_revision_id) return;
          const next = new Map(texts.value);
          for (const segment of slice.segments) { next.delete(segment.segment_id); next.set(segment.segment_id, segment); }
          for (const id of next.keys()) {
            if (next.size <= 1200) break;
            if (!visibleIds.has(id) && !ids.includes(id)) next.delete(id);
          }
          texts.value = next;
        })();
        pending.set(key, request);
      }
      try { await request; } finally { if (pending.get(key) === request) pending.delete(key); }
      if (token !== epoch) return;
    }
  };
  const visible = async (ids: string[]) => { visibleIds = new Set(ids); await ensure(ids); };
  // View Find scans the current view's IDs in bounded batches, retaining only
  // match IDs. It does not create a project-search session or fill the cache.
  const find = async (query: string, cancelled: () => boolean = () => false, documentIds?: string[]) => {
    const current = project;
    if (!current) return [];
    const token = epoch;
    const ids = current.segment_orders.filter(order => !documentIds || documentIds.includes(order.document_id)).flatMap(order => order.entries.map(entry => entry.segment_id));
    const matches: string[] = [];
    for (let offset = 0; offset < ids.length; offset += 200) {
      if (cancelled() || token !== epoch) return [];
      const slice = await client.loadParallelSlice(current, ids.slice(offset, offset + 200));
      if (cancelled() || token !== epoch) return [];
      matches.push(...slice.segments.filter(segment => segment.content.toLocaleLowerCase().includes(query)).map(segment => segment.segment_id));
    }
    const matched = new Set(matches);
    return ids.filter(id => matched.has(id));
  };
  return { texts, selectProject, ensure, visible, find };
}
