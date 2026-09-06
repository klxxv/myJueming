<script setup lang="ts">
import { computed, ref, watchEffect, type Component } from "vue";
import {
  Accessibility as AccessibilityIcon,
  ArrowLeft,
  ChevronRight,
  Database,
  Gauge,
  HardDrive,
  Info,
  Keyboard,
  Monitor,
  Palette,
  RotateCcw,
  Search,
  Settings2,
  ShieldCheck,
  SlidersHorizontal,
  X,
} from "@lucide/vue";
import type { AppSettingsEnvelope } from "../settings/schema";
import {
  isSettingsCapabilityAvailable,
  type SettingsCapabilities,
  type SettingsCapabilityId,
} from "../settings/capabilities";

type SettingsSectionDefinition = {
  id: SettingsCapabilityId;
  group: "应用偏好" | "数据与安全" | "系统";
  label: string;
  description: string;
  icon: Component;
};
type DetailId = "motion-global" | "interface-motion" | "motion-performance";
type SearchEntry = {
  label: string;
  path: string;
  section: SettingsCapabilityId;
  detail?: DetailId;
  keywords: string;
};

const settings = defineModel<AppSettingsEnvelope>("settings", { required: true });
const props = defineProps<{
  capabilities: SettingsCapabilities;
  usesMacShortcuts: boolean;
  shortcutRows: Array<[string, string]>;
  cacheCleaning: boolean;
  lastCacheCleanupAt: number | null;
  settingsSaving: boolean;
  settingsSaveError: string | null;
  effectiveMotionMode: "standard" | "reduced" | "off";
  motionSummary: string;
  systemReducedMotion: boolean;
  projectOpen: boolean;
  appVersion: string;
}>();
const emit = defineEmits<{
  applyGeneral: [];
  applyUi: [];
  applyAccessibility: [];
  applyInteraction: [];
  applyMotion: [];
  applyPersistence: [];
  clearCache: [];
  rememberSection: [section: string];
  resetAll: [];
}>();

const sectionDefinitions: SettingsSectionDefinition[] = [
  { id: "account", group: "应用偏好", label: "账号与登录", description: "账号、会话与设置同步", icon: Settings2 },
  { id: "general", group: "应用偏好", label: "通用", description: "启动、工作区与性能", icon: SlidersHorizontal },
  { id: "appearance", group: "应用偏好", label: "外观与阅读", description: "主题、文字与界面尺寸", icon: Palette },
  { id: "accessibility", group: "应用偏好", label: "辅助功能", description: "动态、对比度与操作辅助", icon: AccessibilityIcon },
  { id: "input", group: "应用偏好", label: "键盘与触控板", description: "快捷键和滚动行为", icon: Keyboard },
  { id: "pet", group: "应用偏好", label: "桌宠", description: "角色、活动区域与动作", icon: Monitor },
  { id: "corpus", group: "数据与安全", label: "语料库与工程", description: "本地语料和工程默认值", icon: Database },
  { id: "upload", group: "数据与安全", label: "上传与发布", description: "上传、许可与隐私检查", icon: Database },
  { id: "community", group: "数据与安全", label: "社区与贡献", description: "贡献、积分和社区身份", icon: Database },
  { id: "agent", group: "数据与安全", label: "AI 与 Agent", description: "Provider、权限与运行记录", icon: Settings2 },
  { id: "plugins", group: "数据与安全", label: "插件与集成", description: "Slot、权限和插件更新", icon: Settings2 },
  { id: "persistence", group: "数据与安全", label: "保存与历史", description: "自动保存与 Revision", icon: Database },
  { id: "storage", group: "数据与安全", label: "存储空间", description: "缓存策略与本地空间", icon: HardDrive },
  { id: "privacy", group: "数据与安全", label: "隐私与安全", description: "本地模式与数据边界", icon: ShieldCheck },
  { id: "notifications", group: "数据与安全", label: "通知", description: "任务和系统通知", icon: Monitor },
  { id: "about", group: "系统", label: "更新与关于", description: "版本、诊断与许可", icon: Info },
];

const visibleSections = computed(() => sectionDefinitions.filter((section) =>
  isSettingsCapabilityAvailable(props.capabilities, section.id)));
const sectionGroups = computed(() => ["应用偏好", "数据与安全", "系统"].map((group) => ({
  label: group,
  sections: visibleSections.value.filter((section) => section.group === group),
})).filter((group) => group.sections.length));

