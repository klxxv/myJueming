<script setup lang="ts">
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
const controller = createCompanionController({ onEvent: event => { if (event.kind === "manual_pet") strolling.value = !strolling.value; } });
const update = () => controller.update({ mode: props.motion === "standard" ? props.settings.presentation : "static", activity: props.activity, dirtyEditor: props.dirtyEditor && props.settings.quietWhileEditing });
onMounted(() => { if (root.value) controller.mount(root.value); update(); });
watch(() => [props.settings.presentation, props.activity, props.dirtyEditor, props.motion], update);
onBeforeUnmount(() => controller.dispose());
</script>

<template>
  <section ref="root" class="companion-garden" :class="{ 'companion-garden--active': active }" aria-label="决明小花园" data-agent-context="exclude">
    <div class="garden-meadow"><img data-companion-actor="plant" class="garden-plant" :src="cassiaImage" alt="决明草与花朵" /><button v-if="settings.catEnabled" v-show="active || strolling" class="garden-cat" type="button" data-companion-actor="cat" aria-label="摸摸花园里的猫猫"><img :src="catImage" alt="橘猫" /></button><button v-if="settings.dogEnabled" v-show="active || strolling" class="garden-dog" type="button" data-companion-actor="dog" aria-label="摸摸花园里的狗狗"><img :src="dogImage" alt="金毛狗狗" /></button><img v-if="settings.butterflyMotion && (guidance || active)" class="garden-butterfly" data-companion-actor="butterfly" :src="butterflyImage" alt="引导工作的蝴蝶" /></div>
    <div v-if="active" class="garden-work"><div class="garden-work__label"><strong>决明小花园</strong><span>{{ proposal ? '需要你审核这次修改' : '助手正在工作' }}</span><button type="button" @click="emit('openAgent')">查看助手</button></div><AgentActionBar v-if="proposal" :proposal="proposal" :busy="busy" @approve="emit('approve', $event)" @reject="emit('reject', $event)" /><p v-else role="status">{{ guidance || '应用操作与当前工程同步' }}</p></div>
    <button v-else class="garden-invite" type="button" @click="strolling = !strolling">{{ strolling ? '回小屋休息' : '来花园走走' }}</button>
  </section>
</template>

<style scoped>
.companion-garden { display: flex; align-items: flex-end; position: relative; min-height: 26px; height: 26px; border-top: 1px solid var(--line); background: var(--surface-subtle); color: var(--ink-900); overflow: hidden; }.companion-garden--active { min-height: 138px; height: auto; max-height: 300px; padding: 14px 22px 12px; align-items: center; gap: 24px; overflow-y: auto; }.garden-meadow { position: relative; flex: 0 0 255px; height: 80px; margin-left: 24px; }.garden-plant { position: absolute; bottom: -4px; left: 35px; height: 65px; width: 180px; transform-origin: bottom center; }.companion-garden:not(.companion-garden--active) .garden-meadow { height: 24px; margin-left: 175px; }.companion-garden:not(.companion-garden--active) .garden-plant { height: 36px; width: 180px; }.garden-meadow button { position: absolute; padding: 0; border: 0; width: 92px; height: 69px; background: transparent; cursor: pointer; bottom: 0; }.garden-cat { left: 0; }.garden-dog { right: 3px; }.garden-meadow button img { width: 100%; height: 100%; }.garden-butterfly { position: absolute; width: 33px; top: 0; left: 110px; }.garden-work { flex: 1; min-width: 0; }.garden-work__label { display: flex; flex-wrap: wrap; align-items: center; gap: 12px; margin-bottom: 10px; }.garden-work__label strong { font-size: 12px; }.garden-work__label span, .garden-work p { font-size: 11px; color: var(--ink-500); }.garden-work__label button, .garden-invite { border: 0; background: transparent; color: var(--green-700); font-size: 11px; cursor: pointer; }.garden-work__label button { margin-left: auto; }.garden-invite { margin: 0 20px 5px auto; font-size: 11px; }.garden-work :deep(.agent-review__changes) { max-height: 100px; }.garden-work :deep(.agent-review) { padding: 10px 13px; }.companion-garden:not(.companion-garden--active) .garden-meadow button { height: 33px; width: 48px; }
</style>
