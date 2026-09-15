<script setup lang="ts">
import { t } from "../i18n";
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { createCompanionController, type CompanionActivity } from "../domain/companion";
import type { AppSettingsEnvelope } from "../settings/schema";
import AgentActionBar, { type ReviewableProposal } from "./AgentActionBar.vue";
import catImage from "../assets/companion/orange-cat.svg";
import dogImage from "../assets/companion/golden-dog.svg";
import cassiaImage from "../assets/companion/cassia.svg";
import butterflyImage from "../assets/companion/butterfly.svg";
const props = defineProps<{ settings: AppSettingsEnvelope["device"]["pet"]; activity: CompanionActivity; dirtyEditor: boolean; motion: "standard" | "reduced" | "off"; proposal: ReviewableProposal | null; busy: boolean; guidance: string }>();
const emit = defineEmits<{ approve: [id: string]; reject: [id: string]; openAgent: [] }>();
const root = ref<HTMLElement | null>(null);
const strolling = defineModel<boolean>("strolling", { default: false });
const active = computed(() => props.activity === "running" || props.activity === "awaiting_approval");
const open = computed(() => active.value || strolling.value);
const controller = createCompanionController();
const update = () => controller.update({ mode: props.motion === "standard" ? props.settings.presentation : "static", activity: props.activity, dirtyEditor: props.dirtyEditor && props.settings.quietWhileEditing });
onMounted(() => { if (root.value) controller.mount(root.value); update(); });
watch(() => [props.settings.presentation, props.activity, props.dirtyEditor, props.motion], update);
onBeforeUnmount(() => controller.dispose());
</script>

<template>
  <section ref="root" class="companion-garden" :class="{ 'companion-garden--open': open, 'companion-garden--active': active }" :aria-label="t('companionGarden')" :aria-hidden="!open" :inert="!open" data-agent-context="exclude">
    <div class="garden-meadow"><img data-companion-actor="plant" class="garden-plant" :src="cassiaImage" :alt="t('companionPlant')" /><button v-if="settings.catEnabled" class="garden-cat" type="button" data-companion-actor="cat" :aria-label="t('companionPetGardenCat')"><img :src="catImage" :alt="t('companionCat')" /></button><button v-if="settings.dogEnabled" class="garden-dog" type="button" data-companion-actor="dog" :aria-label="t('companionPetGardenDog')"><img :src="dogImage" :alt="t('companionDog')" /></button><img v-if="settings.butterflyMotion && (guidance || active)" class="garden-butterfly" data-companion-actor="butterfly" :src="butterflyImage" :alt="t('companionButterfly')" /></div>
    <div v-if="active" class="garden-work"><div class="garden-work__label"><strong>{{ t('companionGarden') }}</strong><span>{{ proposal ? t('companionReview') : t('companionWorking') }}</span><button type="button" @click="emit('openAgent')">{{ t('companionOpenAgent') }}</button></div><AgentActionBar v-if="proposal" :proposal="proposal" :busy="busy" @approve="emit('approve', $event)" @reject="emit('reject', $event)" /><p v-else role="status">{{ guidance || t('companionSync') }}</p></div>
    <div v-else class="garden-rest"><strong>{{ t('companionGarden') }}</strong><span>{{ t('companionRest') }}</span></div>
  </section>
</template>

<style scoped>
.companion-garden { position: relative; z-index: 5; display: flex; grid-column: 2; grid-row: 1; align-self: end; align-items: center; gap: 26px; width: 100%; min-width: 0; min-height: 154px; max-height: min(300px, calc(100% - 18px)); padding: 16px 26px 14px; overflow: auto; visibility: hidden; border-top: 1px solid var(--line); border-top-left-radius: 18px; @apply text-ink-900 bg-subtle; box-shadow: 0 -14px 30px rgb(38 66 43 / 10%); opacity: 0; pointer-events: none; transform: translateY(calc(100% + 2px)); transition: transform 320ms cubic-bezier(.22, .86, .24, 1), opacity 180ms ease, visibility 0s linear 320ms; }
.companion-garden--open { visibility: visible; opacity: 1; pointer-events: auto; transform: translateY(0); transition-delay: 0s; }
.garden-meadow { position: relative; flex: 0 0 310px; height: 112px; }
.garden-plant { position: absolute; bottom: -6px; left: 48px; width: 210px; height: 78px; transform-origin: bottom center; }
.garden-meadow button { position: absolute; bottom: 0; width: 110px; height: 82px; padding: 0; border: 0; background: transparent; cursor: pointer; }
.garden-cat { left: 0; }
.garden-dog { right: 0; }
.garden-meadow button img { width: 100%; height: 100%; }
.garden-butterfly { position: absolute; top: 0; left: 138px; width: 35px; }
.garden-rest, .garden-work { flex: 1; min-width: 0; }
.garden-rest { display: flex; flex-direction: column; gap: 5px; }
.garden-rest strong { font-size: var(--jm-font-size-body); line-height: var(--jm-line-height-body); }
.garden-rest span, .garden-work p { @apply text-ink-500; font-size: var(--jm-font-size-callout); line-height: var(--jm-line-height-callout); }
.garden-work__label { display: flex; flex-wrap: wrap; align-items: center; gap: 12px; margin-bottom: 10px; }
.garden-work__label strong { font-size: 12px; }
.garden-work__label span { @apply text-ink-500; font-size: 11px; }
.garden-work__label button { margin-left: auto; border: 0; @apply text-accent; background: transparent; font-size: 11px; cursor: pointer; }
.garden-work :deep(.agent-review__changes) { max-height: 100px; }
.garden-work :deep(.agent-review) { padding: 10px 13px; }
@media (max-width: 900px) {
  .companion-garden { gap: 14px; padding-inline: 18px; }
  .garden-meadow { flex-basis: 250px; }
  .garden-rest span { display: none; }
}
</style>
