<script setup lang="ts">
import { Monitor, Settings2 } from "@lucide/vue";
import type { AppTheme, CacheCleanupPolicy, ShortcutProfile } from "../composables/useAppSettings";

defineProps<{
  usesMacShortcuts: boolean;
  shortcutRows: Array<[string, string]>;
  cacheCleaning: boolean;
  lastCacheCleanupAt: number | null;
}>();

const theme = defineModel<AppTheme>("theme", { required: true });
const fontScale = defineModel<number>("fontScale", { required: true });
const shortcutProfile = defineModel<ShortcutProfile>("shortcutProfile", { required: true });
const trackpadOptimized = defineModel<boolean>("trackpadOptimized", { required: true });
const autoSaveDelayMs = defineModel<number>("autoSaveDelayMs", { required: true });
const cacheCleanupPolicy = defineModel<CacheCleanupPolicy>("cacheCleanupPolicy", { required: true });

const emit = defineEmits<{
  applyUi: [];
  applyInteraction: [];
  applyPersistence: [];
  clearCache: [];
}>();
</script>

<template>
  <section class="aux-view settings-view">
    <Settings2 :size="28" />
    <h2>设置</h2>
    <p>工程内容完全离线保存；自动保存提交完整 Revision，缓存清理不删除正式历史。</p>
    <div class="settings-card">
      <h3>外观</h3>
      <label>显示主题<select v-model="theme" @change="emit('applyUi')"><option value="light">明亮</option><option value="eye">护眼</option></select></label>
      <label>界面缩放 <input v-model.number="fontScale" type="range" min="85" max="130" step="5" @change="emit('applyUi')" /><output>{{ fontScale }}%</output></label>
      <h3>macOS 与快捷键</h3>
      <label>快捷键布局<select v-model="shortcutProfile" @change="emit('applyInteraction')"><option value="auto">自动识别（推荐）</option><option value="macos">macOS · Command</option><option value="windows">Windows / Linux · Ctrl</option></select><output>{{ usesMacShortcuts ? '⌘' : 'Ctrl' }}</output></label>
      <label class="toggle-setting">触控板优化<input v-model="trackpadOptimized" type="checkbox" @change="emit('applyInteraction')" /><span>{{ trackpadOptimized ? '已开启' : '已关闭' }}</span></label>
      <small>开启后，双指滚动会立即接管并中止查找跳转动画；滚动保持纵向惯性，拖拽仅从手柄开始，避免轻触误拖。</small>
      <div class="shortcut-grid" aria-label="当前快捷键说明"><template v-for="row in shortcutRows" :key="row[0]"><span>{{ row[0] }}</span><kbd>{{ row[1] }}</kbd></template></div>
      <small class="shortcut-note">macOS 遵循系统习惯：使用 Command 作为主修饰键、Shift-Command-Z 重做；输入框内保留系统原生撤销，不会被工程历史抢占。</small>
      <h3>自动保存与历史</h3>
      <label>保存延迟<select v-model.number="autoSaveDelayMs" @change="emit('applyPersistence')"><option :value="1000">1 秒</option><option :value="3000">3 秒（推荐）</option><option :value="5000">5 秒</option><option :value="10000">10 秒</option><option :value="30000">30 秒</option></select><output>{{ autoSaveDelayMs / 1000 }}s</output></label>
      <small>停止输入达到该时间后，将当前句段作为一个完整 ChangeSet 写入 Revision 历史。</small>
      <label>缓存清理<select v-model="cacheCleanupPolicy" @change="emit('applyPersistence')"><option value="startup">每次启动</option><option value="weekly">每 7 天（推荐）</option><option value="monthly">每 30 天</option><option value="never">从不自动清理</option></select></label>
      <div class="cache-actions"><button class="secondary-button" type="button" :disabled="cacheCleaning" @click="emit('clearCache')">{{ cacheCleaning ? '清理中…' : '立即清理派生缓存' }}</button><span v-if="lastCacheCleanupAt">上次：{{ new Date(lastCacheCleanupAt).toLocaleString('zh-CN', { hour12: false }) }}</span></div>
      <small>仅清理可重建的 <code>.jm/cache</code>；<code>revisions/</code> 是正式历史并永久保留。</small>
      <span><Monitor :size="16" />不连接云端服务，不上传原文或译文。</span>
    </div>
  </section>
</template>

<style scoped>
.settings-view { justify-content: flex-start; max-width: 700px; overflow: auto; }
.settings-card { display: grid; gap: 14px; width: min(630px, 76vw); padding: 20px; border: 1px solid var(--line); border-radius: 8px; color: var(--ink-700); background: var(--paper); }
.settings-card h3 { margin: 4px 0 0; padding-bottom: 8px; border-bottom: 1px solid var(--line); color: var(--ink-900); font-size: 14px; }
.settings-card h3:not(:first-child) { margin-top: 10px; }
.settings-card label { display: grid; grid-template-columns: 115px 1fr auto; align-items: center; gap: 10px; font-size: 13px; }
.settings-card select { height: 34px; padding: 0 9px; border: 1px solid #c8d3c9; border-radius: 5px; color: var(--ink-900); background: var(--paper); }
.settings-card output { min-width: 42px; color: var(--green-900); }
.settings-card > small { margin-top: -7px; padding-left: 125px; color: var(--ink-500); line-height: 1.5; }
.settings-card code { padding: 1px 4px; border-radius: 3px; background: var(--surface-subtle); font-size: 10px; }
.settings-card > span { display: flex; align-items: center; gap: 8px; padding-top: 13px; border-top: 1px solid var(--line); color: var(--green-700); font-size: 12px; }
.toggle-setting input { justify-self: start; width: 17px; height: 17px; accent-color: var(--green-700); }
.toggle-setting span { min-width: 48px; color: var(--green-900); font-size: 12px; }
.shortcut-grid { display: grid; grid-template-columns: 1fr auto; gap: 0; margin-left: 125px; overflow: hidden; border: 1px solid var(--line); border-radius: 6px; background: var(--surface-subtle); }
.shortcut-grid > span, .shortcut-grid kbd { padding: 7px 10px; border-bottom: 1px solid var(--line); font-size: 11px; }
.shortcut-grid > :nth-last-child(-n + 2) { border-bottom: 0; }
.shortcut-grid kbd { min-width: 130px; border-left: 1px solid var(--line); color: var(--green-900); background: var(--surface-raised); font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; text-align: center; }
.settings-card > .shortcut-note { margin-top: -5px; }
.cache-actions { display: flex; align-items: center; gap: 12px; padding-left: 125px; }
.cache-actions span { color: var(--ink-500); font-size: 11px; }
.secondary-button { display: inline-flex; align-items: center; justify-content: center; gap: 6px; height: 38px; padding: 0 17px; border: 1px solid #bdcabf; border-radius: 6px; color: var(--ink-700); background: var(--surface-raised); cursor: pointer; }
.secondary-button:disabled { opacity: .55; cursor: wait; }
</style>
