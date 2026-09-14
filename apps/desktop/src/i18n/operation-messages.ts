export const operationMessageRows = {
  opAutosaveSaving: ["自动保存中…", "Autosaving…", "自動保存中…", "Enregistrement automatique…", "Automatisches Speichern…"],
  opEditSaving: ["保存编辑中…", "Saving edit…", "編集を保存中…", "Enregistrement de la modification…", "Bearbeitung wird gespeichert…"],
  opSavedLocally: ["本地存储 · 已保存", "Local storage · Saved", "ローカル保存 · 保存済み", "Stockage local · Enregistré", "Lokaler Speicher · Gespeichert"],
  opAutosaveFailed: ["自动保存失败：{error}", "Autosave failed: {error}", "自動保存に失敗しました：{error}", "Échec de l’enregistrement automatique : {error}", "Automatisches Speichern fehlgeschlagen: {error}"],
  opDiscardedEdit: ["已放弃未保存编辑", "Unsaved edit discarded", "未保存の編集を破棄しました", "Modification non enregistrée abandonnée", "Nicht gespeicherte Bearbeitung verworfen"],
  opSourceSide: ["原文", "source", "原文", "source", "Ausgangstext"],
  opTargetSide: ["译文", "translation", "訳文", "traduction", "Übersetzung"],
  opAbove: ["上方", "above", "上", "au-dessus", "oberhalb"],
  opBelow: ["下方", "below", "下", "en dessous", "unterhalb"],
  opGapInserted: ["已在{side}句段{position}插入空位，并自动重建后续 1:1 对齐", "Inserted a gap {position} the {side} segment and automatically rebuilt subsequent 1:1 alignments", "{side}セグメントの{position}に空きを挿入し、後続の 1:1 アラインメントを自動的に再構築しました", "Un espace a été inséré {position} du segment {side} et les alignements 1:1 suivants ont été reconstruits automatiquement", "Eine Lücke wurde {position} des {side}-Segments eingefügt; nachfolgende 1:1-Alignments wurden automatisch neu aufgebaut"],
  opGapInsertFailed: ["插入空位失败：{error}", "Failed to insert gap: {error}", "空きの挿入に失敗しました：{error}", "Échec de l’insertion de l’espace : {error}", "Lücke konnte nicht eingefügt werden: {error}"],
  opMergeSegmentsSuccess: ["已合并 {count} 个 Segment 内容；首项 ID 与关联锚点已保留", "Merged the content of {count} Segments; the first ID and linked anchors were preserved", "{count} 個の Segment の内容を結合しました。先頭の ID と関連アンカーは保持されています", "Le contenu de {count} Segments a été fusionné ; le premier identifiant et les ancres associées ont été conservés", "Inhalte von {count} Segmenten wurden zusammengeführt; die erste ID und verknüpfte Anker blieben erhalten"],
  opMergeSegmentsFailed: ["Merge 内容失败：{error}", "Failed to merge content: {error}", "内容の結合に失敗しました：{error}", "Échec de la fusion du contenu : {error}", "Inhalte konnten nicht zusammengeführt werden: {error}"],
  opSplitSegmentsSuccess: ["已无损拆分为 {count} 个 Segment；原 Alignment 关系保持不变", "Split losslessly into {count} Segments; the original Alignment relation was preserved", "内容を損なわず {count} 個の Segment に分割しました。元の Alignment 関係は維持されています", "Division sans perte en {count} Segments ; la relation d’Alignment d’origine a été conservée", "Verlustfrei in {count} Segmente geteilt; die ursprüngliche Alignment-Beziehung blieb erhalten"],
  opSplitSegmentsFailed: ["Split 内容失败：{error}", "Failed to split content: {error}", "内容の分割に失敗しました：{error}", "Échec de la division du contenu : {error}", "Inhalt konnte nicht geteilt werden: {error}"],
  opGroupBothSuccess: ["已将 {alignments} 个 Alignment 与 {segments} 条未对齐句段 Group 为一个 Alignment", "Grouped {alignments} Alignments and {segments} unaligned segments into one Alignment", "{alignments} 個の Alignment と未整列の {segments} 個のセグメントを 1 つの Alignment にグループ化しました", "{alignments} Alignments et {segments} segments non alignés ont été regroupés en un seul Alignment", "{alignments} Alignments und {segments} nicht ausgerichtete Segmente wurden zu einem Alignment gruppiert"],
  opGroupAlignmentsSuccess: ["已将 {count} 个 Alignment Group 为一个 Alignment", "Grouped {count} Alignments into one Alignment", "{count} 個の Alignment を 1 つの Alignment にグループ化しました", "{count} Alignments ont été regroupés en un seul Alignment", "{count} Alignments wurden zu einem Alignment gruppiert"],
  opGroupSegmentsSuccess: ["已将 {count} 条未对齐句段 Group 为一个 Alignment", "Grouped {count} unaligned segments into one Alignment", "未整列の {count} 個のセグメントを 1 つの Alignment にグループ化しました", "{count} segments non alignés ont été regroupés en un seul Alignment", "{count} nicht ausgerichtete Segmente wurden zu einem Alignment gruppiert"],
  opGroupFailed: ["Group 失败：{error}", "Grouping failed: {error}", "グループ化に失敗しました：{error}", "Échec du regroupement : {error}", "Gruppierung fehlgeschlagen: {error}"],
  opUngroupSuccess: ["已 Ungroup Alignment", "Alignment ungrouped", "Alignment のグループ化を解除しました", "Alignment dissocié", "Alignment-Gruppierung aufgehoben"],
  opUngroupFailed: ["Ungroup 失败：{error}", "Ungrouping failed: {error}", "グループ解除に失敗しました：{error}", "Échec de la dissociation : {error}", "Aufheben der Gruppierung fehlgeschlagen: {error}"],
} as const;

export type OperationMessageKey = keyof typeof operationMessageRows;
type OperationLocale = "zh" | "en" | "ja" | "fr" | "de";

function buildOperationMessages(index: 0 | 1 | 2 | 3 | 4): Record<OperationMessageKey, string> {
  return Object.fromEntries(
    Object.entries(operationMessageRows).map(([key, values]) => [key, values[index]]),
  ) as Record<OperationMessageKey, string>;
}

export const operationMessages: Record<OperationLocale, Record<OperationMessageKey, string>> = {
  zh: buildOperationMessages(0),
  en: buildOperationMessages(1),
  ja: buildOperationMessages(2),
  fr: buildOperationMessages(3),
  de: buildOperationMessages(4),
};
