/*
 * 核心职责：定义远程挂载页面表单类型。
 * 业务痛点：表单状态类型被页面和弹窗共同使用，不能埋在巨型 hook 文件里。
 * 能力边界：只承载类型，不包含状态和副作用。
 */

import type { MountAdvancedOptions, MountProtocol } from "@/api_tauri";

export type CacheMode = "off" | "minimal" | "writes" | "full";

export type MountFormState = {
  id?: string;
  name: string;
  protocol: MountProtocol;
  host: string;
  port: string;
  username: string;
  password: string;
  passwordDirty: boolean;
  url: string;
  vendor: string;
  keyFile: string;
  remotePath: string;
  mountPoint: string;
  driveLetter: string;
  tlsMode: string;
  noCheckCertificate: boolean;
  readOnly: boolean;
  cacheMode: CacheMode;
  advancedOptions: MountAdvancedOptions;
  enabled: boolean;
};

