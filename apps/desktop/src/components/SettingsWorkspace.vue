<script setup lang="ts">
import { ChevronRight, Database, Keyboard, Minus, Monitor, Palette, Plus, Settings2 } from "@lucide/vue";
import { ref } from "vue";
import type { AppTheme, CacheCleanupPolicy, ShortcutProfile } from "../composables/useAppSettings";

defineProps<{ usesMacShortcuts: boolean; shortcutRows: Array<[string, string]>; cacheCleaning: boolean; lastCacheCleanupAt: number | null }>();
const theme = defineModel<AppTheme>("theme", { required: true });
const fontScale = defineModel<number>("fontScale", { required: true });
const uiScale = defineModel<number>("uiScale", { required: true });
const shortcutProfile = defineModel<ShortcutProfile>("shortcutProfile", { required: true });
const trackpadOptimized = defineModel<boolean>("trackpadOptimized", { required: true });
const autoSaveDelayMs = defineModel<number>("autoSaveDelayMs", { required: true });
const cacheCleanupPolicy = defineModel<CacheCleanupPolicy>("cacheCleanupPolicy", { required: true });

type SettingsSection = "appearance" | "interaction" | "storage";
const activeSection = ref<SettingsSection>("appearance");
const sections = [
  { id: "appearance" as const, label: "外观与缩放", description: "主题、字体与界面空间", icon: Palette },
  { id: "interaction" as const, label: "键盘与触控板", description: "快捷键和滚动行为", icon: Keyboard },
  { id: "storage" as const, label: "保存与历史", description: "自动保存和缓存", icon: Database },
];
const emit = defineEmits<{ applyUi: []; applyInteraction: []; applyPersistence: []; clearCache: [] }>();
</script>

