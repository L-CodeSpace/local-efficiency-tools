/*
 * 核心职责：管理视频详情探测状态。
 * 业务痛点：ffprobe 缓存、加载状态和错误状态不应挤在视频页面主 hook 中。
 * 能力边界：只调用 mediaProbeVideo，不参与转码计划和任务启动。
 */

import { useState } from "react";
import { mediaProbeVideo, type MediaProbeInfo } from "@/api_tauri";
import { logError } from "@/shared/state/logStore";
import { formatError } from "@/shared/utils/error";

export function useVideoProbeDetails() {
  const [selectedDetailPath, setSelectedDetailPath] = useState<string | null>(null);
  const [probeCache, setProbeCache] = useState<Record<string, MediaProbeInfo>>({});
  const [probeLoadingPath, setProbeLoadingPath] = useState<string | null>(null);
  const [probeError, setProbeError] = useState<string | null>(null);

  const selectedProbe = selectedDetailPath ? (probeCache[selectedDetailPath] ?? null) : null;
  const probeLoading = Boolean(selectedDetailPath && probeLoadingPath === selectedDetailPath);

  function resetProbeState() {
    setSelectedDetailPath(null);
    setProbeCache({});
    setProbeLoadingPath(null);
    setProbeError(null);
  }

  async function loadVideoDetails(path: string, force = false) {
    setSelectedDetailPath(path);
    setProbeError(null);
    if (!force && probeCache[path]) return;
    setProbeLoadingPath(path);
    try {
      const detail = await mediaProbeVideo({ path });
      setProbeCache((current) => ({ ...current, [path]: detail }));
    } catch (err) {
      const message = formatError(err);
      setProbeError(message);
      logError(message);
    } finally {
      setProbeLoadingPath((current) => (current === path ? null : current));
    }
  }

  return {
    selectedDetailPath,
    selectedProbe,
    probeLoading,
    probeError,
    setSelectedDetailPath,
    resetProbeState,
    loadVideoDetails,
  };
}
