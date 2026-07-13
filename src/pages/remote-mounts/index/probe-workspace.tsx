/*
 * 核心职责：展示 SMB/FTP 探测结果并创建目录工作区。
 * 业务痛点：端口、认证和目录权限必须直接打印，不能只在日志里给出模糊失败信息。
 * 能力边界：只渲染探测结果、目录选择和本地目标输入。
 */

import { AlertTriangle, CheckCircle2, FolderSearch, LoaderCircle, Network } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { cn } from "@/lib/utils";
import { useI18n } from "@/shared/i18n";
import { transportLabel, type useRemoteMountsPage } from "../hooks";

type Page = ReturnType<typeof useRemoteMountsPage>;

export function ProbeWorkspace({ page }: { page: Page }) {
  const { t } = useI18n();
  const probing = page.busyId === "probe";
  return (
    <section className="border-y py-5">
      <div className="flex flex-col gap-4 lg:flex-row lg:items-end">
        <div className="min-w-0 flex-1 space-y-2">
          <Label>{t("探测远端共享目录")}</Label>
          <Select value={page.probeConnectionId} onValueChange={page.setProbeConnectionId}>
            <SelectTrigger className="w-full"><SelectValue placeholder={t("选择连接")} /></SelectTrigger>
            <SelectContent>
              {page.connections.map((connection) => (
                <SelectItem key={connection.id} value={connection.id}>{connection.name} · {connection.host}</SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
        <Button onClick={page.probeSelectedConnection} disabled={!page.probeConnectionId || probing || !page.runtime?.installed}>
          {probing ? <LoaderCircle className="h-4 w-4 animate-spin" /> : <FolderSearch className="h-4 w-4" />}
          {t("探测 SMB / FTP")}
        </Button>
      </div>

      {page.probe ? (
        <div className="mt-5 space-y-5">
          <div className="flex flex-wrap items-center gap-2">
            <Badge>{t(transportLabel(page.probe.recommendedTransport))}</Badge>
            <span className="text-sm text-muted-foreground">{t("最终推荐传输方式")}</span>
            {page.probe.fallbackReason ? <span className="text-sm text-amber-700 dark:text-amber-300">{page.probe.fallbackReason}</span> : null}
          </div>
          <div className="grid gap-4 lg:grid-cols-2">
            <TransportResult title="SMB" result={page.probe.smb} />
            <TransportResult title="FTP" result={page.probe.ftp} />
          </div>
          {page.selectedTransport ? <WorkspaceBuilder page={page} /> : (
            <div className="flex items-center gap-2 border border-destructive/30 bg-destructive/5 p-3 text-sm text-destructive">
              <AlertTriangle className="h-4 w-4" />
              {t("SMB 和 FTP 均未通过认证，无法创建工作区。")}
            </div>
          )}
        </div>
      ) : null}
    </section>
  );
}

function TransportResult({ title, result }: { title: string; result: Page["probe"] extends infer _T ? NonNullable<Page["probe"]>["smb"] : never }) {
  const { t } = useI18n();
  return (
    <div className="border p-4">
      <div className="flex items-center justify-between gap-3">
        <div className="flex items-center gap-2 font-semibold"><Network className="h-4 w-4" />{title}</div>
        <Badge variant={result.authenticated ? "default" : "outline"}>{result.authenticated ? t("认证成功") : t("不可用")}</Badge>
      </div>
      <p className="mt-2 text-sm text-muted-foreground">{result.message}</p>
      <div className="mt-3 divide-y border-y">
        {result.entries.map((entry) => (
          <div key={entry.path} className="flex items-center justify-between gap-3 py-2 text-sm">
            <span className="min-w-0 truncate font-mono">{entry.path}</span>
            <span className={cn("shrink-0", entry.accessible ? "text-green-600" : "text-destructive")}>
              {entry.accessible ? t("可访问") : entry.error || t("无权限")}
            </span>
          </div>
        ))}
        {result.authenticated && result.entries.length === 0 ? <div className="py-3 text-sm text-muted-foreground">{t("未发现目录")}</div> : null}
      </div>
      {result.rawOutput ? (
        <details className="mt-3 text-xs">
          <summary className="cursor-pointer text-muted-foreground">{t("原始探测输出")}</summary>
          <pre className="mt-2 max-h-32 overflow-auto whitespace-pre-wrap border bg-muted/30 p-2">{result.rawOutput}</pre>
        </details>
      ) : null}
    </div>
  );
}

function WorkspaceBuilder({ page }: { page: Page }) {
  const { t } = useI18n();
  const isWindows = page.uiContext?.platform === "windows";
  const nativeSmb = page.selectedTransport === "nativeSmb";
  return (
    <div className="space-y-4 border-t pt-5">
      <div className="grid gap-4 md:grid-cols-2">
        <Field label={t("工作区名称")}><Input value={page.workspaceName} onChange={(event) => page.setWorkspaceName(event.target.value)} /></Field>
        {!nativeSmb && isWindows ? (
          <Field label={t("聚合盘符")}><Input value={page.workspaceDrive} onChange={(event) => page.setWorkspaceDrive(event.target.value)} placeholder={page.uiContext?.defaultDriveLetter ?? "Z:"} /></Field>
        ) : null}
        {!nativeSmb && !isWindows ? (
          <Field label={t("聚合目录")}><Input value={page.workspaceMountPoint} onChange={(event) => page.setWorkspaceMountPoint(event.target.value)} /></Field>
        ) : null}
      </div>
      <div className="divide-y border-y">
        {page.probeRows.map((row) => (
          <div key={row.path} className="grid gap-3 py-3 md:grid-cols-[auto_minmax(0,1fr)_minmax(10rem,0.7fr)] md:items-center">
            <Checkbox checked={row.selected} disabled={!row.accessible} onCheckedChange={(checked) => page.updateProbeRow(row.path, { selected: checked === true })} />
            <div className="min-w-0">
              <div className="truncate font-medium">{row.name}</div>
              <div className="truncate font-mono text-xs text-muted-foreground">{row.path}</div>
            </div>
            {nativeSmb ? (
              isWindows ? (
                <Input value={row.driveLetter} onChange={(event) => page.updateProbeRow(row.path, { driveLetter: event.target.value })} placeholder="Z:" />
              ) : (
                <Input value={row.mountPoint} onChange={(event) => page.updateProbeRow(row.path, { mountPoint: event.target.value })} />
              )
            ) : <span className="text-xs text-muted-foreground">{t("将显示在聚合入口下")}</span>}
          </div>
        ))}
      </div>
      {page.selectedTransport === "ftpCombine" && !page.dependency?.ready ? (
        <div className="flex items-center gap-2 text-sm text-amber-700 dark:text-amber-300">
          <AlertTriangle className="h-4 w-4" />{t("FTP 聚合挂载需要先安装 {name}", { name: page.dependency?.dependencyName ?? "FUSE" })}
        </div>
      ) : null}
      <div className="flex justify-end">
        <Button onClick={page.createWorkspace} disabled={page.busyId === "create-workspace" || page.selectedRows.length === 0 || (page.selectedTransport === "ftpCombine" && !page.dependency?.ready)}>
          <CheckCircle2 className="h-4 w-4" />{t("创建并挂载")}
        </Button>
      </div>
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return <div className="space-y-2"><Label>{label}</Label>{children}</div>;
}
