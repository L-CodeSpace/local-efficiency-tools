/*
 * 核心职责：执行连接探测并从可访问目录创建挂载工作区。
 * 能力边界：不维护连接表单，不处理已创建工作区的生命周期。
 */

import type { Dispatch, SetStateAction } from "react";
import { toast } from "sonner";
import {
  mountsCreateWorkspace,
  mountsProbeConnection,
  type ConnectionProbeResult,
  type EffectiveTransport,
  type MountDependencyStatus,
  type MountUiContext,
  type ProbeShareEntry,
  type RemoteConnection,
} from "@/api_tauri";
import { logSuccess } from "@/shared/state/logStore";
import { joinPath, type ActionContext, type ProbeWorkspaceRow } from "./model";

type Context = ActionContext & {
  connections: RemoteConnection[];
  dependency: MountDependencyStatus | null;
  uiContext: MountUiContext | null;
  probe: ConnectionProbeResult | null;
  probeConnectionId: string;
  selectedRows: ProbeWorkspaceRow[];
  selectedTransport?: EffectiveTransport;
  workspaceName: string;
  workspaceDrive: string;
  workspaceMountPoint: string;
  setProbe: Dispatch<SetStateAction<ConnectionProbeResult | null>>;
  setProbeRows: Dispatch<SetStateAction<ProbeWorkspaceRow[]>>;
  setWorkspaceName: Dispatch<SetStateAction<string>>;
  setWorkspaceDrive: Dispatch<SetStateAction<string>>;
  setWorkspaceMountPoint: Dispatch<SetStateAction<string>>;
};

export function createProbeActions(context: Context) {
  const updateProbeRow = (path: string, patch: Partial<ProbeWorkspaceRow>) => {
    context.setProbeRows((rows) => rows.map((row) => (row.path === path ? { ...row, ...patch } : row)));
  };

  const probeSelectedConnection = async () => {
    if (!context.probeConnectionId) return;
    context.setBusyId("probe");
    try {
      const result = await mountsProbeConnection({ connectionId: context.probeConnectionId });
      const connection = context.connections.find((item) => item.id === context.probeConnectionId);
      context.setProbe(result);
      context.setProbeRows(activeEntries(result).map(toWorkspaceRow));
      context.setWorkspaceName(connection?.name ?? context.t("远程工作区"));
      context.setWorkspaceDrive(context.uiContext?.defaultDriveLetter ?? "");
      context.setWorkspaceMountPoint(
        joinPath(context.uiContext?.defaultMountRoot ?? "", connection?.name ?? "nas"),
      );
      toast.success(context.t("远端目录探测完成"));
    } catch (cause) {
      context.reportError(cause);
    } finally {
      context.setBusyId(null);
    }
  };

  const createWorkspace = async () => {
    const transport = context.selectedTransport;
    if (!context.probe || !transport || context.selectedRows.length === 0) {
      toast.error(context.t("请选择至少一个可访问目录"));
      return;
    }
    if (transport === "ftpCombine" && !context.dependency?.ready) {
      toast.error(context.t("FTP 聚合挂载需要先安装系统挂载依赖"));
      return;
    }
    context.setBusyId("create-workspace");
    try {
      await mountsCreateWorkspace({
        input: {
          connectionId: context.probe.connectionId,
          name: context.workspaceName || context.t("远程工作区"),
          effectiveTransport: transport,
          bindings: context.selectedRows.map((row) => ({
            name: row.name,
            remotePath: row.path,
            driveLetter: transport === "nativeSmb" ? row.driveLetter || undefined : undefined,
            mountPoint: transport === "nativeSmb" ? row.mountPoint || undefined : undefined,
          })),
          driveLetter: transport === "ftpCombine" ? context.workspaceDrive || undefined : undefined,
          mountPoint: transport === "ftpCombine" ? context.workspaceMountPoint || undefined : undefined,
          enabled: true,
        },
      });
      toast.success(context.t("工作区已创建并挂载"));
      logSuccess(`工作区已创建：${context.workspaceName}`);
      context.setProbe(null);
      context.setProbeRows([]);
      await context.refresh();
    } catch (cause) {
      context.reportError(cause);
    } finally {
      context.setBusyId(null);
    }
  };

  return { updateProbeRow, probeSelectedConnection, createWorkspace };
}

function activeEntries(result: ConnectionProbeResult): ProbeShareEntry[] {
  if (result.recommendedTransport === "nativeSmb") return result.smb.entries;
  if (result.recommendedTransport === "ftpCombine") return result.ftp.entries;
  return [];
}

function toWorkspaceRow(entry: ProbeShareEntry): ProbeWorkspaceRow {
  return {
    ...entry,
    selected: entry.accessible,
    driveLetter: entry.suggestedDriveLetter ?? "",
    mountPoint: entry.suggestedMountPoint ?? "",
  };
}
