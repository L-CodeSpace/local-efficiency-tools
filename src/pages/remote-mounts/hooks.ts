/*
 * 核心职责：管理远程挂载页面状态。
 * 业务痛点：页面状态、IPC 调用和表单转换混在一个大文件会扩大回归风险。
 * 能力边界：只负责 React 状态、副作用和调用应用命令。
 */

import { useEffect, useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import { mountsCheckDependencies, mountsDeleteProfile, mountsGetRuntimeStatus, mountsGetUiContext, mountsListProfiles, mountsSetProfileEnabled, mountsTestProfile, mountsUnmountAll, type MountDependencyStatus, type MountProfile, type MountRuntimeStatus, type MountUiContext } from "@/api_tauri";
import { logError, logInfo, logSuccess } from "@/shared/state/logStore";
import { formatError } from "@/shared/utils/error";
import { useI18n } from "@/shared/i18n";
import {
  createDefaultForm,
  defaultMountPoint,
  profileToForm,
  recommendedAdvancedOptions,
  validateForm,
} from "./hooks/form";
import { useMountLogs } from "./hooks/logs";
import { saveProfileAttempt } from "./hooks/save-profile";
import type { MountFormState } from "./hooks/types";

export type { CacheMode, MountFormState } from "./hooks/types";
export { profileTarget, protocolLabel } from "./hooks/format";

export function useRemoteMountsPage() {
  const { t } = useI18n();
  const logs = useMountLogs();
  const [profiles, setProfiles] = useState<MountProfile[]>([]);
  const [runtime, setRuntime] = useState<MountRuntimeStatus | null>(null);
  const [deps, setDeps] = useState<MountDependencyStatus | null>(null);
  const [uiContext, setUiContext] = useState<MountUiContext | null>(null);
  const [loading, setLoading] = useState(true);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [form, setForm] = useState<MountFormState>(() => createDefaultForm(null));
  const [mountPointEdited, setMountPointEdited] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const mountedCount = useMemo(
    () => profiles.filter((profile) => profile.mounted).length,
    [profiles],
  );
  const enabledCount = useMemo(
    () => profiles.filter((profile) => profile.enabled).length,
    [profiles],
  );

  const refresh = async () => {
    setLoading(true);
    setError(null);
    try {
      const [nextRuntime, nextDeps, nextUiContext, nextProfiles] = await Promise.all([
        mountsGetRuntimeStatus(),
        mountsCheckDependencies(),
        mountsGetUiContext(),
        mountsListProfiles(),
      ]);
      setRuntime(nextRuntime);
      setDeps(nextDeps);
      setUiContext(nextUiContext);
      setProfiles(nextProfiles);
    } catch (err) {
      const message = formatError(err);
      setError(message);
      logError(message);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    refresh();
  }, []);

  useEffect(() => {
    if (!dialogOpen || !uiContext || mountPointEdited) return;
    setForm((current) =>
      current.id
        ? current
        : {
            ...current,
            mountPoint: current.driveLetter.trim()
              ? current.mountPoint
              : defaultMountPoint(uiContext, current.name),
          },
    );
  }, [dialogOpen, mountPointEdited, uiContext]);

  const openCreateDialog = () => {
    setMountPointEdited(false);
    setForm(createDefaultForm(uiContext));
    setDialogOpen(true);
  };

  const openEditDialog = (profile: MountProfile) => {
    setMountPointEdited(true);
    setForm(profileToForm(profile));
    setDialogOpen(true);
  };

  const updateForm = (patch: Partial<MountFormState>) => {
    const changesMountPoint = Object.prototype.hasOwnProperty.call(patch, "mountPoint");
    const changesPassword = Object.prototype.hasOwnProperty.call(patch, "password");
    const changesDriveLetter = Object.prototype.hasOwnProperty.call(patch, "driveLetter");
    if (changesMountPoint) {
      setMountPointEdited(true);
    }
    setForm((current) => {
      const next = { ...current, ...patch };
      if (changesPassword) {
        next.passwordDirty = true;
      }
      if (
        changesDriveLetter &&
        uiContext?.supportsDriveLetter &&
        current.advancedOptions.networkMode === Boolean(current.driveLetter.trim())
      ) {
        next.advancedOptions = {
          ...next.advancedOptions,
          networkMode: Boolean(next.driveLetter.trim()),
        };
      }
      if (
        !current.id &&
        !mountPointEdited &&
        !changesMountPoint &&
        Object.prototype.hasOwnProperty.call(patch, "name") &&
        !next.driveLetter.trim()
      ) {
        next.mountPoint = defaultMountPoint(uiContext, next.name);
      }
      return next;
    });
  };

  const resetAdvancedOptions = () => {
    setForm((current) => ({
      ...current,
      cacheMode: "full",
      advancedOptions: recommendedAdvancedOptions(uiContext, current.driveLetter),
    }));
  };

  const pickMountPoint = async () => {
    const selected = await open({ directory: true });
    if (selected && typeof selected === "string") {
      updateForm({ mountPoint: selected });
    }
  };

  const pickKeyFile = async () => {
    const selected = await open({ multiple: false });
    if (selected && typeof selected === "string") {
      updateForm({ keyFile: selected });
    }
  };

  const saveProfile = async () => {
    const validation = validateForm(form);
    if (validation) {
      toast.error(validation);
      return;
    }

    setSaving(true);
    setError(null);
    try {
      await saveProfileAttempt({
        currentForm: form,
        currentMountPointEdited: mountPointEdited,
        uiContext,
        setDialogOpen,
        setError,
        setForm,
        setMountPointEdited,
        refresh,
        t,
      });
    } finally {
      setSaving(false);
    }
  };

  const toggleProfile = async (profile: MountProfile, enabled: boolean) => {
    setBusyId(profile.id);
    setError(null);
    try {
      await mountsSetProfileEnabled({ id: profile.id, enabled });
      toast.success(enabled ? t("已启用 {name}", { name: profile.name }) : t("已停用 {name}", { name: profile.name }));
      logInfo(`${enabled ? "启用" : "停用"}远程挂载：${profile.name}`);
      await refresh();
    } catch (err) {
      const message = formatError(err);
      setError(message);
      toast.error(message);
      logError(message);
    } finally {
      setBusyId(null);
    }
  };

  const testProfile = async (profile: MountProfile) => {
    setBusyId(profile.id);
    setError(null);
    try {
      const result = await mountsTestProfile({ id: profile.id });
      if (result.success) {
        toast.success(result.message);
        logSuccess(`连接测试成功：${profile.name}`);
      } else {
        toast.error(result.message || t("连接测试失败"));
        logError(`连接测试失败：${profile.name}`);
      }
    } catch (err) {
      const message = formatError(err);
      setError(message);
      toast.error(message);
      logError(message);
    } finally {
      setBusyId(null);
    }
  };

  const deleteProfile = async (profile: MountProfile) => {
    setBusyId(profile.id);
    setError(null);
    try {
      await mountsDeleteProfile({ id: profile.id });
      toast.success(t("已删除 {name}", { name: profile.name }));
      logSuccess(`已删除远程挂载：${profile.name}`);
      await refresh();
    } catch (err) {
      const message = formatError(err);
      setError(message);
      toast.error(message);
      logError(message);
    } finally {
      setBusyId(null);
    }
  };

  const unmountAll = async () => {
    setBusyId("all");
    setError(null);
    try {
      await mountsUnmountAll();
      toast.success(t("已卸载全部挂载"));
      logSuccess("已卸载全部 rclone 挂载");
      await refresh();
    } catch (err) {
      const message = formatError(err);
      setError(message);
      toast.error(message);
      logError(message);
    } finally {
      setBusyId(null);
    }
  };

  return {
    profiles,
    runtime,
    deps,
    uiContext,
    loading,
    busyId,
    saving,
    dialogOpen,
    setDialogOpen,
    form,
    updateForm,
    resetAdvancedOptions,
    error,
    mountedCount,
    enabledCount,
    refresh,
    openCreateDialog,
    openEditDialog,
    pickMountPoint,
    pickKeyFile,
    saveProfile,
    toggleProfile,
    testProfile,
    deleteProfile,
    unmountAll,
    logs,
  };
}