const activeSection = ref<SettingsCapabilityId>("appearance");
const detail = ref<DetailId | null>(null);
const searchQuery = ref("");

watchEffect(() => {
  const lastSection = settings.value.device.navigation.lastSection as SettingsCapabilityId;
  if (visibleSections.value.some((section) => section.id === lastSection)) activeSection.value = lastSection;
  if (!visibleSections.value.some((section) => section.id === activeSection.value)) {
    activeSection.value = visibleSections.value[0]?.id ?? "appearance";
  }
});

const activeDefinition = computed(() => visibleSections.value.find((section) => section.id === activeSection.value));
const searchEntries: SearchEntry[] = [
  { label: "启动后打开", path: "通用 > 启动行为", section: "general", keywords: "启动 上次工程 欢迎页" },
  { label: "默认工作模式", path: "通用 > 启动行为", section: "general", keywords: "审阅 编辑 排序" },
  { label: "动效性能策略", path: "通用 > 性能与节能", section: "general", detail: "motion-performance", keywords: "后台 暂停 性能 流畅" },
  { label: "显示主题", path: "外观与阅读 > 显示主题", section: "appearance", keywords: "明亮 护眼" },
  { label: "字体大小", path: "外观与阅读 > 阅读文字", section: "appearance", keywords: "字号 正文" },
  { label: "界面空间大小", path: "外观与阅读 > 界面尺寸", section: "appearance", keywords: "缩放 紧凑 宽松" },
  { label: "行间距", path: "外观与阅读 > 阅读文字", section: "appearance", keywords: "紧凑 标准 宽松" },
  { label: "界面动画", path: "外观与阅读 > 界面动效", section: "appearance", detail: "interface-motion", keywords: "转场 跳转 高亮 拖拽" },
  { label: "动态效果", path: "辅助功能 > 动态与感知", section: "accessibility", detail: "motion-global", keywords: "减少 关闭 动画 系统" },
  { label: "增强对比度", path: "辅助功能 > 视觉辅助", section: "accessibility", keywords: "对比 高对比" },
  { label: "强化键盘焦点", path: "辅助功能 > 视觉辅助", section: "accessibility", keywords: "键盘 焦点" },
  { label: "更大点击区域", path: "辅助功能 > 操作辅助", section: "accessibility", keywords: "按钮 点击 触控" },
  { label: "快捷键布局", path: "键盘与触控板 > 快捷键布局", section: "input", keywords: "macOS Windows Ctrl Command" },
  { label: "触控板优化", path: "键盘与触控板 > 触控板", section: "input", keywords: "滚动 拖拽" },
  { label: "保存延迟", path: "保存与历史 > 自动保存", section: "persistence", keywords: "自动保存 ChangeSet" },
  { label: "缓存清理", path: "存储空间 > 缓存", section: "storage", keywords: "cache 清理 空间" },
  { label: "本地模式", path: "隐私与安全 > 数据边界", section: "privacy", keywords: "离线 上传 隐私" },
  { label: "应用版本", path: "更新与关于 > 版本信息", section: "about", keywords: "版本 构建" },
];
const searchResults = computed(() => {
  const query = searchQuery.value.trim().toLocaleLowerCase();
  if (!query) return [];
  const visibleIds = new Set(visibleSections.value.map((section) => section.id));
  return searchEntries.filter((entry) => visibleIds.has(entry.section)
    && `${entry.label} ${entry.path} ${entry.keywords}`.toLocaleLowerCase().includes(query));
});

function chooseSection(section: SettingsCapabilityId) {
  activeSection.value = section;
  detail.value = null;
  if (settings.value.device.general.rememberSettingsLocation) emit("rememberSection", section);
}

function openDetail(nextDetail: DetailId, sourceSection = activeSection.value) {
  activeSection.value = sourceSection;
  detail.value = nextDetail;
  if (settings.value.device.general.rememberSettingsLocation) emit("rememberSection", sourceSection);
}

function chooseSearchResult(entry: SearchEntry) {
  searchQuery.value = "";
  activeSection.value = entry.section;
  detail.value = entry.detail ?? null;
  if (settings.value.device.general.rememberSettingsLocation) emit("rememberSection", entry.section);
}

function requestReset() {
  if (window.confirm("恢复所有本机设置为默认值？这不会修改任何 .jm 工程或 Revision。")) emit("resetAll");
}

const formatCleanupTime = computed(() => props.lastCacheCleanupAt
  ? new Date(props.lastCacheCleanupAt).toLocaleString("zh-CN", { hour12: false })
  : "尚未清理");
