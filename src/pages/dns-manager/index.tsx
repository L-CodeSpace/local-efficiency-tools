import { useMemo, useState, type FormEvent, type MouseEvent } from "react";
import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
import { toast } from "sonner";
import { FolderOpen, Plus, RefreshCw, Search, ShieldAlert, ShieldCheck, Trash2, Wrench } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useI18n } from "@/shared/i18n";
import { logError } from "@/shared/state/logStore";
import { formatError } from "@/shared/utils/error";
import { useDnsManagerPage } from "./hooks";

type Filter = "all" | "enabled" | "disabled";

export default function DnsManagerPage() {
  const page = useDnsManagerPage();
  const { t } = useI18n();
  const [filter, setFilter] = useState<Filter>("all");
  const [search, setSearch] = useState("");

  const displayedEntries = useMemo(
    () =>
      page.entries.filter((entry) => {
        if (entry.isCommentOrBlank) return false;
        if (filter === "enabled" && !entry.enabled) return false;
        if (filter === "disabled" && entry.enabled) return false;
        if (!search) return true;
        const query = search.toLowerCase();
        return entry.hosts.some((host) => host.toLowerCase().includes(query)) || (entry.ip?.toLowerCase().includes(query) ?? false);
      }),
    [filter, page.entries, search],
  );

  const enabledCount = page.entries.filter((entry) => !entry.isCommentOrBlank && entry.enabled).length;
  const disabledCount = page.entries.filter((entry) => !entry.isCommentOrBlank && !entry.enabled).length;
  const canManageHelper = Boolean(page.helperStatus?.required && page.helperStatus.installSupported);
  const helperNeedsAttention = Boolean(
    page.helperStatus &&
      (!page.helperStatus.installed || !page.helperStatus.running || !page.helperStatus.tokenExists || page.helperStatus.needsRepair),
  );
  const helperName =
    page.helperStatus?.helperKind === "macosLaunchDaemon"
      ? "macOS hosts helper"
      : page.helperStatus?.helperKind === "windowsService"
        ? "Windows hosts helper"
        : "hosts helper";
  const helperTitle = page.helperStatus?.installed
    ? helperNeedsAttention
      ? t("{name} 需要修复", { name: helperName })
      : t("{name} 正在运行", { name: helperName })
    : t("需要安装 {name}", { name: helperName });

  const handleHelperAction = async () => {
    if (!page.helperStatus) return;
    const ok = page.helperStatus.installed ? await page.repairHelper() : await page.installHelper();
    if (ok) toast.success(page.helperStatus.installed ? t("hosts helper 已修复") : t("hosts helper 已安装"));
  };

  const handleHelperUninstall = async () => {
    const ok = await page.uninstallHelper();
    if (ok) toast.success(t("hosts helper 已卸载"));
  };

  const handleAdd = async (event: FormEvent) => {
    event.preventDefault();
    const ok = await page.addEntry();
    if (ok) toast.success(t("已添加 {name}", { name: page.newDomain || t("记录") }));
  };

  const handleOpenHost = async (event: MouseEvent<HTMLButtonElement>, host: string) => {
    event.preventDefault();
    event.stopPropagation();
    const url = hostToUrl(host);
    try {
      await openUrl(url);
    } catch (err) {
      const message = formatError(err);
      toast.error(`${t("打开失败")}: ${message}`);
      logError(`打开 DNS 域名失败：${url}：${message}`);
    }
  };

  return (
    <div className="space-y-6 px-6 pb-12 animate-in fade-in duration-300">
      <div className="sticky top-0 z-10 -mx-6 flex flex-col justify-between gap-4 border-b bg-background/95 px-6 py-6 backdrop-blur supports-[backdrop-filter]:bg-background/60 md:flex-row md:items-start">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">{t("DNS 管理")}</h1>
          <p className="mt-1 text-muted-foreground">{t("编辑本地 hosts 文件，管理 DNS 解析记录")}</p>
          <div className="mt-2 inline-block rounded bg-muted/30 px-2 py-1 font-mono text-xs text-muted-foreground/80">
            {page.hostsPath || `${t("检测系统路径中")}...`}
          </div>
        </div>
        <div className="flex items-center gap-3">
          <Badge variant="outline" className="bg-background px-3 py-1 text-sm">
            <span className="mr-1 font-bold text-green-500">{enabledCount}</span> 激活
            <span className="mx-2 text-muted-foreground">/</span>
            <span className="mr-1 text-muted-foreground">{disabledCount}</span> 关闭
          </Badge>
          <Button onClick={() => page.hostsPath && revealItemInDir(page.hostsPath)} disabled={!page.hostsPath} variant="outline" size="sm" title="在文件管理器中定位 hosts 文件">
            <FolderOpen className="h-4 w-4" /> {t("打开文件")}
          </Button>
          <Button onClick={page.refresh} disabled={page.loading} variant="outline" size="sm">
            <RefreshCw className={`h-4 w-4 ${page.loading ? "animate-spin" : ""}`} /> {t("刷新")}
          </Button>
        </div>
      </div>

      {canManageHelper && page.helperStatus ? (
        <div
          className={`flex flex-col gap-3 rounded-lg border p-4 md:flex-row md:items-center ${
            helperNeedsAttention
              ? "border-amber-500/20 bg-amber-500/10 text-amber-900 dark:text-amber-200"
              : "border-border bg-muted/20 text-foreground"
          }`}
        >
          {helperNeedsAttention ? <ShieldAlert className="h-5 w-5 shrink-0" /> : <ShieldCheck className="h-5 w-5 shrink-0 text-green-600" />}
          <div className="flex-1 text-sm">
            <strong className="mb-1 block font-semibold">{helperTitle}</strong>
            <p>{page.helperStatus.message}</p>
            <p className="mt-1 text-xs opacity-80">
              {t("安装或修复后，后续添加、启用、停用、删除记录将通过受限后台服务写入 hosts；卸载后会恢复为系统授权写入。")}
            </p>
          </div>
          <div className="flex shrink-0 gap-2">
            {!page.helperStatus.installed || helperNeedsAttention ? (
              <Button onClick={handleHelperAction} disabled={page.helperBusy} variant="outline" size="sm" className="bg-background">
                {page.helperStatus.installed ? <Wrench className="h-4 w-4" /> : <ShieldCheck className="h-4 w-4" />}
                {page.helperBusy ? `${t("处理中")}...` : page.helperStatus.installed ? t("修复 Helper") : t("安装 Helper")}
              </Button>
            ) : null}
            {page.helperStatus.installed ? (
              <Button onClick={handleHelperUninstall} disabled={page.helperBusy} variant="ghost" size="sm" title="卸载后将恢复为系统授权写入 hosts">
                {t("卸载 Helper")}
              </Button>
            ) : null}
          </div>
        </div>
      ) : null}

      {page.error ? (
        <div className="flex flex-col gap-3 rounded-lg border border-destructive/20 bg-destructive/10 p-4 text-destructive md:flex-row">
          <ShieldAlert className="h-5 w-5 shrink-0" />
          <div className="flex-1 text-sm">
            <strong className="mb-1 block font-semibold">{t("操作失败")}</strong>
            <p>{page.error}</p>
          </div>
        </div>
      ) : null}

      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="flex items-center gap-2 text-lg">
            <Plus className="h-5 w-5 text-primary" />
            {t("添加 DNS 解析")}
          </CardTitle>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleAdd} className="flex flex-col items-end gap-4 md:flex-row">
            <div className="flex-1 space-y-2">
              <Label htmlFor="ip-input">{t("IP 地址")}</Label>
              <Input id="ip-input" placeholder={t("例如 127.0.0.1")} value={page.newIp} onChange={(event) => page.setNewIp(event.target.value)} autoComplete="off" spellCheck={false} />
            </div>
            <div className="flex-1 space-y-2">
              <Label htmlFor="domain-input">{t("域名")}</Label>
              <Input id="domain-input" placeholder={t("例如 my-service.local")} value={page.newDomain} onChange={(event) => page.setNewDomain(event.target.value)} autoComplete="off" spellCheck={false} />
            </div>
            <Button type="submit" disabled={page.loading || !page.newIp.trim() || !page.newDomain.trim()} className="w-full md:w-auto">
              {page.loading ? `${t("添加中")}...` : t("添加记录")}
            </Button>
          </form>
        </CardContent>
      </Card>

      <div className="flex flex-col items-center justify-between gap-4 rounded-xl border bg-muted/20 p-2 sm:flex-row">
        <div className="relative w-full sm:max-w-xs">
          <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
          <Input placeholder={`${t("搜索 IP 或域名")}...`} value={search} onChange={(event) => setSearch(event.target.value)} className="bg-background pl-9" />
        </div>
        <Tabs value={filter} onValueChange={(value) => setFilter(value as Filter)} className="w-full sm:w-auto">
          <TabsList className="grid w-full grid-cols-3">
            <TabsTrigger value="all">{t("全部")}</TabsTrigger>
            <TabsTrigger value="enabled">{t("已激活")}</TabsTrigger>
            <TabsTrigger value="disabled">{t("已关闭")}</TabsTrigger>
          </TabsList>
        </Tabs>
      </div>

      {!page.loading && displayedEntries.length === 0 ? (
        <div className="rounded-xl border border-dashed bg-muted/10 px-4 py-16 text-center">
          <div className="mb-4 text-4xl">📭</div>
          <p className="text-muted-foreground">{search || filter !== "all" ? t("没有找到匹配的 DNS 记录。") : t("暂无自定义解析。请在上方表单添加第一条记录。")}</p>
        </div>
      ) : (
        <div className="grid gap-3">
          {displayedEntries.map((entry, index) => (
            <Card key={`${entry.ip}-${entry.hosts.join(",")}-${index}`} className={`overflow-hidden transition-all duration-200 ${entry.enabled ? "border-primary/20 bg-card shadow-sm" : "border-muted bg-muted/10 opacity-70"}`}>
              <div className="flex items-center gap-4 px-4 py-2">
                <Switch checked={entry.enabled} onCheckedChange={(checked) => page.toggleEntry(entry, checked)} />
                <div className="min-w-0 flex-1">
                  <div className="mb-1.5 flex flex-wrap gap-1.5">
                    {entry.hosts.map((host) => (
                      <button
                        key={host}
                        type="button"
                        title={t("使用默认浏览器打开")}
                        onClick={(event) => handleOpenHost(event, host)}
                        className={`cursor-pointer rounded px-2 py-0.5 text-sm font-medium transition-colors hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring ${
                          entry.enabled ? "bg-primary/10 text-primary hover:bg-primary/15" : "bg-muted text-muted-foreground hover:bg-muted/80"
                        }`}
                      >
                        {host}
                      </button>
                    ))}
                  </div>
                  <div className="flex items-center font-mono text-sm text-muted-foreground">
                    <span className={`mr-2 h-2 w-2 rounded-full ${entry.enabled ? "bg-green-500" : "bg-muted-foreground/30"}`} />
                    {entry.ip}
                  </div>
                </div>
                <Badge variant={entry.enabled ? "default" : "secondary"} className="hidden sm:inline-flex">
                  {entry.enabled ? t("生效中") : t("已停用")}
                </Badge>
                <Button variant="ghost" size="icon-sm" className="text-destructive hover:bg-destructive/10 hover:text-destructive" onClick={() => page.removeEntry(entry)} title="删除记录">
                  <Trash2 className="h-4 w-4" />
                </Button>
              </div>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}

function hostToUrl(host: string) {
  const trimmed = host.trim();
  if (/^https?:\/\//i.test(trimmed)) return trimmed;
  return `http://${trimmed}`;
}
