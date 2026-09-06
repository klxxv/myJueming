export type SettingsCapabilityId =
  | "account"
  | "general"
  | "appearance"
  | "accessibility"
  | "input"
  | "pet"
  | "corpus"
  | "upload"
  | "community"
  | "agent"
  | "plugins"
  | "persistence"
  | "storage"
  | "privacy"
  | "notifications"
  | "about";

export type CapabilityAvailability = "available" | "unbound" | "disabled-by-policy" | "incompatible";

export interface SettingsCapability {
  availability: CapabilityAvailability;
  reason?: string;
}

export type SettingsCapabilities = Record<SettingsCapabilityId, SettingsCapability>;

export const createSettingsCapabilities = (): SettingsCapabilities => ({
  account: { availability: "unbound", reason: "账号服务尚未接入" },
  general: { availability: "available" },
  appearance: { availability: "available" },
  accessibility: { availability: "available" },
  input: { availability: "available" },
  pet: { availability: "unbound", reason: "窗口内桌宠运行时和正式动画资源尚未接入" },
  corpus: { availability: "unbound", reason: "本地语料库管理服务尚未接入" },
  upload: { availability: "unbound", reason: "上传、许可和审核服务尚未接入" },
  community: { availability: "unbound", reason: "社区与积分服务尚未接入" },
  agent: { availability: "unbound", reason: "Agent Runtime 尚未接入" },
  plugins: { availability: "unbound", reason: "Plugin Runtime 尚未接入" },
  persistence: { availability: "available" },
  storage: { availability: "available" },
  privacy: { availability: "available" },
  notifications: { availability: "unbound", reason: "系统通知能力尚未接入" },
  about: { availability: "available" },
});

export const isSettingsCapabilityAvailable = (
  capabilities: SettingsCapabilities,
  id: SettingsCapabilityId,
) => capabilities[id].availability === "available";