</script>

<template>
  <section class="settings-view">
    <header class="settings-header">
      <span class="settings-header__icon"><Settings2 :size="20" /></span>
      <div><h2>设置</h2><p>本机偏好不会写入 .jm 工程</p></div>
      <span v-if="settingsSaving" class="settings-save-state">正在保存…</span>
      <span v-else-if="settingsSaveError" class="settings-save-state settings-save-state--error">保存失败</span>
      <span v-else class="settings-save-state">已保存在这台设备上</span>
    </header>

    <div class="settings-split">
      <nav class="settings-sidebar" aria-label="设置分类">
        <label class="settings-search">
          <Search :size="15" />
          <input v-model="searchQuery" type="search" placeholder="搜索设置" aria-label="搜索设置" />
          <button v-if="searchQuery" type="button" title="清除搜索" @click="searchQuery = ''"><X :size="14" /></button>
        </label>

        <div v-if="searchQuery" class="settings-search-results" aria-live="polite">
          <button v-for="entry in searchResults" :key="`${entry.section}-${entry.label}`" type="button" @click="chooseSearchResult(entry)">
            <span><strong>{{ entry.label }}</strong><small>{{ entry.path }}</small></span><ChevronRight :size="14" />
          </button>
          <p v-if="!searchResults.length">没有匹配的可用设置</p>
        </div>

        <template v-else v-for="group in sectionGroups" :key="group.label">
          <p class="settings-sidebar__group">{{ group.label }}</p>
          <button v-for="section in group.sections" :key="section.id" type="button" class="settings-sidebar__item" :class="{ active: activeSection === section.id }" @click="chooseSection(section.id)">
            <span class="settings-sidebar__icon"><component :is="section.icon" :size="17" /></span>
            <strong>{{ section.label }}</strong><ChevronRight :size="14" />
          </button>
        </template>

        <p class="settings-local-status"><Monitor :size="14" />本地模式 · 当前无联网传输</p>
      </nav>

      <div class="settings-detail">
        <section v-if="detail" class="settings-pane settings-pane--detail">
          <button class="settings-back" type="button" @click="detail = null"><ArrowLeft :size="16" />{{ activeDefinition?.label }}</button>

          <template v-if="detail === 'motion-global'">
            <header><p>辅助功能 · 动态与感知</p><h2>动态效果</h2><span>统一限制决明中的非必要动画；系统“减少动态效果”拥有更高优先级。</span></header>
            <div class="settings-group">
              <label class="setting-row"><span><strong>全局动效模式</strong><small>当前有效状态：{{ effectiveMotionMode === 'standard' ? '标准' : effectiveMotionMode === 'reduced' ? '已减少' : '已关闭' }}</small></span><select v-model="settings.device.motion.mode" @change="emit('applyMotion')"><option value="system">跟随系统（推荐）</option><option value="standard">标准</option><option value="reduced">减少</option><option value="off">关闭非必要动画</option></select></label>
              <div class="setting-row setting-row--readonly"><span><strong>系统偏好</strong><small>应用不能用更强动画覆盖系统辅助功能</small></span><output>{{ systemReducedMotion ? '系统已要求减少' : '系统未要求减少' }}</output></div>
            </div>
            <p class="settings-note">关闭非必要动画后，保存、错误、拖拽目标和跳转位置仍使用静态文字、图标和边框反馈。</p>
          </template>

          <template v-else-if="detail === 'interface-motion'">
            <header><p>外观与阅读 · 界面动效</p><h2>界面动画</h2><span>逐项控制页面反馈；所有选项仍受全局动效模式限制。</span></header>
            <div class="settings-group">
              <label class="setting-row"><span><strong>页面与面板转场</strong><small>关闭后页面即时切换并保留键盘焦点</small></span><input v-model="settings.device.motion.interfaceTransitions" class="switch" type="checkbox" @change="emit('applyMotion')" /></label>
              <label class="setting-row"><span><strong>平滑跳转</strong><small>查找、书签和搜索结果滚动</small></span><input v-model="settings.device.motion.smoothNavigation" class="switch" type="checkbox" @change="emit('applyMotion')" /></label>
              <label class="setting-row"><span><strong>高亮淡出</strong><small>关闭后使用短时静态边框</small></span><input v-model="settings.device.motion.highlightFade" class="switch" type="checkbox" @change="emit('applyMotion')" /></label>
              <label class="setting-row"><span><strong>拖拽反馈动画</strong><small>关闭后仍显示静态插入位置</small></span><input v-model="settings.device.motion.dragFeedback" class="switch" type="checkbox" @change="emit('applyMotion')" /></label>
              <label class="setting-row"><span><strong>成功微动效</strong><small>关闭后保留文字确认</small></span><input v-model="settings.device.motion.successMotion" class="switch" type="checkbox" @change="emit('applyMotion')" /></label>
            </div>
          </template>

          <template v-else>
            <header><p>通用 · 性能与节能</p><h2>动效性能策略</h2><span>窗口不在前台时暂停非必要动画，减少后台资源使用。</span></header>
            <div class="settings-group">
              <label class="setting-row"><span><strong>失去焦点时暂停</strong><small>不影响自动保存、导出或其他数据任务</small></span><input v-model="settings.device.motion.pauseWhenUnfocused" class="switch" type="checkbox" @change="emit('applyMotion')" /></label>
              <div class="setting-row setting-row--readonly"><span><strong>当前动效状态</strong><small>{{ motionSummary }}</small></span><output>{{ effectiveMotionMode === 'off' ? '已暂停' : '前台运行' }}</output></div>
            </div>
          </template>
        </section>

        <section v-else-if="activeSection === 'general'" class="settings-pane">
          <header><p>应用偏好</p><h2>通用</h2><span>设置启动路径、默认工作模式和后台动效行为。</span></header>
          <h3>启动行为</h3>
          <div class="settings-group">
            <label class="setting-row"><span><strong>启动后打开</strong><small>“上次工程”只打开仍然存在的本地工程</small></span><select v-model="settings.device.general.startupDestination" @change="emit('applyGeneral')"><option value="welcome">欢迎页</option><option value="last-project">上次工程</option><option value="project-picker">工程选择器</option></select></label>
            <label class="setting-row"><span><strong>默认工作模式</strong><small>新建或打开工程后的初始模式</small></span><select v-model="settings.device.general.defaultWorkspace" @change="emit('applyGeneral')"><option value="review">审阅</option><option value="edit">编辑</option><option value="order">排序</option></select></label>
            <label class="setting-row"><span><strong>记住设置位置</strong><small>下次打开设置时回到最后一个分类</small></span><input v-model="settings.device.general.rememberSettingsLocation" class="switch" type="checkbox" @change="emit('applyGeneral')" /></label>
          </div>
          <h3>性能与节能</h3>
          <button class="settings-group settings-link-row" type="button" @click="openDetail('motion-performance', 'general')"><span><Gauge :size="18" /><span><strong>动效性能策略</strong><small>后台暂停与当前有效状态</small></span></span><span>{{ settings.device.motion.pauseWhenUnfocused ? '自动' : '持续运行' }}<ChevronRight :size="15" /></span></button>
        </section>

        <section v-else-if="activeSection === 'appearance'" class="settings-pane">
          <header><p>应用偏好</p><h2>外观与阅读</h2><span>分别调整正文阅读、应用尺寸和界面反馈。</span></header>
          <h3>显示主题</h3>
          <div class="settings-group"><label class="setting-row"><span><strong>显示主题</strong><small>选择适合当前环境的界面色彩</small></span><select v-model="settings.device.appearance.theme" @change="emit('applyUi')"><option value="light">明亮</option><option value="eye">护眼</option></select></label></div>
          <h3>阅读文字</h3>
          <div class="settings-group scale-group"><div class="scale-heading"><span><strong>字体大小</strong><small>只调整双语正文和编辑文字</small></span><output>{{ settings.device.appearance.fontScale }}%</output></div><div class="stepped-slider"><span>小</span><input v-model.number="settings.device.appearance.fontScale" aria-label="字体大小" type="range" min="90" max="130" step="10" @input="emit('applyUi')" /><span>大</span></div></div>
          <div class="settings-group"><label class="setting-row"><span><strong>行间距</strong><small>调整双语正文的纵向阅读密度</small></span><select v-model="settings.device.appearance.lineSpacing" @change="emit('applyUi')"><option value="compact">紧凑</option><option value="standard">标准</option><option value="relaxed">宽松</option></select></label></div>
          <h3>界面尺寸</h3>
          <div class="settings-group scale-group"><div class="scale-heading"><span><strong>界面空间大小</strong><small>调整按钮、边栏和控件的整体比例</small></span><output>{{ settings.device.appearance.uiScale }}%</output></div><div class="stepped-slider"><span>紧</span><input v-model.number="settings.device.appearance.uiScale" aria-label="界面空间大小" type="range" min="85" max="115" step="5" @input="emit('applyUi')" /><span>松</span></div></div>
          <div class="settings-group">
            <label class="setting-row"><span><strong>列表密度</strong><small>不会降低可点击区域的最小尺寸</small></span><select v-model="settings.device.appearance.listDensity" @change="emit('applyUi')"><option value="compact">紧凑</option><option value="standard">标准</option><option value="comfortable">宽松</option></select></label>
            <label class="setting-row"><span><strong>侧栏尺寸</strong><small>设置应用主导航的默认宽度</small></span><select v-model="settings.device.appearance.sidebarSize" @change="emit('applyUi')"><option value="auto">自动</option><option value="compact">紧凑</option><option value="wide">宽</option></select></label>
            <label class="setting-row"><span><strong>双栏宽度</strong><small>改变中英文阅读空间比例</small></span><select v-model="settings.device.appearance.columnBalance" @change="emit('applyUi')"><option value="equal">均分</option><option value="source-wide">中文较宽</option><option value="target-wide">英文较宽</option></select></label>
            <label class="setting-row"><span><strong>Alignment 辅助标签</strong><small>关闭后仍保留关系本身和非颜色状态</small></span><input v-model="settings.device.appearance.alignmentHints" class="switch" type="checkbox" @change="emit('applyUi')" /></label>
          </div>
          <h3>界面动效</h3>
          <button class="settings-group settings-link-row" type="button" @click="openDetail('interface-motion', 'appearance')"><span><Gauge :size="18" /><span><strong>界面动画</strong><small>转场、跳转、高亮和拖拽反馈</small></span></span><span>{{ motionSummary }}<ChevronRight :size="15" /></span></button>
        </section>

        <section v-else-if="activeSection === 'accessibility'" class="settings-pane">
          <header><p>应用偏好</p><h2>辅助功能</h2><span>基础键盘可达性和非颜色状态表达始终开启。</span></header>
          <h3>动态与感知</h3>
          <button class="settings-group settings-link-row" type="button" @click="openDetail('motion-global', 'accessibility')"><span><AccessibilityIcon :size="18" /><span><strong>动态效果</strong><small>跟随系统并集中限制非必要动画</small></span></span><span>{{ motionSummary }}<ChevronRight :size="15" /></span></button>
          <h3>视觉辅助</h3>
          <div class="settings-group">
            <label class="setting-row"><span><strong>增强对比度</strong><small>保持原文、译文和操作语义色不变</small></span><select v-model="settings.device.accessibility.contrast" @change="emit('applyAccessibility')"><option value="system">跟随系统</option><option value="standard">标准</option><option value="increased">增强</option></select></label>
            <label class="setting-row"><span><strong>强化键盘焦点</strong><small>为当前焦点增加更清晰的边框</small></span><input v-model="settings.device.accessibility.enhancedFocus" class="switch" type="checkbox" @change="emit('applyAccessibility')" /></label>
            <label class="setting-row"><span><strong>更大点击区域</strong><small>放大紧凑按钮和拖拽手柄的命中范围</small></span><input v-model="settings.device.accessibility.largerTargets" class="switch" type="checkbox" @change="emit('applyAccessibility')" /></label>
            <label class="setting-row"><span><strong>减少透明效果</strong><small>将浮层和抽屉改为不透明表面</small></span><input v-model="settings.device.accessibility.reduceTransparency" class="switch" type="checkbox" @change="emit('applyAccessibility')" /></label>
          </div>
        </section>

        <section v-else-if="activeSection === 'input'" class="settings-pane">
          <header><p>应用偏好</p><h2>键盘与触控板</h2><span>按操作系统习惯调整快捷键和滚动方式。</span></header>
          <h3>输入设备</h3>
          <div class="settings-group">
            <label class="setting-row"><span><strong>快捷键布局</strong><small>当前主修饰键：{{ usesMacShortcuts ? 'Command' : 'Ctrl' }}</small></span><select v-model="settings.device.input.shortcutProfile" @change="emit('applyInteraction')"><option value="auto">自动识别（推荐）</option><option value="macos">macOS · Command</option><option value="windows">Windows / Linux · Ctrl</option></select></label>
            <label class="setting-row"><span><strong>触控板优化</strong><small>滚动会取消跳转动画，排序只从手柄开始</small></span><input v-model="settings.device.input.trackpadOptimized" class="switch" type="checkbox" @change="emit('applyInteraction')" /></label>
          </div>
          <h3>当前快捷键</h3>
          <div class="settings-group shortcut-grid" aria-label="当前快捷键说明"><template v-for="row in shortcutRows" :key="row[0]"><span>{{ row[0] }}</span><kbd>{{ row[1] }}</kbd></template></div>
        </section>

        <section v-else-if="activeSection === 'persistence'" class="settings-pane">
          <header><p>数据与安全</p><h2>保存与历史</h2><span>每次成功 canonical ChangeSet 都会保留完整 Revision。</span></header>
          <h3>自动保存</h3>
          <div class="settings-group"><label class="setting-row"><span><strong>保存延迟</strong><small>停止输入后自动写入一个完整 ChangeSet</small></span><select v-model.number="settings.device.persistence.autoSaveDelayMs" @change="emit('applyPersistence')"><option :value="1000">1 秒</option><option :value="3000">3 秒（推荐）</option><option :value="5000">5 秒</option><option :value="10000">10 秒</option><option :value="30000">30 秒</option></select></label></div>
          <h3>Revision 历史</h3>
          <div class="settings-group">
            <div class="setting-row setting-row--readonly"><span><strong>完整历史</strong><small>Undo、Redo 和 Restore 也会产生新 Revision</small></span><output>始终保留</output></div>
            <div class="setting-row setting-row--readonly"><span><strong>自动删除历史</strong><small>Revision 不属于缓存</small></span><output>不提供</output></div>
          </div>
        </section>

        <section v-else-if="activeSection === 'storage'" class="settings-pane">
          <header><p>数据与安全</p><h2>存储空间</h2><span>只清理可重建的派生缓存，不触碰正式工程历史。</span></header>
          <h3>缓存策略</h3>
          <div class="settings-group"><label class="setting-row"><span><strong>自动清理</strong><small>只清理 `.jm/cache`</small></span><select v-model="settings.device.persistence.cacheCleanupPolicy" @change="emit('applyPersistence')"><option value="startup">每次启动</option><option value="weekly">每 7 天（推荐）</option><option value="monthly">每 30 天</option><option value="never">从不自动清理</option></select></label></div>
          <h3>手动清理</h3>
          <div class="settings-group cache-row"><span><strong>立即清理派生缓存</strong><small>{{ projectOpen ? `上次：${formatCleanupTime}` : '打开本地工程后可清理' }}</small></span><div><button class="secondary-button" type="button" :disabled="cacheCleaning || !projectOpen" @click="emit('clearCache')">{{ cacheCleaning ? '清理中…' : '立即清理' }}</button><small>revisions/ 不会被删除</small></div></div>
        </section>

        <section v-else-if="activeSection === 'privacy'" class="settings-pane">
          <header><p>数据与安全</p><h2>隐私与安全</h2><span>当前版本没有账号、上传、远程 AI 或外部插件运行时。</span></header>
          <h3>本地模式</h3>
          <div class="settings-group">
            <div class="setting-row setting-row--readonly"><span><strong>工程正文与译文</strong><small>只通过 typed KernelClient 访问本地 `.jm` 工程</small></span><output>不上传</output></div>
            <div class="setting-row setting-row--readonly"><span><strong>在线数据传输</strong><small>没有已绑定的在线能力</small></span><output>0 个服务</output></div>
            <div class="setting-row setting-row--readonly"><span><strong>诊断和使用统计</strong><small>当前未接入遥测端点</small></span><output>不发送</output></div>
          </div>
        </section>

        <section v-else class="settings-pane">
          <header><p>系统</p><h2>更新与关于</h2><span>查看当前构建、设置存储状态和本地能力边界。</span></header>
          <h3>版本信息</h3>
          <div class="settings-group">
            <div class="setting-row setting-row--readonly"><span><strong>决明对齐器</strong><small>Jueming Aligner</small></span><output>v{{ appVersion }}</output></div>
            <div class="setting-row setting-row--readonly"><span><strong>设置格式</strong><small>支持旧版 localStorage 自动迁移</small></span><output>Schema v{{ settings.schemaVersion }}</output></div>
            <div class="setting-row setting-row--readonly"><span><strong>数据模式</strong><small>偏好设置与 `.jm` canonical data 分离</small></span><output>本地优先</output></div>
          </div>
          <h3>恢复</h3>
          <div class="settings-group reset-row"><span><strong>恢复本机默认设置</strong><small>不会修改工程、批注或 Revision</small></span><button class="secondary-button" type="button" @click="requestReset"><RotateCcw :size="15" />恢复默认</button></div>
        </section>
      </div>
    </div>
  </section>
