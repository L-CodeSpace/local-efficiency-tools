/*
 * 核心职责：装配远程连接、协议探测和挂载工作区页面状态。
 * 能力边界：业务动作分别下沉到同名目录，入口只负责加载与状态组合。
 */

import { useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import {
  mountsCheckDependencies,
  mountsGetRuntimeStatus,
  mountsGetUiContext,
  mountsListConnections,
  mountsListWorkspaces,
  type ConnectionProbeResult,
  type MountDependencyStatus,
  type MountRuntimeStatus,
  type MountUiContext,
  type MountWorkspace,
  type RemoteConnection,
} from "@/api_tauri";
import { useI18n } from "@/shared/i18n";
import { logError } from "@/shared/state/logStore";
import { formatError } from "@/shared/utils/error";
import { createConnectionActions } from "./hooks/connection-actions";
import { createProbeActions } from "./hooks/probe-actions";
import { EMPTY_CONNECTION, type ConnectionForm, type ProbeWorkspaceRow } from "./hooks/model";
import { createWorkspaceActions } from "./hooks/workspace-actions";

export { transportLabel } from "./hooks/model";
export type { ConnectionForm, ProbeWorkspaceRow } from "./hooks/model";

export function useRemoteMountsPage() {
  const { t } = useI18n();
  const [connections, setConnections] = useState<RemoteConnection[]>([]);
  const [workspaces, setWorkspaces] = useState<MountWorkspace[]>([]);
  const [runtime, setRuntime] = useState<MountRuntimeStatus | null>(null);
  const [dependency, setDependency] = useState<MountDependencyStatus | null>(null);
  const [uiContext, setUiContext] = useState<MountUiContext | null>(null);
  const [connectionDialogOpen, setConnectionDialogOpen] = useState(false);
  const [connectionForm, setConnectionForm] = useState<ConnectionForm>(EMPTY_CONNECTION);
  const [probe, setProbe] = useState<ConnectionProbeResult | null>(null);
  const [probeConnectionId, setProbeConnectionId] = useState("");
  const [probeRows, setProbeRows] = useState<ProbeWorkspaceRow[]>([]);
  const [workspaceName, setWorkspaceName] = useState("");
  const [workspaceDrive, setWorkspaceDrive] = useState("");
  const [workspaceMountPoint, setWorkspaceMountPoint] = useState("");
  const [loading, setLoading] = useState(true);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const mountedCount = useMemo(() => workspaces.filter((item) => item.mounted).length, [workspaces]);
  const selectedTransport = probe?.recommendedTransport;
  const selectedRows = useMemo(
    () => probeRows.filter((row) => row.selected && row.accessible),
    [probeRows],
  );

  const reportError = (cause: unknown) => {
    const message = formatError(cause);
    setError(message);
    toast.error(message);
    logError(message);
  };

  const refresh = async () => {
    setLoading(true);
    setError(null);
    try {
      const [nextRuntime, nextDependency, nextContext, nextConnections, nextWorkspaces] = await Promise.all([
        mountsGetRuntimeStatus(),
        mountsCheckDependencies(),
        mountsGetUiContext(),
        mountsListConnections(),
        mountsListWorkspaces(),
      ]);
      setRuntime(nextRuntime);
      setDependency(nextDependency);
      setUiContext(nextContext);
      setConnections(nextConnections);
      setWorkspaces(nextWorkspaces);
      setProbeConnectionId((current) => current || nextConnections[0]?.id || "");
    } catch (cause) {
      reportError(cause);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void refresh();
  }, []);

  const resetProbe = () => {
    setProbe(null);
    setProbeRows([]);
  };
  const common = { t, refresh, reportError, setBusyId };
  const connectionActions = createConnectionActions({
    ...common,
    connectionForm,
    probeConnectionId,
    setConnectionDialogOpen,
    setConnectionForm,
    setProbeConnectionId,
    resetProbe,
  });
  const probeActions = createProbeActions({
    ...common,
    connections,
    dependency,
    uiContext,
    probe,
    probeConnectionId,
    selectedRows,
    selectedTransport,
    workspaceName,
    workspaceDrive,
    workspaceMountPoint,
    setProbe,
    setProbeRows,
    setWorkspaceName,
    setWorkspaceDrive,
    setWorkspaceMountPoint,
  });
  const workspaceActions = createWorkspaceActions(common);

  return {
    connections,
    workspaces,
    runtime,
    dependency,
    uiContext,
    connectionDialogOpen,
    setConnectionDialogOpen,
    connectionForm,
    probe,
    probeConnectionId,
    setProbeConnectionId,
    probeRows,
    selectedTransport,
    selectedRows,
    workspaceName,
    setWorkspaceName,
    workspaceDrive,
    setWorkspaceDrive,
    workspaceMountPoint,
    setWorkspaceMountPoint,
    loading,
    busyId,
    error,
    mountedCount,
    refresh,
    ...connectionActions,
    ...probeActions,
    ...workspaceActions,
  };
}