<template>
  <section class="settings-view">
    <header class="settings-header"><span class="settings-header__icon"><Settings2 :size="20" /></span><div><h2>设置</h2><p>偏好设置仅保存在这台设备上</p></div></header>
    <div class="settings-split">
      <nav class="settings-sidebar" aria-label="设置分类">
        <button v-for="section in sections" :key="section.id" type="button" :class="{ active: activeSection === section.id }" @click="activeSection = section.id">
          <span class="settings-sidebar__icon"><component :is="section.icon" :size="18" /></span><span><strong>{{ section.label }}</strong><small>{{ section.description }}</small></span><ChevronRight :size="15" class="settings-sidebar__chevron" />
        </button>
        <p><Monitor :size="15" />完全离线，不上传原文或译文</p>
      </nav>
      <div class="settings-detail">
        <section v-if="activeSection === 'appearance'" class="settings-pane">
          <header><p>外观</p><h3>外观与缩放</h3><span>分别调整阅读文字和整个操作界面的尺寸。</span></header>
          <div class="settings-group"><label class="setting-row"><span><strong>显示主题</strong><small>选择适合当前环境的界面色彩</small></span><select v-model="theme" @change="emit('applyUi')"><option value="light">明亮</option><option value="eye">护眼</option></select></label></div>
          <div class="settings-group scale-group">
            <div class="scale-heading"><span><strong>字体大小</strong><small>调整双语正文和编辑文字</small></span><output>{{ fontScale }}%</output></div>
            <div class="stepped-slider"><Minus :size="17" /><input v-model.number="fontScale" aria-label="字体大小" type="range" min="90" max="130" step="10" @input="emit('applyUi')" /><Plus :size="17" /></div><div class="scale-labels"><span>较小</span><span>标准</span><span>较大</span></div>
          </div>
          <div class="settings-group scale-group">
            <div class="scale-heading"><span><strong>界面空间大小</strong><small>调整按钮、边栏和控件的整体显示比例</small></span><output>{{ uiScale }}%</output></div>
            <div class="stepped-slider"><Minus :size="17" /><input v-model.number="uiScale" aria-label="界面空间大小" type="range" min="85" max="115" step="5" @input="emit('applyUi')" /><Plus :size="17" /></div><div class="scale-labels"><span>紧凑</span><span>默认</span><span>宽松</span></div>
          </div>
        </section>
        <section v-else-if="activeSection === 'interaction'" class="settings-pane">
          <header><p>输入</p><h3>键盘与触控板</h3><span>按操作系统习惯调整快捷键和滚动方式。</span></header>
          <div class="settings-group">
            <label class="setting-row"><span><strong>快捷键布局</strong><small>当前主修饰键：{{ usesMacShortcuts ? 'Command' : 'Ctrl' }}</small></span><select v-model="shortcutProfile" @change="emit('applyInteraction')"><option value="auto">自动识别（推荐）</option><option value="macos">macOS · Command</option><option value="windows">Windows / Linux · Ctrl</option></select></label>
            <label class="setting-row"><span><strong>触控板优化</strong><small>双指滚动接管跳转动画，拖拽仅从手柄开始</small></span><input v-model="trackpadOptimized" class="switch" type="checkbox" @change="emit('applyInteraction')" /></label>
          </div>
          <div class="settings-group shortcut-grid" aria-label="当前快捷键说明"><template v-for="row in shortcutRows" :key="row[0]"><span>{{ row[0] }}</span><kbd>{{ row[1] }}</kbd></template></div>
        </section>
        <section v-else class="settings-pane">
          <header><p>工程</p><h3>保存与历史</h3><span>每次成功变更都会保留完整 Revision。</span></header>
          <div class="settings-group">
            <label class="setting-row"><span><strong>保存延迟</strong><small>停止输入后自动写入一个完整 ChangeSet</small></span><select v-model.number="autoSaveDelayMs" @change="emit('applyPersistence')"><option :value="1000">1 秒</option><option :value="3000">3 秒（推荐）</option><option :value="5000">5 秒</option><option :value="10000">10 秒</option><option :value="30000">30 秒</option></select></label>
            <label class="setting-row"><span><strong>缓存清理</strong><small>只清理可重建的 .jm/cache</small></span><select v-model="cacheCleanupPolicy" @change="emit('applyPersistence')"><option value="startup">每次启动</option><option value="weekly">每 7 天（推荐）</option><option value="monthly">每 30 天</option><option value="never">从不自动清理</option></select></label>
          </div>
          <div class="settings-group cache-row"><span><strong>立即清理派生缓存</strong><small>revisions/ 中的正式历史不会被删除</small></span><div><button class="secondary-button" type="button" :disabled="cacheCleaning" @click="emit('clearCache')">{{ cacheCleaning ? '清理中…' : '立即清理' }}</button><small v-if="lastCacheCleanupAt">上次：{{ new Date(lastCacheCleanupAt).toLocaleString('zh-CN', { hour12: false }) }}</small></div></div>
        </section>
      </div>
    </div>
  </section>
</template>

