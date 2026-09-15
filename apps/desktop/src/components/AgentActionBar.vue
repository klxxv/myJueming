<script setup lang="ts">
import { t } from "../i18n";

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
  <section class="agent-review" :aria-label="t('rwReviewAssistantChanges')" data-agent-context="exclude">
    <div class="agent-review__heading"><strong>{{ proposal.title }}</strong><span>{{ t('rwBasedOnRevision', { revision: proposal.revision }) }}</span></div>
    <details open>
      <summary>{{ t('rwChangesDetail', { count: proposal.changes.length }) }}</summary>
      <div class="agent-review__changes">
        <article v-for="change in proposal.changes" :key="change.id">
          <small :title="change.id">{{ change.id.slice(0, 8) }}</small>
          <del>{{ change.before }}</del><ins>{{ change.after }}</ins>
        </article>
      </div>
    </details>
    <div class="agent-review__actions"><button type="button" :disabled="busy" @click="emit('reject', proposal.id)">{{ t('rwRejectChanges') }}</button><button class="agent-review__approve" type="button" :disabled="busy" @click="emit('approve', proposal.id)">{{ busy ? t('rwSubmitting') : t('rwApproveApply') }}</button></div>
  </section>
</template>

<style scoped>
.agent-review { padding: 13px; border: 1px solid var(--line); border-radius: 10px; @apply bg-raised text-ink-900; }
.agent-review__heading { display: flex; flex-wrap: wrap; justify-content: space-between; align-items: center; gap: 8px; font-size: 13px; }
.agent-review__heading span, summary, small { @apply text-ink-500; font-size: 11px; }
details { margin-top: 10px; } summary { cursor: pointer; }
.agent-review__changes { max-height: 210px; overflow: auto; overscroll-behavior: contain; scrollbar-width: thin; }
article { display: grid; gap: 6px; padding-top: 10px; }
del, ins { display: block; padding: 7px; border-radius: 4px; overflow-wrap: anywhere; white-space: pre-wrap; font-size: 12px; line-height: 1.6; }
del { @apply bg-muted; text-decoration-color: var(--ink-500); }
ins { @apply bg-green-soft; text-decoration: none; @apply text-accent-strong; }
.agent-review__actions { display: flex; flex-wrap: wrap; justify-content: flex-end; gap: 8px; margin-top: 12px; }
button { min-height: 34px; padding: 6px 12px; border: 1px solid var(--line); border-radius: 6px; @apply text-ink-700 bg-raised; cursor: pointer; }
.agent-review__approve { @apply bg-accent-solid border-accent text-white; }
button:disabled { opacity: .55; cursor: wait; }
</style>
