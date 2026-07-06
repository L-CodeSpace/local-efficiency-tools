/*
 * 核心职责：管理远程挂载 rclone 日志状态。
 * 业务痛点：日志读取是按需 IPC 操作，不能和 profile 表单状态混在一起。
 * 能力边界：只读取当前 profile 的尾部日志，不修改挂载配置。
 */

import { useCallback, useState } from "react";
import { toast } from "sonner";
import { mountsGetProfileLog, type MountProfile, type MountProfileLog } from "@/api_tauri";
import { logError } from "@/shared/state/logStore";
import { formatError } from "@/shared/utils/error";

export function useMountLogs() {
  const [open, setOpen] = useState(false);
  const [profile, setProfile] = useState<MountProfile | null>(null);
  const [log, setLog] = useState<MountProfileLog | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadLog = useCallback(async (profileId: string) => {
    setLoading(true);
    setError(null);
    try {
      setLog(await mountsGetProfileLog({ id: profileId, maxLines: 400 }));
    } catch (err) {
      const message = formatError(err);
      setError(message);
      toast.error(message);
      logError(message);
    } finally {
      setLoading(false);
    }
  }, []);

  const openLog = useCallback(
    (nextProfile: MountProfile) => {
      setProfile(nextProfile);
      setLog(null);
      setOpen(true);
      void loadLog(nextProfile.id);
    },
    [loadLog],
  );

  const refresh = useCallback(() => {
    if (profile) {
      void loadLog(profile.id);
    }
  }, [loadLog, profile]);

  return {
    open,
    setOpen,
    profile,
    log,
    loading,
    error,
    openLog,
    refresh,
  };
}

export type MountLogsState = ReturnType<typeof useMountLogs>;