<style scoped>
.settings-view { display: flex; flex-direction: column; width: 100%; height: 100%; min-height: 0; color: var(--ink-900); background: var(--surface-app); }
.settings-header { display: flex; flex: 0 0 74px; align-items: center; gap: 12px; padding: 0 24px; border-bottom: 1px solid var(--line); background: var(--surface-raised); }.settings-header__icon { display: grid; width: 36px; height: 36px; place-items: center; border-radius: 9px; color: #fff; background: var(--green-700); }.settings-header h2, .settings-header p { margin: 0; }.settings-header h2 { font-size: 18px; }.settings-header p { margin-top: 2px; color: var(--ink-500); font-size: 11px; }
.settings-split { display: grid; flex: 1; min-height: 0; grid-template-columns: 260px minmax(0, 1fr); }.settings-sidebar { display: flex; min-height: 0; flex-direction: column; gap: 6px; padding: 18px 12px; border-right: 1px solid var(--line); background: var(--surface-subtle); }.settings-sidebar button { display: grid; grid-template-columns: 34px minmax(0, 1fr) 16px; align-items: center; gap: 9px; min-height: 58px; padding: 8px 10px; border: 0; border-radius: 10px; color: var(--ink-700); background: transparent; text-align: left; cursor: pointer; }.settings-sidebar button:hover { background: var(--surface-hover); }.settings-sidebar button.active { color: var(--green-900); background: var(--surface-green-selected); }.settings-sidebar__icon { display: grid; width: 32px; height: 32px; place-items: center; border-radius: 8px; color: var(--green-700); background: var(--surface-green-soft); }.settings-sidebar strong, .settings-sidebar small { display: block; }.settings-sidebar strong { font-size: 13px; }.settings-sidebar small { margin-top: 3px; overflow: hidden; color: var(--ink-500); font-size: 10px; text-overflow: ellipsis; white-space: nowrap; }.settings-sidebar__chevron { color: var(--ink-500); }.settings-sidebar > p { display: flex; align-items: center; gap: 7px; margin: auto 8px 0; padding-top: 14px; border-top: 1px solid var(--line); color: var(--ink-500); font-size: 10px; }
.settings-detail { min-height: 0; overflow-y: auto; scrollbar-width: none; }.settings-detail::-webkit-scrollbar { display: none; }.settings-pane { width: min(720px, calc(100% - 64px)); margin: 0 auto; padding: 36px 0 48px; }.settings-pane > header { margin-bottom: 24px; }.settings-pane > header p { margin: 0 0 5px; color: var(--green-700); font-size: 10px; font-weight: 750; letter-spacing: .1em; }.settings-pane > header h3 { margin: 0; font-size: 23px; }.settings-pane > header span { display: block; margin-top: 7px; color: var(--ink-500); font-size: 12px; }
.settings-group { overflow: hidden; margin-bottom: 18px; border: 1px solid var(--line); border-radius: 12px; background: var(--surface-raised); box-shadow: 0 1px 2px rgb(31 53 34 / 3%); }.setting-row, .cache-row { display: flex; min-height: 68px; align-items: center; justify-content: space-between; gap: 24px; padding: 12px 16px; }.setting-row + .setting-row { border-top: 1px solid var(--line); }.setting-row > span, .cache-row > span, .scale-heading > span { min-width: 0; }.setting-row strong, .setting-row small, .cache-row strong, .cache-row small, .scale-heading strong, .scale-heading small { display: block; }.setting-row strong, .cache-row strong, .scale-heading strong { font-size: 13px; }.setting-row small, .cache-row small, .scale-heading small { margin-top: 4px; color: var(--ink-500); font-size: 10px; line-height: 1.4; }.setting-row select { min-width: 190px; height: 34px; padding: 0 30px 0 10px; border: 1px solid #c8d3c9; border-radius: 7px; color: var(--ink-900); background: var(--surface-input); }.switch { width: 38px; height: 22px; accent-color: var(--green-700); }
.scale-group { padding: 17px 20px 14px; }.scale-heading { display: flex; align-items: flex-start; justify-content: space-between; gap: 20px; }.scale-heading output { padding: 4px 8px; border-radius: 7px; color: var(--green-900); background: var(--surface-green-soft); font-size: 12px; font-weight: 650; }.stepped-slider { display: grid; grid-template-columns: 22px 1fr 22px; align-items: center; gap: 12px; margin-top: 18px; color: var(--ink-500); }.stepped-slider input { width: 100%; accent-color: var(--green-700); cursor: pointer; }.scale-labels { display: flex; justify-content: space-between; padding: 3px 34px 0; color: var(--ink-500); font-size: 9px; }
.shortcut-grid { display: grid; grid-template-columns: 1fr auto; }.shortcut-grid > span, .shortcut-grid kbd { padding: 10px 14px; border-bottom: 1px solid var(--line); font-size: 11px; }.shortcut-grid > :nth-last-child(-n + 2) { border-bottom: 0; }.shortcut-grid kbd { min-width: 180px; border-left: 1px solid var(--line); color: var(--green-900); background: var(--surface-subtle); font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; text-align: center; }.cache-row > div { display: grid; justify-items: end; gap: 6px; }.cache-row > div small { color: var(--ink-500); font-size: 9px; }.secondary-button { height: 34px; padding: 0 14px; border: 1px solid #bdcabf; border-radius: 7px; color: var(--green-900); background: var(--surface-raised); cursor: pointer; }.secondary-button:disabled { opacity: .55; cursor: wait; }
@media (max-width: 980px) { .settings-split { grid-template-columns: 220px minmax(0, 1fr); }.settings-pane { width: calc(100% - 40px); }.setting-row { gap: 14px; }.setting-row select { min-width: 160px; } }
</style>
