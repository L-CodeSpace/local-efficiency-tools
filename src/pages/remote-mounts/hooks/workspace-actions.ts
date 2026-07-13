/*
 * 核心职责：处理既有挂载工作区的启停、刷新、修复和删除。
 * 能力边界：不处理连接编辑和远端目录探测。
 */

import { toast } from "sonner";
import {
  mountsDeleteWorkspace,
  mountsRefreshWorkspace,
  mountsRepairWorkspace,
  mountsSetWorkspaceEnabled,
  mountsUnmountAll,
  type MountWorkspace,
} from "@/api_tauri";
import type { ActionContext } from "./model";

export function createWorkspaceActions(context: ActionContext) {
  const run = async (busyId: string, action: () => Promise<unknown>, message: string, refresh = false) => {
    context.setBusyId(busyId);
    try {
      await action();
      toast.success(context.t(message));
      if (refresh) await context.refresh();
    } catch (cause) {
      context.reportError(cause);
    } finally {
      context.setBusyId(null);
    }
  };

  const toggleWorkspace = (workspace: MountWorkspace, enabled: boolean) => run(
    workspace.id,
    () => mountsSetWorkspaceEnabled({ id: workspace.id, enabled }),
    enabled ? "工作区已挂载" : "工作区已卸载",
    true,
  );

  const refreshWorkspace = (workspace: MountWorkspace) => run(
    `refresh:${workspace.id}`,
    () => mountsRefreshWorkspace({ id: workspace.id }),
    "挂载缓存已刷新",
  );

  const repairWorkspace = (workspace: MountWorkspace) => run(
    `repair:${workspace.id}`,
    () => mountsRepairWorkspace({ id: workspace.id }),
    "工作区已修复",
    true,
  );

  const deleteWorkspace = async (workspace: MountWorkspace) => {
    if (!window.confirm(context.t("确定删除该挂载工作区吗？"))) return;
    await run(
      `delete:${workspace.id}`,
      () => mountsDeleteWorkspace({ id: workspace.id }),
      "工作区已删除",
      true,
    );
  };

  const unmountAll = () => run("unmount-all", mountsUnmountAll, "已卸载全部挂载", true);

  return { toggleWorkspace, refreshWorkspace, repairWorkspace, deleteWorkspace, unmountAll };
}
