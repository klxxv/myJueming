const rows = {
  parallelStatusMissingAnchor: ["未找到跳转锚定的句段", "The target segment could not be found", "移動先のセグメントが見つかりません", "Le segment cible est introuvable", "Das Zielsegment wurde nicht gefunden"],
  parallelStatusNoUnmatched: ["当前视图没有未匹配句段", "No unmatched segments in the current view", "現在のビューに未対応のセグメントはありません", "Aucun segment non aligné dans la vue actuelle", "Keine nicht zugeordneten Segmente in der aktuellen Ansicht"],
  parallelStatusFindFailed: ["查找失败：{error}", "Find failed: {error}", "検索に失敗しました：{error}", "Échec de la recherche : {error}", "Suche fehlgeschlagen: {error}"],
  parallelStatusDemo: ["当前为演示预览，请先新建或打开工程", "This is a demo preview. Create or open a project first", "デモプレビューです。まずプロジェクトを作成するか開いてください", "Ceci est un aperçu de démonstration. Créez ou ouvrez d’abord un projet", "Dies ist eine Demovorschau. Erstellen oder öffnen Sie zuerst ein Projekt"],
  parallelStatusSelectOtherSide: ["已选中待匹配句段；请在另一侧选择句段后使用 Link", "Unmatched segment selected; select segments on the other side, then use Link", "未対応のセグメントを選択しました。反対側のセグメントを選択して Link を実行してください", "Segment non aligné sélectionné ; sélectionnez des segments de l’autre côté, puis utilisez Link", "Nicht zugeordnetes Segment ausgewählt; wählen Sie Segmente auf der anderen Seite und verwenden Sie Link"],
  parallelStatusRevisionChanged: ["工程版本已变化，请保留草稿并重新打开内容预览", "The project revision changed. Keep your draft and reopen the content preview", "プロジェクトの版が変更されました。下書きを保持して内容プレビューを開き直してください", "La révision du projet a changé. Conservez votre brouillon et rouvrez l’aperçu", "Die Projektrevision hat sich geändert. Behalten Sie den Entwurf und öffnen Sie die Inhaltsvorschau erneut"],
} as const;
type Key = keyof typeof rows;
const catalogue = (index: 0 | 1 | 2 | 3 | 4) => Object.fromEntries(Object.entries(rows).map(([key, values]) => [key, values[index]])) as Record<Key, string>;
export const parallelStatusMessages = { zh: catalogue(0), en: catalogue(1), ja: catalogue(2), fr: catalogue(3), de: catalogue(4) };
