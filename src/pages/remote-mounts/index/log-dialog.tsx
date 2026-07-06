/*
 * 核心职责：展示单个挂载 profile 的 rclone 日志。
 * 业务痛点：挂载异常需要直接查看底层 rclone 输出，不能只显示压缩后的错误摘要。
 * 能力边界：只展示和定位日志文件，不编辑日志内容。
 */

import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";
import { FolderOpen, RefreshCw, ScrollText } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { cn } from "@/lib/utils";
import { formatError } from "@/shared/utils/error";
import { dirname, formatBytes, formatDate } from "@/shared/utils/path";
import type { MountLogsState } from "../hooks/logs";

type MountLogDialogProps = {
  logs: MountLogsState;
};

export function MountLogDialog({ logs }: MountLogDialogProps) {
  const log = logs.log;

  return (
    <Dialog open={logs.open} onOpenChange={logs.setOpen}>
      <DialogContent className="flex max-h-[90vh] flex-col sm:max-w-4xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <ScrollText className="h-5 w-5 text-primary" />
            rclone 日志
          </DialogTitle>
          <DialogDescription>{logs.profile?.name ?? "未选择挂载"}</DialogDescription>
        </DialogHeader>

        <div className="flex flex-col gap-3">
          <div className="flex flex-col gap-2 border bg-muted/30 p-3 text-xs md:flex-row md:items-center">
            <div className="min-w-0 flex-1 space-y-1">
              <div className="truncate font-mono">{log?.path ?? "-"}</div>
              <div className="flex flex-wrap gap-3 text-muted-foreground">
                <span>大小 {log?.exists ? formatBytes(log.sizeBytes) : "-"}</span>
                <span>更新 {formatDate(log?.modifiedAt)}</span>
              </div>
            </div>
            <div className="flex shrink-0 gap-2">
              <Button variant="outline" size="sm" onClick={logs.refresh} disabled={!logs.profile || logs.loading}>
                <RefreshCw className={cn("h-4 w-4", logs.loading && "animate-spin")} />
                刷新
              </Button>
              <Button variant="outline" size="sm" onClick={() => log && openLogLocation(log.path, log.exists)} disabled={!log?.path}>
                <FolderOpen className="h-4 w-4" />
                定位
              </Button>
            </div>
          </div>

          {logs.error ? (
            <div className="rounded-md border border-destructive/20 bg-destructive/10 p-3 text-sm text-destructive">
              {logs.error}
            </div>
          ) : null}

          {logs.loading ? (
            <div className="rounded-md border border-dashed p-10 text-center text-sm text-muted-foreground">
              读取日志中...
            </div>
          ) : renderLogContent(log)}
        </div>
      </DialogContent>
    </Dialog>
  );
}

function renderLogContent(log: MountLogsState["log"]) {
  if (!log) return null;
  if (!log.exists) {
    return (
      <div className="rounded-md border border-dashed p-10 text-center text-sm text-muted-foreground">
        暂无 rclone 日志。
      </div>
    );
  }
  if (!log.content.trim()) {
    return (
      <div className="rounded-md border border-dashed p-10 text-center text-sm text-muted-foreground">
        日志文件为空。
      </div>
    );
  }
  return (
    <pre className="max-h-[60vh] overflow-auto whitespace-pre-wrap break-words border bg-muted/40 p-3 font-mono text-xs leading-5 text-foreground">
      {log.content}
    </pre>
  );
}

async function openLogLocation(path: string, exists: boolean) {
  try {
    if (exists) {
      await revealItemInDir(path);
    } else {
      await openPath(dirname(path));
    }
  } catch (error) {
    const detail = formatError(error);
    toast.error(detail ? `打开日志位置失败：${detail}` : "打开日志位置失败");
  }
}

