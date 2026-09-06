<script setup lang="ts">
export interface ReviewableProposal {
  id: string;
  title: string;
  status: string;
  revision: string;
  changes: Array<{ id: string; before: string; after: string }>;
}
defineProps<{ proposal: ReviewableProposal; busy?: boolean }>();
const emit = defineEmits<{ approve: [id: string]; reject: [id: string] }>();
</script>

<template>
  <section class="agent-review" aria-label="审核助手修改" data-agent-context="exclude">
    <div class="agent-review__heading"><strong>{{ proposal.title }}</strong><span>基于 R{{ proposal.revision }}</span></div>
    <details open>
      <summary>{{ proposal.changes.length }} 项修改 · 查看完整差异</summary>
      <div class="agent-review__changes">
        <article v-for="change in proposal.changes" :key="change.id">
          <small :title="change.id">{{ change.id.slice(0, 8) }}</small>
          <del>{{ change.before }}</del><ins>{{ change.after }}</ins>
        </article>
      </div>
    </details>
    <div class="agent-review__actions"><button type="button" :disabled="busy" @click="emit('reject', proposal.id)">拒绝修改</button><button class="agent-review__approve" type="button" :disabled="busy" @click="emit('approve', proposal.id)">{{ busy ? '提交中…' : '批准并应用' }}</button></div>
  </section>
</template>

<style scoped>
.agent-review { padding: 13px; border: 1px solid var(--line); border-radius: 10px; background: var(--surface-raised); color: var(--ink-900); }
.agent-review__heading { display: flex; flex-wrap: wrap; justify-content: space-between; align-items: center; gap: 8px; font-size: 13px; }
.agent-review__heading span, summary, small { color: var(--ink-500); font-size: 11px; }
details { margin-top: 10px; } summary { cursor: pointer; }
.agent-review__changes { max-height: 210px; overflow: auto; overscroll-behavior: contain; scrollbar-width: thin; }
article { display: grid; gap: 6px; padding-top: 10px; }
del, ins { display: block; padding: 7px; border-radius: 4px; overflow-wrap: anywhere; white-space: pre-wrap; font-size: 12px; line-height: 1.6; }
del { background: var(--surface-muted); text-decoration-color: var(--ink-500); }
ins { background: var(--surface-green-soft); text-decoration: none; color: var(--green-900); }
.agent-review__actions { display: flex; flex-wrap: wrap; justify-content: flex-end; gap: 8px; margin-top: 12px; }
button { min-height: 34px; padding: 6px 12px; border: 1px solid var(--line); border-radius: 6px; color: var(--ink-700); background: var(--surface-raised); cursor: pointer; }
.agent-review__approve { background: var(--green-700); border-color: var(--green-700); color: var(--paper); }
button:disabled { opacity: .55; cursor: wait; }
</style>
