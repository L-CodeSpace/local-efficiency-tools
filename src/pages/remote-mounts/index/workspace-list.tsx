/*
 * 核心职责：展示连接分组下的原生 SMB 与 FTP 聚合工作区。
 * 业务痛点：用户需要直接看到实际传输方式、每个共享的本地目标和运行状态。
 * 能力边界：只渲染列表操作，不保存连接或执行探测。
 */

import { openPath } from "@tauri-apps/plugin-opener";
import { FolderOpen, RefreshCw, Trash2, Wrench } from "lucide-react";
import { toast } from "sonner";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { useI18n } from "@/shared/i18n";
import { formatError } from "@/shared/utils/error";
import { transportLabel, type useRemoteMountsPage } from "../hooks";

type Page = ReturnType<typeof useRemoteMountsPage>;

export function WorkspaceList({ page }: { page: Page }) {
  const { t } = useI18n();
  return (
    <section className="space-y-4">
      <div>
        <h2 className="text-lg font-semibold">{t("挂载工作区")}</h2>
        <p className="text-sm text-muted-foreground">{t("SMB 使用系统原生共享，FTP 使用已探测目录构建单一聚合入口。")}</p>
      </div>
      {page.workspaces.length === 0 ? (
        <div className="border border-dashed p-10 text-center text-sm text-muted-foreground">{t("暂无挂载工作区，请先保存连接并探测目录。")}</div>
      ) : (
        <div className="divide-y border-y">
          {page.workspaces.map((workspace) => {
            const connection = page.connections.find((item) => item.id === workspace.connectionId);
            const busy = page.busyId === workspace.id;
            return (
              <div key={workspace.id} className="space-y-4 py-4">
                <div className="flex flex-col gap-4 lg:flex-row lg:items-center">
                  <Switch checked={workspace.enabled} disabled={busy} onCheckedChange={(checked) => page.toggleWorkspace(workspace, checked)} />
                  <div className="min-w-0 flex-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="font-semibold">{workspace.name}</span>
                      <Badge variant="secondary">{t(transportLabel(workspace.effectiveTransport))}</Badge>
                      <Badge variant={workspace.mounted ? "default" : "outline"}>{workspace.mounted ? t("已挂载") : workspace.enabled ? t("已停止") : t("已停用")}</Badge>
                    </div>
                    <div className="mt-1 text-sm text-muted-foreground">{connection?.name ?? t("未知连接")} · {connection?.host}</div>
                    {workspace.error ? <div className="mt-2 text-sm text-destructive">{workspace.error}</div> : null}
                  </div>
                  <div className="flex flex-wrap gap-2">
                    <Button variant="outline" size="sm" disabled={!workspace.mounted || page.busyId === `refresh:${workspace.id}`} onClick={() => page.refreshWorkspace(workspace)}>
                      <RefreshCw className="h-4 w-4" />{workspace.effectiveTransport === "nativeSmb" ? t("刷新状态") : t("刷新缓存")}
                    </Button>
                    <Button variant="outline" size="sm" disabled={page.busyId === `repair:${workspace.id}`} onClick={() => page.repairWorkspace(workspace)}>
                      <Wrench className="h-4 w-4" />{t("修复")}
                    </Button>
                    <Button variant="ghost" size="icon-sm" disabled={page.busyId === `delete:${workspace.id}`} onClick={() => page.deleteWorkspace(workspace)} title={t("删除工作区")}>
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </div>
                </div>
                <div className="grid gap-2 md:grid-cols-2 xl:grid-cols-3">
                  {workspaceTargets(workspace).map((target) => (
                    <button key={`${target.name}:${target.path}`} type="button" className="flex min-w-0 items-center gap-3 border p-3 text-left transition-colors hover:bg-muted/50 disabled:cursor-not-allowed disabled:opacity-50" disabled={!workspace.mounted || !target.path} onClick={() => openTarget(target.path)}>
                      <FolderOpen className="h-4 w-4 shrink-0 text-primary" />
                      <span className="min-w-0">
                        <span className="block truncate text-sm font-medium">{target.name}</span>
                        <span className="block truncate font-mono text-xs text-muted-foreground">{target.path || t("自动分配")}</span>
                      </span>
                    </button>
                  ))}
                </div>
              </div>
            );
          })}
        </div>
      )}
    </section>
  );
}

function workspaceTargets(workspace: Page["workspaces"][number]) {
  if (workspace.effectiveTransport === "nativeSmb") {
    return workspace.bindings.map((binding) => ({
      name: binding.name,
      path: binding.driveLetter ? `${binding.driveLetter}\\` : binding.mountPoint ?? "",
    }));
  }
  return [{
    name: workspace.name,
    path: workspace.driveLetter ? `${workspace.driveLetter}\\` : workspace.mountPoint ?? "",
  }];
}

async function openTarget(path: string) {
  try {
    await openPath(path);
  } catch (cause) {
    toast.error(formatError(cause));
  }
}
