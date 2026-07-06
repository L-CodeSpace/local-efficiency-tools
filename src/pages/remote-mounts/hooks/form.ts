/*
 * 核心职责：转换和校验远程挂载表单。
 * 业务痛点：profile 与表单之间的映射需要独立测试和复用，避免塞进页面状态 hook。
 * 能力边界：只处理纯数据转换，不触发 IPC 或 UI 副作用。
 */

import type { MountAdvancedOptions, MountProfile, MountProfileInput, MountUiContext } from "@/api_tauri";
import { joinPath } from "@/shared/utils/path";
import type { CacheMode, MountFormState } from "./types";

const DEFAULT_MOUNT_NAME = "nas";
const DEFAULT_FTP_HOST = "192.168.88.186";
const DEFAULT_FTP_PORT = "21";

export function recommendedAdvancedOptions(
  uiContext: MountUiContext | null,
  driveLetter: string,
): MountAdvancedOptions {
  return {
    vfsCacheMaxSize: "5G",
    vfsCacheMaxAge: "24h",
    vfsReadChunkSize: "64M",
    bufferSize: "32M",
    pollInterval: "0",
    links: true,
    networkMode: Boolean(uiContext?.supportsDriveLetter && driveLetter.trim()),
    connectTimeout: "5s",
    ioTimeout: "30s",
    retries: 1,
    lowLevelRetries: 3,
    retriesSleep: "2s",
  };
}

export function createDefaultForm(uiContext: MountUiContext | null): MountFormState {
  const driveLetter = uiContext?.defaultDriveLetter ?? "";
  return {
    name: DEFAULT_MOUNT_NAME,
    protocol: "ftp",
    host: DEFAULT_FTP_HOST,
    port: DEFAULT_FTP_PORT,
    username: "",
    password: "",
    passwordDirty: false,
    url: "",
    vendor: "other",
    keyFile: "",
    remotePath: "",
    mountPoint: driveLetter ? "" : defaultMountPoint(uiContext, DEFAULT_MOUNT_NAME),
    driveLetter,
    tlsMode: "none",
    noCheckCertificate: false,
    readOnly: false,
    cacheMode: "full",
    advancedOptions: recommendedAdvancedOptions(uiContext, driveLetter),
    enabled: true,
  };
}

export function profileToForm(profile: MountProfile): MountFormState {
  return {
    id: profile.id,
    name: profile.name,
    protocol: profile.protocol,
    host: profile.host ?? "",
    port: profile.port ? String(profile.port) : "",
    username: profile.username ?? "",
    password: profile.password ?? "",
    passwordDirty: false,
    url: profile.url ?? "",
    vendor: profile.vendor ?? "other",
    keyFile: profile.keyFile ?? "",
    remotePath: profile.remotePath ?? "",
    mountPoint: profile.mountPoint ?? "",
    driveLetter: profile.driveLetter ?? "",
    tlsMode: profile.tlsMode ?? "none",
    noCheckCertificate: profile.noCheckCertificate,
    readOnly: profile.readOnly,
    cacheMode: (profile.cacheMode || "full") as CacheMode,
    advancedOptions: profile.advancedOptions ?? recommendedAdvancedOptions(null, profile.driveLetter ?? ""),
    enabled: profile.enabled,
  };
}

export function formToInput(
  form: MountFormState,
  uiContext: MountUiContext | null,
  mountPointEdited: boolean,
): MountProfileInput {
  const port = form.port.trim() ? Number(form.port.trim()) : undefined;
  const supportsDriveLetter = uiContext?.supportsDriveLetter ?? false;
  return {
    id: form.id,
    name: form.name.trim(),
    protocol: form.protocol,
    host: trimOrUndefined(form.host),
    port: Number.isFinite(port) ? port : undefined,
    username: trimOrUndefined(form.username),
    password: form.passwordDirty ? form.password.trim() : undefined,
    url: trimOrUndefined(form.url),
    vendor: form.protocol === "webdav" ? trimOrUndefined(form.vendor) : undefined,
    keyFile: form.protocol === "sftp" ? trimOrUndefined(form.keyFile) : undefined,
    remotePath: trimOrUndefined(form.remotePath),
    mountPoint: mountPointEdited ? trimOrUndefined(form.mountPoint) : undefined,
    driveLetter: supportsDriveLetter ? trimOrUndefined(form.driveLetter) : undefined,
    tlsMode: form.protocol === "ftp" ? trimOrUndefined(form.tlsMode) : undefined,
    noCheckCertificate: form.noCheckCertificate,
    readOnly: form.readOnly,
    cacheMode: form.cacheMode,
    advancedOptions: normalizeAdvancedOptionsForInput(form, supportsDriveLetter),
    enabled: form.enabled,
  };
}

export function validateForm(form: MountFormState) {
  if (!form.name.trim()) return "请填写配置名称";
  if ((form.protocol === "ftp" || form.protocol === "sftp") && !form.host.trim()) {
    return "请填写主机地址";
  }
  if (form.protocol === "webdav" && !form.url.trim()) {
    return "请填写 WebDAV URL";
  }
  const advancedError = validateAdvancedOptions(form.advancedOptions);
  if (advancedError) return advancedError;
  return null;
}

export function defaultMountPoint(uiContext: MountUiContext | null, name: string) {
  if (!uiContext) return "";
  return joinPath(uiContext.defaultMountRoot, defaultMountDirName(name));
}

function defaultMountDirName(name: string) {
  const sanitized = [...name]
    .map((char) => (/[<>:"/\\|?*]/.test(char) ? "_" : char))
    .join("")
    .slice(0, 48);
  return sanitized.trim() ? sanitized : "remote";
}


function trimOrUndefined(value: string) {
  const trimmed = value.trim();
  return trimmed ? trimmed : undefined;
}

function normalizeAdvancedOptionsForInput(
  form: MountFormState,
  supportsDriveLetter: boolean,
): MountAdvancedOptions {
  return {
    ...form.advancedOptions,
    networkMode: Boolean(supportsDriveLetter && form.driveLetter.trim() && form.advancedOptions.networkMode),
  };
}

function validateAdvancedOptions(options: MountAdvancedOptions) {
  const sizeFields: Array<[string, string]> = [
    ["VFS 缓存上限", options.vfsCacheMaxSize],
    ["读取块大小", options.vfsReadChunkSize],
    ["Buffer 大小", options.bufferSize],
  ];
  for (const [label, value] of sizeFields) {
    if (!isSizeSuffix(value)) return `${label}格式无效`;
  }

  const durationFields: Array<[string, string]> = [
    ["VFS 缓存保留时间", options.vfsCacheMaxAge],
    ["轮询间隔", options.pollInterval],
    ["连接超时", options.connectTimeout],
    ["IO 超时", options.ioTimeout],
    ["重试间隔", options.retriesSleep],
  ];
  for (const [label, value] of durationFields) {
    if (!isDuration(value)) return `${label}格式无效`;
  }

  if (!Number.isInteger(options.retries) || options.retries < 0 || options.retries > 100) {
    return "重试次数必须是 0-100 的整数";
  }
  if (!Number.isInteger(options.lowLevelRetries) || options.lowLevelRetries < 0 || options.lowLevelRetries > 100) {
    return "低层重试次数必须是 0-100 的整数";
  }
  return null;
}

function isSizeSuffix(value: string) {
  const trimmed = value.trim();
  return /^(\d+[a-zA-Z]*|off)$/i.test(trimmed);
}

function isDuration(value: string) {
  const trimmed = value.trim();
  return trimmed === "0" || /^(\d+(ms|s|m|h))+$/.test(trimmed);
}
