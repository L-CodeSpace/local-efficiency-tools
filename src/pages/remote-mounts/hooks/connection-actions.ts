/*
 * 核心职责：处理远程连接表单、保存和删除动作。
 * 能力边界：不执行协议探测或工作区生命周期操作。
 */

import type { Dispatch, SetStateAction } from "react";
import { toast } from "sonner";
import {
  mountsCleanupSmbHost,
  mountsDeleteConnection,
  mountsSaveConnection,
  type MountWorkspace,
  type RemoteConnection,
} from "@/api_tauri";
import { EMPTY_CONNECTION, nextNasName, type ActionContext, type ConnectionForm } from "./model";

type Context = ActionContext & {
  connections: RemoteConnection[];
  workspaces: MountWorkspace[];
  connectionForm: ConnectionForm;
  probeConnectionId: string;
  setConnectionDialogOpen: Dispatch<SetStateAction<boolean>>;
  setConnectionForm: Dispatch<SetStateAction<ConnectionForm>>;
  setProbeConnectionId: Dispatch<SetStateAction<string>>;
  resetProbe: () => void;
};

export function createConnectionActions(context: Context) {
  const {
    connectionForm,
    connections,
    probeConnectionId,
    refresh,
    reportError,
    resetProbe,
    setBusyId,
    setConnectionDialogOpen,
    setConnectionForm,
    setProbeConnectionId,
    t,
    workspaces,
  } = context;

  const openCreateConnection = () => {
    setConnectionForm({
      ...EMPTY_CONNECTION,
      name: nextNasName([
        ...connections.map((connection) => connection.name),
        ...workspaces.map((workspace) => workspace.name),
      ]),
    });
    setConnectionDialogOpen(true);
  };

  const openEditConnection = (connection: RemoteConnection) => {
    setConnectionForm({
      id: connection.id,
      name: connection.name,
      host: connection.host,
      username: connection.username,
      password: connection.password ?? "",
      passwordDirty: false,
      domain: connection.domain ?? "",
      ftpPort: String(connection.ftpPort),
      smbPort: String(connection.smbPort),
      tlsMode: (connection.tlsMode as ConnectionForm["tlsMode"] | undefined) ?? "none",
      noCheckCertificate: connection.noCheckCertificate,
      transportPreference: connection.transportPreference,
      windowsAuthMode: connection.windowsAuthMode,
    });
    setConnectionDialogOpen(true);
  };

  const updateConnectionForm = (patch: Partial<ConnectionForm>) => {
    setConnectionForm((current) => ({
      ...current,
      ...patch,
      passwordDirty: Object.prototype.hasOwnProperty.call(patch, "password")
        ? true
        : current.passwordDirty,
    }));
  };

  const saveConnection = async () => {
    if (!connectionForm.name.trim() || !connectionForm.host.trim() || !connectionForm.username.trim()) {
      toast.error(t("请填写连接名称、主机和用户名"));
      return;
    }
    setBusyId("save-connection");
    try {
      const saved = await mountsSaveConnection({
        input: {
          id: connectionForm.id,
          name: connectionForm.name,
          host: connectionForm.host,
          username: connectionForm.username,
          password: connectionForm.id && !connectionForm.passwordDirty
            ? undefined
            : connectionForm.password,
          domain: connectionForm.domain || undefined,
          ftpPort: Number(connectionForm.ftpPort || 21),
          smbPort: Number(connectionForm.smbPort || 445),
          tlsMode: connectionForm.tlsMode === "none" ? undefined : connectionForm.tlsMode,
          noCheckCertificate: connectionForm.noCheckCertificate,
          transportPreference: connectionForm.transportPreference,
          windowsAuthMode: connectionForm.windowsAuthMode,
        },
      });
      setConnectionDialogOpen(false);
      setProbeConnectionId(saved.id);
      toast.success(t("连接已保存"));
      await refresh();
    } catch (cause) {
      reportError(cause);
    } finally {
      setBusyId(null);
    }
  };

  const deleteConnection = async (connection: RemoteConnection) => {
    if (!window.confirm(t("删除连接会同时删除其工作区，是否继续？"))) return;
    setBusyId(`connection:${connection.id}`);
    try {
      await mountsDeleteConnection({ id: connection.id });
      if (probeConnectionId === connection.id) {
        resetProbe();
        setProbeConnectionId("");
      }
      toast.success(t("连接已删除"));
      await refresh();
    } catch (cause) {
      reportError(cause);
    } finally {
      setBusyId(null);
    }
  };

  const cleanupSmbHost = async (connection: RemoteConnection) => {
    if (!window.confirm(t(
      "将断开当前 Windows 用户中所有来自 {host} 的 SMB 映射，包括非本应用创建的网络盘，并停用相关工作区。是否继续？",
      { host: connection.host },
    ))) return;
    setBusyId(`cleanup-smb:${connection.id}`);
    try {
      const result = await mountsCleanupSmbHost({ connectionId: connection.id });
      toast.success(t("已清理 {count} 个来自 {host} 的 SMB 映射", {
        count: result.removedCount,
        host: result.host,
      }));
    } catch (cause) {
      reportError(cause);
    } finally {
      await refresh();
      setBusyId(null);
    }
  };

  return {
    openCreateConnection,
    openEditConnection,
    updateConnectionForm,
    saveConnection,
    deleteConnection,
    cleanupSmbHost,
  };
}
