/*
 * 核心职责：远程挂载页面路由入口。
 * 业务痛点：列表、依赖提示和配置表单混在一起会形成难以维护的巨型页面。
 * 能力边界：只负责页面骨架、运行时提示和 profile 列表装配。
 */

import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
import {
  AlertTriangle,
  CheckCircle2,
  Edit,
  ExternalLink,
  FolderOpen,
  HardDrive,
  Link2,
  Plus,
  RefreshCw,
  ScrollText,
  Trash2,
  Unplug,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { RuntimeDependencyPrompt } from "@/components/common/RuntimeDependencyPrompt";
import { Switch } from "@/components/ui/switch";
import { cn } from "@/lib/utils";
import { useI18n } from "@/shared/i18n";
import { profileTarget, protocolLabel, useRemoteMountsPage } from "./hooks";
import { ConfigPaths } from "./index/config-paths";
import { MountLogDialog } from "./index/log-dialog";
import { ProfileDialog } from "./index/profile-dialog";

export default function RemoteMountsPage() {
  const page = useRemoteMountsPage();
  const { t } = useI18n();
  const dependencyReady = page.deps?.ready ?? false;
  const supportsDriveLetter = page.uiContext?.supportsDriveLetter ?? false;

  return (
    <div className="space-y-6 px-6 pb-12 animate-in fade-in duration-300">
      <div className="sticky top-0 z-10 -mx-6 flex flex-col justify-between gap-4 border-b bg-background/95 px-6 py-6 backdrop-blur supports-[backdrop-filter]:bg-background/60 md:flex-row md:items-start">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">{t("远程挂载")}</h1>
          <p className="mt-1 text-muted-foreground">{t("使用 rclone 将 FTP、SFTP、WebDAV 映射为本地磁盘或目录")}</p>
          <div className="mt-3 flex flex-wrap gap-2">
            <Badge variant={page.runtime?.installed ? "default" : "secondary"}>
              rclone {page.runtime?.version ?? page.runtime?.expectedVersion ?? "检测中"}
            </Badge>
            <Badge variant={dependencyReady ? "default" : "outline"}>
              {page.deps?.dependencyName ?? t("挂载依赖")} {dependencyReady ? t("就绪") : t("未就绪")}
            </Badge>
          </div>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Badge variant="outline" className="bg-background px-3 py-1 text-sm">
            <span className="mr-1 font-bold text-green-600 dark:text-green-400">{page.mountedCount}</span>
            {t("挂载")}
            <span className="mx-2 text-muted-foreground">/</span>
            <span className="mr-1 text-muted-foreground">{page.enabledCount}</span>
            {t("启用")}
          </Badge>
          <Button variant="outline" size="sm" onClick={page.refresh} disabled={page.loading}>
            <RefreshCw className={cn("h-4 w-4", page.loading && "animate-spin")} />
            {t("刷新")}
          </Button>
          <Button variant="outline" size="sm" onClick={page.unmountAll} disabled={page.busyId === "all"}>
            <Unplug className="h-4 w-4" />
            {t("全部卸载")}
          </Button>
          <Button size="sm" onClick={page.openCreateDialog}>
            <Plus className="h-4 w-4" />
            {t("新建挂载")}
          </Button>
        </div>
      </div>

      {page.error ? (
        <div className="rounded-md border border-destructive/20 bg-destructive/10 p-3 text-sm text-destructive">
          {page.error}
        </div>
      ) : null}

      {page.deps && !page.deps.ready ? (
        <div className="flex flex-col gap-3 rounded-md border border-amber-500/20 bg-amber-500/10 p-4 text-sm text-amber-900 dark:text-amber-200 md:flex-row md:items-center">
          <AlertTriangle className="h-5 w-5 shrink-0" />
          <div className="flex-1">
            <div className="font-semibold">{t("需要安装")} {page.deps.dependencyName}</div>
            <div className="mt-1">{page.deps.message}</div>
          </div>
          {page.deps.installUrl ? (
            <Button variant="outline" size="sm" className="bg-background" onClick={() => openUrl(page.deps!.installUrl!)}>
              <ExternalLink className="h-4 w-4" />
              {t("官方安装")}
            </Button>
          ) : null}
        </div>
      ) : null}

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

      <ConfigPaths context={page.uiContext} />

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <HardDrive className="h-5 w-5 text-primary" />
            {t("挂载 Profile")}
          </CardTitle>
          <CardDescription>{t("配置远程服务、测试连接并控制本地挂载状态")}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          {page.profiles.length === 0 && !page.loading ? (
            <div className="rounded-md border border-dashed p-10 text-center text-sm text-muted-foreground">
              {t("暂无远程挂载配置。")}
            </div>
          ) : null}

          {page.profiles.map((profile) => {
            const target = profileTarget(profile);
            const busy = page.busyId === profile.id;
            return (
              <div key={profile.id} className="flex flex-col gap-4 rounded-md border p-4 lg:flex-row lg:items-center">
                <div className="flex min-w-0 flex-1 items-start gap-4">
                  <Switch
                    className="mt-1"
                    checked={profile.enabled}
                    disabled={busy || (!profile.enabled && !dependencyReady)}
                    onCheckedChange={(checked) => page.toggleProfile(profile, checked)}
                  />
                  <div className="min-w-0 flex-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <div className="truncate font-semibold">{profile.name}</div>
                      <Badge variant="secondary">{protocolLabel(profile.protocol)}</Badge>
                      {profile.mounted ? (
                        <Badge className="gap-1">
                          <CheckCircle2 className="h-3 w-3" />
                          {t("已挂载")}
                        </Badge>
                      ) : (
                        <Badge variant="outline">{profile.enabled ? t("已启用") : t("已停用")}</Badge>
                      )}
                    </div>
                    <div className="mt-2 grid gap-1 font-mono text-xs text-muted-foreground">
                      <div className="truncate">
                        {t("远程")}: {profile.protocol === "webdav" ? profile.url : profile.host}
                        {profile.remotePath ? `/${profile.remotePath.replace(/^\/+/, "")}` : ""}
                      </div>
                      <div className="truncate">{t("本地")}: {target}</div>
                    </div>
                  </div>
                </div>
                <div className="flex flex-wrap justify-end gap-2">
                  <Button variant="outline" size="sm" disabled={busy} onClick={() => page.testProfile(profile)}>
                    <Link2 className="h-4 w-4" />
                    {t("测试")}
                  </Button>
                  <Button
                    variant="outline"
                    size="sm"
                    disabled={!profile.mounted || target === "自动分配"}
                    onClick={() => revealItemInDir(target)}
                  >
                    <FolderOpen className="h-4 w-4" />
                    {t("打开")}
                  </Button>
                  <Button variant="ghost" size="icon-sm" onClick={() => page.openEditDialog(profile)} title="编辑挂载">
                    <Edit className="h-4 w-4" />
                  </Button>
                  <Button variant="ghost" size="icon-sm" onClick={() => page.logs.openLog(profile)} title="查看 rclone 日志">
                    <ScrollText className="h-4 w-4" />
                  </Button>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    className="text-destructive hover:bg-destructive/10 hover:text-destructive"
                    disabled={busy}
                    onClick={() => page.deleteProfile(profile)}
                    title="删除挂载"
                  >
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </div>
              </div>
            );
          })}
        </CardContent>
      </Card>

      <MountLogDialog logs={page.logs} />
      <ProfileDialog page={page} supportsDriveLetter={supportsDriveLetter} />
    </div>
  );
}
