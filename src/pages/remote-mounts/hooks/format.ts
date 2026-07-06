/*
 * 核心职责：格式化远程挂载展示和错误。
 * 业务痛点：列表展示标签和后端冲突错误解析需要统一出口。
 * 能力边界：只做无副作用的展示转换和错误解析。
 */

import type { MountProfile, MountProtocol } from "@/api_tauri";

export function profileTarget(profile: MountProfile) {
  return profile.driveLetter || profile.mountPoint || "自动分配";
}

export function protocolLabel(protocol: MountProtocol) {
  if (protocol === "webdav") return "WebDAV";
  return protocol.toUpperCase();
}


export function mountTargetConflict(error: unknown): { target: string; suggested: string } | null {
  if (!error || typeof error !== "object") return null;
  const payload = error as { code?: unknown; detail?: unknown };
  if (payload.code !== "mount_target_exists" || typeof payload.detail !== "string") return null;

  try {
    const detail = JSON.parse(payload.detail) as { target?: unknown; suggested?: unknown };
    if (typeof detail.target !== "string" || typeof detail.suggested !== "string") return null;
    if (!detail.target || !detail.suggested) return null;
    return { target: detail.target, suggested: detail.suggested };
  } catch {
    // 【合理吞噬】后端 detail 可能来自旧版本或损坏数据，解析失败时只表示没有可自动修复的挂载路径。
    return null;
  }
}
