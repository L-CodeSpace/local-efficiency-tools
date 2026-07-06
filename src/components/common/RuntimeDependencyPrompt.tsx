import { AlertTriangle, RefreshCw, Settings } from "lucide-react";
import { Link } from "react-router-dom";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

type RuntimeKind = "ffmpeg" | "rclone";

export function RuntimeDependencyPrompt({
  className,
  dependencyName,
  runtime,
  ready,
  loading,
  message,
  sourceName,
  sourceUrl,
  downloadSupported,
  refreshing,
  onRefresh,
}: {
  className?: string;
  dependencyName: string;
  runtime: RuntimeKind;
  ready: boolean;
  loading?: boolean;
  message: string;
  sourceName?: string;
  sourceUrl?: string;
  downloadSupported?: boolean;
  refreshing?: boolean;
  onRefresh: () => void | Promise<void>;
}) {
  if (loading || ready) {
    return null;
  }

  return (
    <div
      className={cn(
        "flex flex-col gap-3 border border-amber-500/30 bg-amber-500/10 p-4 text-sm text-amber-950 dark:text-amber-100 md:flex-row md:items-center",
        className,
      )}
    >
      <AlertTriangle className="h-5 w-5 shrink-0 text-amber-600 dark:text-amber-300" />
      <div className="min-w-0 flex-1">
        <div className="font-semibold">需要安装 {dependencyName} 运行时</div>
        <div className="mt-1 break-words text-amber-900/80 dark:text-amber-100/80">{message}</div>
        <div className="mt-1 break-all text-xs text-amber-900/70 dark:text-amber-100/70">
          来源：{sourceName && sourceUrl ? `${sourceName} · ${sourceUrl}` : "当前平台未配置下载源"}
        </div>
      </div>
      <div className="flex shrink-0 flex-wrap gap-2">
        <Button variant="outline" size="sm" className="bg-background" asChild>
          <Link to={`/settings?runtime=${runtime}`}>
            <Settings className="h-4 w-4" />
            {downloadSupported ? "前往设置下载" : "前往设置查看"}
          </Link>
        </Button>
        <Button variant="outline" size="sm" className="bg-background" onClick={onRefresh} disabled={refreshing}>
          <RefreshCw className={cn("h-4 w-4", refreshing && "animate-spin")} />
          重新检测
        </Button>
      </div>
    </div>
  );
}
