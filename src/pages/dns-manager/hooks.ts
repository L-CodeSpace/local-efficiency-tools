import { useCallback, useEffect, useState } from "react";
import {
  hostsExecuteChange,
  hostsGetPath,
  hostsGetStatus,
  hostsInstallHelper,
  hostsPreviewChange,
  hostsRead,
  hostsRepairHelper,
  hostsUninstallHelper,
  type HostEntry,
  type HostsChangeRequest,
  type HostsHelperStatus,
} from "@/api_tauri";
import { logError, logInfo, logSuccess } from "@/shared/state/logStore";
import { formatError } from "@/shared/utils/error";

export function useDnsManagerPage() {
  const [entries, setEntries] = useState<HostEntry[]>([]);
  const [hostsPath, setHostsPath] = useState("");
  const [helperStatus, setHelperStatus] = useState<HostsHelperStatus | null>(null);
  const [loading, setLoading] = useState(false);
  const [helperBusy, setHelperBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [newIp, setNewIp] = useState("127.0.0.1");
  const [newDomain, setNewDomain] = useState("");

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [path, status, rows] = await Promise.all([hostsGetPath(), hostsGetStatus(), hostsRead()]);
      setHostsPath(path);
      setHelperStatus(status);
      setEntries(rows);
    } catch (err) {
      setError(formatError(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const executeChange = async (request: HostsChangeRequest, success: string) => {
    setLoading(true);
    setError(null);
    try {
      const plan = await hostsPreviewChange({ request });
      const rows = await hostsExecuteChange({
        planId: plan.id,
        confirmationToken: plan.confirmationToken,
      });
      setEntries(rows);
      logSuccess(success);
      return true;
    } catch (err) {
      const message = formatError(err);
      setError(message);
      logError(message);
      return false;
    } finally {
      setLoading(false);
    }
  };

  const addEntry = async () => {
    const ip = newIp.trim();
    const host = newDomain.trim();
    if (!ip || !host) return false;
    const ok = await executeChange({ action: "add", ip, host }, `已添加 ${host}`);
    if (ok) {
      setNewDomain("");
      logInfo("hosts 已刷新");
    }
    return ok;
  };

  const toggleEntry = (entry: HostEntry, enabled: boolean) =>
    executeChange(
      { action: "toggle", host: entry.hosts[0], enabled },
      `已${enabled ? "启用" : "禁用"} ${entry.hosts[0]}`,
    );

  const removeEntry = (entry: HostEntry) =>
    executeChange({ action: "remove", host: entry.hosts[0] }, `已删除 ${entry.hosts[0]}`);

  const installHelper = async () => {
    setHelperBusy(true);
    setError(null);
    try {
      const status = await hostsInstallHelper();
      setHelperStatus(status);
      logSuccess("hosts helper 已安装");
      await refresh();
      return true;
    } catch (err) {
      const message = formatError(err);
      setError(message);
      logError(message);
      return false;
    } finally {
      setHelperBusy(false);
    }
  };

  const repairHelper = async () => {
    setHelperBusy(true);
    setError(null);
    try {
      const status = await hostsRepairHelper();
      setHelperStatus(status);
      logSuccess("hosts helper 已修复");
      await refresh();
      return true;
    } catch (err) {
      const message = formatError(err);
      setError(message);
      logError(message);
      return false;
    } finally {
      setHelperBusy(false);
    }
  };

  const uninstallHelper = async () => {
    setHelperBusy(true);
    setError(null);
    try {
      const status = await hostsUninstallHelper();
      setHelperStatus(status);
      logSuccess("hosts helper 已卸载");
      await refresh();
      return true;
    } catch (err) {
      const message = formatError(err);
      setError(message);
      logError(message);
      return false;
    } finally {
      setHelperBusy(false);
    }
  };

  return {
    entries,
    hostsPath,
    helperStatus,
    loading,
    helperBusy,
    error,
    newIp,
    setNewIp,
    newDomain,
    setNewDomain,
    refresh,
    addEntry,
    toggleEntry,
    removeEntry,
    installHelper,
    repairHelper,
    uninstallHelper,
  };
}
