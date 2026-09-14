import type { WorkspaceMode } from "./kernel-client";
import type { ReviewMessageKey } from "../i18n/review-messages";

export const PARALLEL_TUTORIAL_GLOBAL_CONFIG = Object.freeze({
  autoStartOnFirstLaunch: true,
  firstLaunchStorageKey: "jueming-parallel-tutorial-launched-v2",
});

export type TutorialPreset = "clear" | "complex-relation" | "content" | "unlinked" | "order" | "find" | "context";
export interface ParallelTutorialStep {
  id: string;
  titleKey: ReviewMessageKey;
  bodyKey: ReviewMessageKey;
  tipKey: ReviewMessageKey;
  selector: string;
  mode: WorkspaceMode;
  preset: TutorialPreset;
}

export const parallelTutorialSteps: ParallelTutorialStep[] = [
  { id: "welcome", titleKey: "tutorialWelcomeTitle", bodyKey: "tutorialWelcomeBody", tipKey: "tutorialWelcomeTip", selector: '[data-tutorial="help-button"]', mode: "review", preset: "clear" },
  { id: "columns", titleKey: "tutorialColumnsTitle", bodyKey: "tutorialColumnsBody", tipKey: "tutorialColumnsTip", selector: '[data-tutorial="parallel-columns"]', mode: "review", preset: "clear" },
  { id: "relation", titleKey: "tutorialRelationTitle", bodyKey: "tutorialRelationBody", tipKey: "tutorialRelationTip", selector: '[data-tutorial="relation-rail"]', mode: "review", preset: "complex-relation" },
  { id: "relations", titleKey: "tutorialRelationsTitle", bodyKey: "tutorialRelationsBody", tipKey: "tutorialRelationsTip", selector: '[data-tutorial="relation-tools"]', mode: "review", preset: "unlinked" },
  { id: "content", titleKey: "tutorialContentTitle", bodyKey: "tutorialContentBody", tipKey: "tutorialContentTip", selector: '[data-tutorial="content-tools"]', mode: "review", preset: "content" },
  { id: "edit", titleKey: "tutorialEditTitle", bodyKey: "tutorialEditBody", tipKey: "tutorialEditTip", selector: '[data-segment-id="tutorial-source-1"]', mode: "edit", preset: "clear" },
  { id: "order", titleKey: "tutorialOrderTitle", bodyKey: "tutorialOrderBody", tipKey: "tutorialOrderTip", selector: '[data-tutorial="order-tools"]', mode: "order", preset: "order" },
  { id: "unmatched", titleKey: "tutorialUnmatchedTitle", bodyKey: "tutorialUnmatchedBody", tipKey: "tutorialUnmatchedTip", selector: '[data-tutorial="unmatched-nav"]', mode: "review", preset: "clear" },
  { id: "find", titleKey: "tutorialFindTitle", bodyKey: "tutorialFindBody", tipKey: "tutorialFindTip", selector: '[data-tutorial="view-find"]', mode: "review", preset: "find" },
  { id: "context", titleKey: "tutorialContextTitle", bodyKey: "tutorialContextBody", tipKey: "tutorialContextTip", selector: '[data-segment-id="tutorial-source-1"] .segment-card__context-actions', mode: "review", preset: "context" },
  { id: "undo", titleKey: "tutorialUndoTitle", bodyKey: "tutorialUndoBody", tipKey: "tutorialUndoTip", selector: '[data-tutorial="history-buttons"]', mode: "review", preset: "clear" },
  { id: "finish", titleKey: "tutorialFinishTitle", bodyKey: "tutorialFinishBody", tipKey: "tutorialFinishTip", selector: '[data-tutorial="parallel-workspace"]', mode: "review", preset: "clear" },
];
