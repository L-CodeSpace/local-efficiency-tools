/*
 * 核心职责：展示设置页运行时资源行。
 * 业务痛点：下载、复制和打开运行时路径的 UI 会让设置页入口过长。
 * 能力边界：只负责单个运行时资源行和相关按钮动作。
 */

import type { Ref } from "react";
import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
import { Copy, Download, ExternalLink, FolderOpen } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { useI18n } from "@/shared/i18n";

export function RuntimeResourceLine({
  lineRef,
  highlighted,
  name,
  status,
  ready,
  detail,
  path,
  emptyPath,
  sourceName,
  sourceUrl,
  downloadSupported,
  downloading,
  canOpen,
  onDownload,
}: {
  lineRef?: Ref<HTMLDivElement>;
  highlighted?: boolean;
  name: string;
  status: string;
  ready: boolean;
  detail: string;
  path?: string;
  emptyPath: string;
  sourceName?: string;
  sourceUrl?: string;
  downloadSupported: boolean;
  downloading: boolean;
  canOpen: boolean;
  onDownload: () => void;
}) {
  const { t } = useI18n();

  return (
    <div
      ref={lineRef}
      className={cn(
        "grid gap-3 border p-4 transition-colors hover:bg-muted/50 md:grid-cols-[minmax(0,1fr)_auto] md:items-center",
        highlighted && "bg-primary/5 ring-2 ring-primary/40 ring-offset-2 ring-offset-background",
      )}
    >
      <div className="min-w-0 space-y-2">
        <div className="flex flex-wrap items-center gap-2">
          <div className="text-sm font-medium">{name}</div>
          <span className={`border px-2 py-0.5 text-xs ${ready ? "border-primary/30 bg-primary/10 text-primary" : "border-border bg-muted text-muted-foreground"}`}>
            {status}
          </span>
        </div>
        <div className="text-xs text-muted-foreground">{detail}</div>
        <div className={`break-all font-mono text-xs ${path ? "text-foreground" : "text-muted-foreground"}`}>
          {path ?? emptyPath}
        </div>
        <div className="break-all text-xs text-muted-foreground">
          {t("来源")}：{sourceName && sourceUrl ? `${sourceName} · ${sourceUrl}` : t("当前平台未配置下载源")}
        </div>
      </div>
      <div className="flex flex-wrap items-center gap-2 md:justify-end">
        <Button variant="outline" size="sm" disabled={!downloadSupported || downloading} onClick={onDownload}>
          <Download className="h-4 w-4" />
          {downloading ? t("下载中") : t("下载/更新")}
        </Button>
        <Button variant="outline" size="icon-sm" title={t("复制路径")} disabled={!path} onClick={() => copyResourcePath(path, t)}>
          <Copy className="h-4 w-4" />
        </Button>
        <Button variant="outline" size="icon-sm" title={t("打开所在位置")} disabled={!canOpen || !path} onClick={() => openResourcePath(path, t)}>
          <FolderOpen className="h-4 w-4" />
        </Button>
        <Button variant="outline" size="icon-sm" title={t("打开资源来源")} disabled={!sourceUrl} onClick={() => openSourceUrl(sourceUrl, t)}>
          <ExternalLink className="h-4 w-4" />
        </Button>
      </div>
    </div>
  );
}

async function copyResourcePath(path: string | undefined, t: (text: string) => string) {
  if (!path) return;
  try {
    await navigator.clipboard.writeText(path);
    toast.success(t("路径已复制"));
  } catch (error) {
    toast.error(formatUiError(error) || t("复制路径失败"));
  }
}

async function openResourcePath(path: string | undefined, t: (text: string) => string) {
  if (!path) return;
  try {
    await revealItemInDir(path);
  } catch (error) {
    toast.error(formatUiError(error) || t("打开所在位置失败"));
  }
}

async function openSourceUrl(url: string | undefined, t: (text: string) => string) {
  if (!url) return;
  try {
    await openUrl(url);
  } catch (error) {
    toast.error(formatUiError(error) || t("打开资源来源失败"));
  }
}

function formatUiError(error: unknown) {
  if (error instanceof Error) return error.message;
  return String(error);
}
