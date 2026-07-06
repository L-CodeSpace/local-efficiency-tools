import { useEffect, useState } from "react";
import { toast } from "sonner";
import {
  appSettingsGetBackground,
  appSettingsSetBackground,
  mediaDownloadRuntime,
  mediaRuntimeStatus,
  mountsDownloadRuntime,
  mountsGetRuntimeStatus,
  systemOverview,
  systemHardwareInfo,
  type HardwareInfo,
  type MediaRuntimeStatus,
  type MountRuntimeStatus,
  type SystemOverview,
} from "@/api_tauri";
import { logError, logSuccess } from "@/shared/state/logStore";
import { formatError } from "@/shared/utils/error";

export function useSettingsPage() {
  const [overview, setOverview] = useState<SystemOverview | null>(null);
  const [hardware, setHardware] = useState<HardwareInfo | null>(null);
  const [mediaRuntime, setMediaRuntime] = useState<MediaRuntimeStatus | null>(null);
  const [mountRuntime, setMountRuntime] = useState<MountRuntimeStatus | null>(null);
  const [backgroundEnabled, setBackgroundEnabled] = useState(false);
  const [backgroundSaving, setBackgroundSaving] = useState(false);
  const [runtimeDownloading, setRuntimeDownloading] = useState<"ffmpeg" | "rclone" | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refresh = async () => {
    setError(null);
    try {
      const [nextOverview, nextHardware, background, nextMediaRuntime, nextMountRuntime] = await Promise.all([
        systemOverview(),
        systemHardwareInfo(),
        appSettingsGetBackground(),
        mediaRuntimeStatus(),
        mountsGetRuntimeStatus(),
      ]);
      setOverview(nextOverview);
      setHardware(nextHardware);
      setBackgroundEnabled(background.enabled);
      setMediaRuntime(nextMediaRuntime);
      setMountRuntime(nextMountRuntime);
    } catch (err) {
      setError(formatError(err));
    }
  };

  useEffect(() => {
    refresh();
  }, []);

  const setBackground = async (enabled: boolean) => {
    const previous = backgroundEnabled;
    setBackgroundEnabled(enabled);
    setBackgroundSaving(true);
    try {
      const next = await appSettingsSetBackground({ enabled });
      setBackgroundEnabled(next.enabled);
      toast.success(next.enabled ? "已开启后台运行" : "已关闭后台运行");
    } catch (err) {
      setBackgroundEnabled(previous);
      const message = formatError(err);
      setError(message);
      toast.error(message);
    } finally {
      setBackgroundSaving(false);
    }
  };

  const downloadRuntime = async (kind: "ffmpeg" | "rclone") => {
    setRuntimeDownloading(kind);
    setError(null);
    try {
      if (kind === "ffmpeg") {
        const next = await mediaDownloadRuntime();
        setMediaRuntime(next);
        logSuccess("FFmpeg 运行时已下载");
        toast.success("FFmpeg 运行时已下载");
      } else {
        const next = await mountsDownloadRuntime();
        setMountRuntime(next);
        logSuccess("rclone 运行时已下载");
        toast.success("rclone 运行时已下载");
      }
    } catch (err) {
      const message = formatError(err);
      setError(message);
      logError(message);
      toast.error(message);
    } finally {
      setRuntimeDownloading(null);
    }
  };

  return {
    overview,
    hardware,
    mediaRuntime,
    mountRuntime,
    runtimeDownloading,
    backgroundEnabled,
    backgroundSaving,
    refresh,
    setBackground,
    downloadRuntime,
    error,
  };
}
