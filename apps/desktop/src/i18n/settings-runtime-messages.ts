const zh = {
  runtimeMotionSystemReduced: "跟随系统 · 已减少",
  runtimeMotionSystem: "跟随系统",
  runtimeMotionReduced: "减少",
  runtimeMotionOff: "关闭非必要动画",
  runtimeGeneralUpdated: "通用偏好已更新",
  runtimeAccessibilityUpdated: "辅助功能偏好已更新",
  runtimeMotionUpdated: "动态效果：{p0}",
  runtimeSettingsSaveFailed: "设置保存失败：{p0}",
  runtimeSettingsLoadFailed: "读取设置失败，已使用默认值：{p0}",
  runtimeSettingsReset: "已恢复本机默认设置",
};
type Messages = Record<keyof typeof zh, string>;
export const settingsRuntimeMessages = {
  zh,
  en: {
    runtimeMotionSystemReduced: "System preference · Reduced", runtimeMotionSystem: "System preference",
    runtimeMotionReduced: "Reduced", runtimeMotionOff: "Nonessential animations off",
    runtimeGeneralUpdated: "General preferences updated", runtimeAccessibilityUpdated: "Accessibility preferences updated",
    runtimeMotionUpdated: "Motion: {p0}", runtimeSettingsSaveFailed: "Could not save settings: {p0}",
    runtimeSettingsLoadFailed: "Could not load settings; defaults applied: {p0}", runtimeSettingsReset: "Device settings restored to defaults",
  } satisfies Messages,
  ja: {
    runtimeMotionSystemReduced: "システム設定に従う · 軽減", runtimeMotionSystem: "システム設定に従う",
    runtimeMotionReduced: "軽減", runtimeMotionOff: "不要なアニメーションを無効化",
    runtimeGeneralUpdated: "一般設定を更新しました", runtimeAccessibilityUpdated: "アクセシビリティ設定を更新しました",
    runtimeMotionUpdated: "モーション：{p0}", runtimeSettingsSaveFailed: "設定を保存できませんでした：{p0}",
    runtimeSettingsLoadFailed: "設定を読み込めないため既定値を使用します：{p0}", runtimeSettingsReset: "このデバイスの設定を初期値に戻しました",
  } satisfies Messages,
  fr: {
    runtimeMotionSystemReduced: "Préférence système · Réduites", runtimeMotionSystem: "Préférence système",
    runtimeMotionReduced: "Réduites", runtimeMotionOff: "Animations non essentielles désactivées",
    runtimeGeneralUpdated: "Préférences générales mises à jour", runtimeAccessibilityUpdated: "Préférences d’accessibilité mises à jour",
    runtimeMotionUpdated: "Animations : {p0}", runtimeSettingsSaveFailed: "Échec de l’enregistrement des réglages : {p0}",
    runtimeSettingsLoadFailed: "Échec du chargement des réglages ; valeurs par défaut appliquées : {p0}", runtimeSettingsReset: "Réglages de l’appareil réinitialisés",
  } satisfies Messages,
  de: {
    runtimeMotionSystemReduced: "Systemeinstellung · Reduziert", runtimeMotionSystem: "Systemeinstellung",
    runtimeMotionReduced: "Reduziert", runtimeMotionOff: "Nicht notwendige Animationen aus",
    runtimeGeneralUpdated: "Allgemeine Einstellungen aktualisiert", runtimeAccessibilityUpdated: "Einstellungen zur Barrierefreiheit aktualisiert",
    runtimeMotionUpdated: "Bewegung: {p0}", runtimeSettingsSaveFailed: "Einstellungen konnten nicht gespeichert werden: {p0}",
    runtimeSettingsLoadFailed: "Einstellungen konnten nicht geladen werden; Standardwerte angewendet: {p0}", runtimeSettingsReset: "Geräteeinstellungen zurückgesetzt",
  } satisfies Messages,
};