</template>

<style scoped>
.settings-view { display: flex; flex-direction: column; width: 100%; height: 100%; min-height: 0; color: var(--ink-900); background: var(--surface-app); }
.settings-header { display: flex; flex: 0 0 70px; align-items: center; gap: 12px; padding: 0 22px; border-bottom: 1px solid var(--line); background: var(--surface-raised); }
.settings-header__icon { display: grid; width: 34px; height: 34px; place-items: center; border-radius: 9px; color: #fff; background: var(--green-700); }
.settings-header h2, .settings-header p { margin: 0; }.settings-header h2 { font-size: var(--jm-font-size-title-2); line-height: var(--jm-line-height-title-2); }.settings-header p { margin-top: 2px; color: var(--ink-500); font-size: var(--jm-font-size-callout); }
.settings-save-state { margin-left: auto; color: var(--green-700); font-size: var(--jm-font-size-callout); }.settings-save-state--error { color: #a34f4f; }
.settings-split { display: grid; flex: 1; min-height: 0; grid-template-columns: 248px minmax(0, 1fr); }
.settings-sidebar { display: flex; min-height: 0; flex-direction: column; overflow-y: auto; padding: 14px 10px; border-right: 1px solid var(--line); background: var(--surface-subtle); scrollbar-width: thin; }
.settings-search { display: flex; flex: 0 0 34px; align-items: center; gap: 7px; margin: 0 2px 10px; padding: 0 9px; border: 1px solid var(--line); border-radius: 9px; color: var(--ink-500); background: var(--surface-input); }
.settings-search:focus-within { border-color: #8fbf95; box-shadow: 0 0 0 3px rgb(47 129 67 / 12%); }.settings-search input { flex: 1; min-width: 0; border: 0; outline: 0; color: var(--ink-900); background: transparent; }.settings-search button { display: grid; padding: 2px; border: 0; place-items: center; color: var(--ink-500); background: transparent; cursor: pointer; }
.settings-sidebar__group { margin: 11px 10px 5px; color: var(--ink-500); font-size: var(--jm-font-size-subheadline); font-weight: var(--jm-font-weight-semibold); letter-spacing: .06em; }
.settings-sidebar__item { display: grid; grid-template-columns: 30px minmax(0, 1fr) 14px; min-height: 44px; align-items: center; gap: 8px; padding: 5px 9px; border: 0; border-radius: 9px; color: var(--ink-700); background: transparent; text-align: left; cursor: pointer; }.settings-sidebar__item:hover { background: var(--surface-hover); }.settings-sidebar__item.active { color: var(--green-900); background: var(--surface-green-selected); }.settings-sidebar__item strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: var(--jm-font-size-body); }.settings-sidebar__icon { display: grid; width: 28px; height: 28px; place-items: center; border-radius: 7px; color: var(--green-700); background: var(--surface-green-soft); }
.settings-local-status { display: flex; align-items: center; gap: 7px; margin: auto 7px 0; padding: 13px 3px 2px; border-top: 1px solid var(--line); color: var(--ink-500); font-size: var(--jm-font-size-subheadline); }
.settings-search-results { display: grid; gap: 3px; }.settings-search-results button { display: flex; align-items: center; justify-content: space-between; gap: 8px; min-height: 48px; padding: 7px 9px; border: 0; border-radius: 8px; background: transparent; text-align: left; cursor: pointer; }.settings-search-results button:hover { background: var(--surface-hover); }.settings-search-results strong, .settings-search-results small { display: block; }.settings-search-results small { margin-top: 2px; color: var(--ink-500); }.settings-search-results p { padding: 18px 10px; color: var(--ink-500); }
.settings-detail { min-height: 0; overflow-y: auto; scrollbar-width: thin; }.settings-pane { width: min(760px, calc(100% - 56px)); margin: 0 auto; padding: 31px 0 52px; }.settings-pane > header { margin-bottom: 23px; }.settings-pane > header p { margin: 0 0 5px; color: var(--green-700); font-size: var(--jm-font-size-subheadline); font-weight: var(--jm-font-weight-semibold); letter-spacing: .08em; }.settings-pane > header h2 { margin: 0; font-size: var(--jm-font-size-title-1); line-height: var(--jm-line-height-title-1); }.settings-pane > header span { display: block; margin-top: 7px; color: var(--ink-500); line-height: 1.5; }.settings-pane > h3 { margin: 22px 4px 8px; color: var(--ink-700); font-size: var(--jm-font-size-callout); font-weight: var(--jm-font-weight-semibold); }
.settings-back { display: inline-flex; align-items: center; gap: 6px; margin-bottom: 20px; padding: 5px 7px 5px 2px; border: 0; color: var(--green-700); background: transparent; cursor: pointer; }.settings-back:hover { color: var(--green-900); }
.settings-group { overflow: hidden; width: 100%; margin: 0 0 12px; border: 1px solid var(--line); border-radius: 12px; background: var(--surface-raised); box-shadow: 0 1px 2px rgb(31 53 34 / 3%); }
.setting-row, .cache-row, .reset-row { display: flex; min-height: 64px; align-items: center; justify-content: space-between; gap: 24px; padding: 11px 15px; }.setting-row + .setting-row { border-top: 1px solid var(--line); }.setting-row > span, .cache-row > span, .reset-row > span, .scale-heading > span { min-width: 0; }.setting-row strong, .setting-row small, .cache-row strong, .cache-row small, .reset-row strong, .reset-row small, .scale-heading strong, .scale-heading small { display: block; }.setting-row strong, .cache-row strong, .reset-row strong, .scale-heading strong { font-size: var(--jm-font-size-body); }.setting-row small, .cache-row small, .reset-row small, .scale-heading small { margin-top: 4px; color: var(--ink-500); font-size: var(--jm-font-size-callout); line-height: 1.4; }.setting-row select { min-width: 190px; height: 34px; padding: 0 28px 0 9px; border: 1px solid #c8d3c9; border-radius: 7px; color: var(--ink-900); background: var(--surface-input); }.setting-row output { flex: none; color: var(--green-900); font-size: var(--jm-font-size-callout); }.setting-row--readonly { background: var(--surface-raised); }
.switch { flex: none; width: 38px; height: 22px; accent-color: var(--green-700); }.scale-group { padding: 16px 18px 13px; }.scale-heading { display: flex; align-items: flex-start; justify-content: space-between; gap: 20px; }.scale-heading output { padding: 4px 8px; border-radius: 7px; color: var(--green-900); background: var(--surface-green-soft); font-size: var(--jm-font-size-callout); font-weight: var(--jm-font-weight-semibold); }.stepped-slider { display: grid; grid-template-columns: 24px 1fr 24px; align-items: center; gap: 11px; margin-top: 16px; color: var(--ink-500); text-align: center; }.stepped-slider input { width: 100%; accent-color: var(--green-700); cursor: pointer; }
.settings-link-row { display: flex; min-height: 64px; align-items: center; justify-content: space-between; gap: 20px; padding: 11px 15px; color: var(--ink-900); text-align: left; cursor: pointer; }.settings-link-row:hover { border-color: #b8ccb9; background: var(--surface-hover); }.settings-link-row > span { display: flex; align-items: center; gap: 10px; }.settings-link-row > span:last-child { flex: none; color: var(--ink-500); font-size: var(--jm-font-size-callout); }.settings-link-row strong, .settings-link-row small { display: block; }.settings-link-row small { margin-top: 4px; color: var(--ink-500); }
.shortcut-grid { display: grid; grid-template-columns: 1fr auto; }.shortcut-grid > span, .shortcut-grid kbd { padding: 10px 14px; border-bottom: 1px solid var(--line); font-size: var(--jm-font-size-callout); }.shortcut-grid > :nth-last-child(-n + 2) { border-bottom: 0; }.shortcut-grid kbd { min-width: 180px; border-left: 1px solid var(--line); color: var(--green-900); background: var(--surface-subtle); font-family: var(--jm-font-mono); text-align: center; }.cache-row > div { display: grid; justify-items: end; gap: 6px; }.cache-row > div small { color: var(--ink-500); font-size: var(--jm-font-size-subheadline); }.secondary-button { display: inline-flex; height: 34px; align-items: center; gap: 6px; padding: 0 13px; border: 1px solid #bdcabf; border-radius: 7px; color: var(--green-900); background: var(--surface-raised); cursor: pointer; }.secondary-button:disabled { opacity: .5; cursor: not-allowed; }.settings-note { margin: -2px 4px 18px; color: var(--ink-500); font-size: var(--jm-font-size-callout); line-height: 1.55; }
@media (max-width: 1080px) { .settings-split { grid-template-columns: 210px minmax(0, 1fr); }.settings-pane { width: calc(100% - 36px); }.setting-row { gap: 12px; }.setting-row select { min-width: 158px; } }
</style>
