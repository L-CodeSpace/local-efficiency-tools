/*
 * 核心职责：装配远程连接、双协议探测和挂载工作区页面。
 * 业务痛点：SMB 与 FTP 的依赖和运行状态必须分开呈现，避免原生 SMB 被 FUSE 状态阻塞。
 * 能力边界：只负责页面布局，所有业务动作由 useRemoteMountsPage 提供。
 */

import { openPath, openUrl } from "@tauri-apps/plugin-opener";
import { Edit, ExternalLink, FolderCog, Plus, RefreshCw, Server, Trash2, Unplug } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { RuntimeDependencyPrompt } from "@/components/common/RuntimeDependencyPrompt";
import { cn } from "@/lib/utils";
import { useI18n } from "@/shared/i18n";
import { useRemoteMountsPage } from "./hooks";
import { ConnectionDialog } from "./index/connection-dialog";
import { ProbeWorkspace } from "./index/probe-workspace";
import { WorkspaceList } from "./index/workspace-list";

export default function RemoteMountsPage() {
  const page = useRemoteMountsPage();
  const { t } = useI18n();
  return (
    <div className="space-y-6 px-6 pb-12 animate-in fade-in duration-300">
      <header className="sticky top-0 z-10 -mx-6 flex flex-col justify-between gap-4 border-b bg-background/95 px-6 py-5 backdrop-blur md:flex-row md:items-start">
        <div>
          <h1 className="text-3xl font-bold">{t("远程挂载")}</h1>
          <p className="mt-1 text-muted-foreground">{t("原生 SMB 优先，FTP 自动聚合回退")}</p>
          <div className="mt-3 flex flex-wrap gap-2">
            <Badge variant={page.runtime?.installed ? "default" : "secondary"}>rclone {page.runtime?.version ?? page.runtime?.expectedVersion ?? t("检测中")}</Badge>
            <Badge variant="outline">{t("连接")} {page.connections.length}</Badge>
            <Badge variant="outline">{t("工作区")} {page.workspaces.length}</Badge>
            <Badge variant="outline">{t("已挂载")} {page.mountedCount}</Badge>
          </div>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button variant="outline" size="sm" onClick={page.refresh} disabled={page.loading}>
            <RefreshCw className={cn("h-4 w-4", page.loading && "animate-spin")} />{t("刷新")}
          </Button>
          <Button variant="outline" size="sm" onClick={page.unmountAll} disabled={page.busyId === "unmount-all"}>
            <Unplug className="h-4 w-4" />{t("全部卸载")}
          </Button>
          <Button size="sm" onClick={page.openCreateConnection}>
            <Plus className="h-4 w-4" />{t("新建连接")}
          </Button>
        </div>
      </header>

      {page.error ? <div className="border border-destructive/30 bg-destructive/5 p-3 text-sm text-destructive">{page.error}</div> : null}

      <RuntimeDependencyPrompt
        dependencyName="rclone"
        runtime="rclone"
        ready={Boolean(page.runtime?.installed)}
        loading={page.loading && !page.runtime}
        message={page.runtime ? `${t("尚未安装应用内 rclone，期望版本")}: ${page.runtime.expectedVersion}` : t("正在检测 rclone 运行时")}
        sourceName={page.runtime?.sourceName}
        sourceUrl={page.runtime?.sourceUrl}
        downloadSupported={page.runtime?.downloadSupported}
        refreshing={page.loading}
        onRefresh={page.refresh}
      />

      <section className="flex flex-col gap-3 border-b pb-5 md:flex-row md:items-center">
        <div className="min-w-0 flex-1">
          <div className="font-semibold">{t("配置与依赖")}</div>
          <div className="mt-1 text-sm text-muted-foreground">
            {t("原生 SMB 不需要 WinFsp 或 macFUSE；只有 FTP 聚合挂载需要系统挂载依赖。")}
          </div>
          <div className="mt-2 flex flex-wrap gap-2">
            <Badge variant={page.dependency?.ready ? "default" : "outline"}>{page.dependency?.dependencyName ?? "FUSE"} {page.dependency?.ready ? t("就绪") : t("未安装")}</Badge>
            <Badge variant="secondary">{page.uiContext?.platform ?? t("检测中")}</Badge>
          </div>
        </div>
        <div className="flex flex-wrap gap-2">
          {page.dependency?.installUrl && !page.dependency.ready ? (
            <Button variant="outline" size="sm" onClick={() => openUrl(page.dependency!.installUrl!)}>
              <ExternalLink className="h-4 w-4" />{t("安装挂载依赖")}
            </Button>
          ) : null}
          <Button variant="outline" size="sm" disabled={!page.uiContext?.configDir} onClick={() => page.uiContext?.configDir && openPath(page.uiContext.configDir)}>
            <FolderCog className="h-4 w-4" />{t("打开配置目录")}
          </Button>
        </div>
      </section>

      <ConnectionList page={page} />
      <ProbeWorkspace page={page} />
      <WorkspaceList page={page} />
      <ConnectionDialog page={page} />
    </div>
  );
}

function ConnectionList({ page }: { page: ReturnType<typeof useRemoteMountsPage> }) {
  const { t } = useI18n();
  return (
    <section className="space-y-3">
      <div>
        <h2 className="text-lg font-semibold">{t("远程连接")}</h2>
        <p className="text-sm text-muted-foreground">{t("一个连接保存一套 NAS 凭据，可探测并创建多个共享目录绑定。")}</p>
      </div>
      {page.connections.length === 0 ? (
        <div className="border border-dashed p-8 text-center text-sm text-muted-foreground">{t("暂无连接")}</div>
      ) : (
        <div className="divide-y border-y">
          {page.connections.map((connection) => (
            <div key={connection.id} className="flex flex-col gap-3 py-3 md:flex-row md:items-center">
              <Server className="h-5 w-5 shrink-0 text-primary" />
              <div className="min-w-0 flex-1">
                <div className="flex flex-wrap items-center gap-2">
                  <span className="font-semibold">{connection.name}</span>
                  <Badge variant="secondary">{connection.transportPreference === "auto" ? t("SMB 优先") : connection.transportPreference.toUpperCase()}</Badge>
                </div>
                <div className="mt-1 truncate font-mono text-xs text-muted-foreground">{connection.username}@{connection.host} · SMB {connection.smbPort} · FTP {connection.ftpPort}</div>
              </div>
              <div className="flex gap-1">
                <Button variant="ghost" size="icon-sm" onClick={() => page.openEditConnection(connection)} title={t("编辑连接")}><Edit className="h-4 w-4" /></Button>
                <Button variant="ghost" size="icon-sm" onClick={() => page.deleteConnection(connection)} disabled={page.busyId === `connection:${connection.id}`} title={t("删除连接")}><Trash2 className="h-4 w-4" /></Button>
              </div>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}
