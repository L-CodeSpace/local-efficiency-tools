/*
 * 核心职责：定义远程挂载页表单、探测行和通用动作上下文。
 * 能力边界：只包含类型、默认值和纯格式化函数。
 */

import type { Dispatch, SetStateAction } from "react";
import type {
  EffectiveTransport,
  ProbeShareEntry,
  TransportPreference,
  WindowsSmbAuthMode,
} from "@/api_tauri";

export type ConnectionForm = {
  id?: string;
  name: string;
  host: string;
  username: string;
  password: string;
  passwordDirty: boolean;
  domain: string;
  ftpPort: string;
  smbPort: string;
  tlsMode: "none" | "explicit" | "implicit";
  noCheckCertificate: boolean;
  transportPreference: TransportPreference;
  windowsAuthMode: WindowsSmbAuthMode;
};

export type ProbeWorkspaceRow = ProbeShareEntry & {
  selected: boolean;
  driveLetter: string;
  mountPoint: string;
};

export type Translator = (text: string, vars?: Record<string, string | number>) => string;

export type ActionContext = {
  t: Translator;
  refresh: () => Promise<void>;
  reportError: (cause: unknown) => void;
  setBusyId: Dispatch<SetStateAction<string | null>>;
};

export const EMPTY_CONNECTION: ConnectionForm = {
  name: "nas",
  host: "192.168.88.186",
  username: "was",
  password: "123456Aa",
  passwordDirty: false,
  domain: "WORKGROUP",
  ftpPort: "21",
  smbPort: "445",
  tlsMode: "none",
  noCheckCertificate: false,
  transportPreference: "auto",
  windowsAuthMode: "auto",
};

export function nextNasName(existingNames: string[]): string {
  const usedNames = new Set(existingNames.map((name) => name.trim().toLowerCase()));
  if (!usedNames.has("nas")) return "nas";
  let suffix = 1;
  while (usedNames.has(`nas-${suffix}`)) suffix += 1;
  return `nas-${suffix}`;
}

export function joinPath(root: string, name: string): string {
  if (!root) return "";
  const separator = root.includes("\\") ? "\\" : "/";
  return `${root.replace(/[\\/]$/, "")}${separator}${name || "nas"}`;
}

export function transportLabel(transport?: EffectiveTransport): string {
  if (transport === "nativeSmb") return "原生 SMB";
  if (transport === "ftpCombine") return "FTP 聚合";
  return "未选择";
}
