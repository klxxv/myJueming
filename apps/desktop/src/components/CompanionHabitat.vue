<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { createCompanionController, type CompanionActivity } from "../domain/companion";
import type { AppSettingsEnvelope } from "../settings/schema";
import catImage from "../assets/companion/orange-cat.svg";
import dogImage from "../assets/companion/golden-dog.svg";
const props = defineProps<{ settings: AppSettingsEnvelope["device"]["pet"]; activity: CompanionActivity; inGarden: boolean; dirtyEditor: boolean; motion: "standard" | "reduced" | "off" }>();
const root = ref<HTMLElement | null>(null);
const notice = ref("");
const active = computed(() => props.inGarden || props.activity === "running" || props.activity === "awaiting_approval");
const controller = createCompanionController({ onEvent: event => { if (event.kind === "manual_pet") notice.value = event.actor === "cat" ? "猫猫伸了个懒腰" : "狗狗向你摇摇尾巴"; } });
const update = () => controller.update({ mode: props.motion === "standard" ? props.settings.presentation : "static", activity: props.activity, dirtyEditor: props.dirtyEditor && props.settings.quietWhileEditing });
onMounted(() => { if (root.value) controller.mount(root.value); update(); });
watch(() => [props.settings.presentation, props.activity, props.dirtyEditor, props.motion], update);
onBeforeUnmount(() => controller.dispose());
</script>

<template>
  <section ref="root" class="companion-habitat" aria-label="猫猫与狗狗的小家" data-agent-context="exclude">
    <div class="cat-tree"><div class="cat-tree__platform"></div><div class="cat-tree__post"></div><div class="cat-tree__bed"></div><button v-if="settings.catEnabled" v-show="!active" type="button" data-companion-actor="cat" aria-label="摸摸猫猫"><img :src="catImage" alt="趴在猫窝里的橘猫" /></button></div>
    <div class="dog-house"><div class="dog-house__roof"></div><div class="dog-house__wall"><span></span></div><button v-if="settings.dogEnabled" v-show="!active" type="button" data-companion-actor="dog" aria-label="摸摸狗狗"><img :src="dogImage" alt="在小屋前休息的金毛" /></button></div>
    <p role="status">{{ active ? '去小花园陪你工作了' : notice || '决明小花园' }}</p>
  </section>
</template>

<style scoped>
.companion-habitat { position: relative; flex: 1; min-height: 120px; max-height: 300px; width: 100%; margin-top: auto; color: var(--ink-500); overflow: hidden; }.cat-tree { position: relative; width: 112px; height: 105px; margin: 0 auto; }.cat-tree__post { position: absolute; width: 11px; height: 64px; left: 32px; bottom: 0; border-radius: 3px; background: #cbbb92; background-image: repeating-linear-gradient(0deg, transparent 0 4px, #b9a779 5px 6px); }.cat-tree__platform { position: absolute; top: 30px; left: 11px; width: 86px; height: 9px; border-radius: 7px; background: #d7caa8; }.cat-tree__bed { position: absolute; bottom: 3px; left: 5px; width: 103px; height: 26px; border-radius: 8px 8px 30px 30px; background: #e1d7bb; border-bottom: 5px solid #c6b68f; }.companion-habitat button { position: absolute; z-index: 1; padding: 0; border: 0; background: transparent; cursor: pointer; width: 102px; height: 76px; }.companion-habitat img { width: 100%; height: 100%; pointer-events: none; }.cat-tree button { left: 9px; bottom: 12px; }.dog-house { position: relative; width: 116px; height: 106px; margin: 0 auto; }.dog-house__roof { position: absolute; top: 9px; left: 6px; width: 104px; height: 65px; background: #b4bea0; clip-path: polygon(50% 0, 100% 66%, 92% 76%, 50% 23%, 8% 76%, 0 66%); z-index: 1; }.dog-house__wall { position: absolute; inset: 49px 20px 8px; border-radius: 3px; background: #ddd4b8; }.dog-house__wall span { position: absolute; bottom: 0; left: 16px; height: 40px; width: 44px; border-radius: 23px 23px 0 0; background: #a3997d; }.dog-house button { bottom: -3px; left: 10px; }.companion-habitat p { text-align: center; font-size: 11px; margin: 7px 0; }
.companion-habitat { container-type: size; display: flex; flex-direction: column; justify-content: center; }
.cat-tree, .dog-house { flex: 0 0 105px; }
@container(max-height: 235px) { .cat-tree, .dog-house { transform: scale(.72); transform-origin: top center; margin-bottom: -29px; } p { display: none; } }
@container(max-height: 150px) { .cat-tree, .dog-house { transform: scale(.6); margin-bottom: -46px; } }
</style>
