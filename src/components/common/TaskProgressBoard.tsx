import { CheckCircle2, FolderOpen, X, XCircle } from "lucide-react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { type JobProgressItem, type JobSnapshot } from "@/api_tauri";
import { ArtifactPathList } from "@/components/common/ArtifactPathList";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";

const itemStatusLabel: Record<JobProgressItem["status"], string> = {
  queued: "等待",
  running: "处理中",
  succeeded: "完成",
  failed: "失败",
  cancelled: "已取消",
};

function itemStatusVariant(status: JobProgressItem["status"]) {
  if (status === "succeeded") return "default" as const;
  if (status === "failed") return "destructive" as const;
  return "outline" as const;
}

export function TaskProgressBoard({
  job,
  onCancel,
}: {
  job: JobSnapshot | null;
  onCancel?: (jobId: string) => void;
}) {
  if (!job) return null;

  const isProcessing = job.status === "queued" || job.status === "running";
  const isFinished = job.status === "succeeded";
  const isFailed = job.status === "failed";
  const isCancelled = job.status === "cancelled";
  const artifacts = job.result?.artifacts ?? [];
  const artifact = artifacts[0];
  const progressItems = job.progressItems ?? [];

  return (
    <div className="space-y-6">
      {isProcessing ? (
        <Card className="border-primary/20 shadow-md">
          <CardContent className="space-y-4 p-6">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2 font-medium">
                <div className="h-4 w-4 rounded-full border-2 border-primary border-t-transparent animate-spin" />
                正在处理...
              </div>
              <div className="flex items-center gap-4">
                <div className="font-mono text-sm text-muted-foreground">{job.progress}%</div>
                {onCancel ? (
                  <Button variant="ghost" size="sm" onClick={() => onCancel(job.id)} className="h-7 px-2 text-destructive">
                    <X className="h-4 w-4" />
                    取消
                  </Button>
                ) : null}
              </div>
            </div>
            <Progress value={job.progress} className="h-2" />
            <div className="truncate font-mono text-xs text-muted-foreground">{job.message}</div>
            {progressItems.length > 0 ? <VideoProgressItemList items={progressItems} /> : null}
          </CardContent>
        </Card>
      ) : null}

      {isCancelled ? (
        <Card className="border-orange-500/20 shadow-md">
          <CardContent className="space-y-4 p-6">
            <div className="font-medium text-orange-500">任务已取消</div>
            {progressItems.length > 0 ? <VideoProgressItemList items={progressItems} /> : null}
          </CardContent>
        </Card>
      ) : null}

      {isFailed ? (
        <Card className="border-destructive/20 shadow-md">
          <CardContent className="space-y-4 p-6">
            <div className="font-medium text-destructive">任务失败: {job.error?.message ?? job.message}</div>
            {progressItems.length > 0 ? <VideoProgressItemList items={progressItems} /> : null}
          </CardContent>
        </Card>
      ) : null}

      {isFinished ? (
        <Card className="border-green-500/20">
          <CardHeader className="border-b bg-green-500/5 pb-3 dark:bg-green-500/10">
            <CardTitle className="flex items-center justify-between text-green-600 dark:text-green-400">
              <div className="flex items-center gap-2">
                <CheckCircle2 className="h-5 w-5" />
                处理完成
              </div>
              {artifact ? (
                <Button
                  variant="outline"
                  size="sm"
                  className="border-green-500/30 text-foreground hover:bg-green-500/10"
                  onClick={() => revealItemInDir(artifact)}
                >
                  <FolderOpen className="h-4 w-4" />
                  打开产物文件夹
                </Button>
              ) : null}
            </CardTitle>
            <CardDescription className="flex gap-4 pt-2">
              <Badge variant="outline" className="border-green-500/30 bg-background text-green-600">
                成功
              </Badge>
              <Badge variant="secondary" className="font-mono">
                {job.message}
              </Badge>
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4 p-5 text-sm text-muted-foreground">
            {progressItems.length > 0 ? <VideoProgressItemList items={progressItems} /> : null}
            {artifacts.length > 0 ? (
              <div className="space-y-2">
                <div className="flex items-center justify-between gap-3">
                  <div className="font-medium text-foreground">实际产物</div>
                  <span className="text-xs">共 {artifacts.length} 个文件</span>
                </div>
                <ArtifactPathList paths={artifacts} emptyMessage="暂无实际产物" />
              </div>
            ) : null}
            <div className="flex items-center gap-2">
              <XCircle className="h-4 w-4 opacity-0" />
              Job ID: <span className="font-mono">{job.id}</span>
            </div>
          </CardContent>
        </Card>
      ) : null}
    </div>
  );
}

function VideoProgressItemList({ items }: { items: JobProgressItem[] }) {
  return (
    <div className="max-h-72 space-y-2 overflow-y-auto rounded-md border bg-muted/40 p-3">
      {items.map((item) => (
        <div key={item.id} className="space-y-2 rounded-md border bg-background p-3">
          <div className="flex items-start justify-between gap-3">
            <div className="min-w-0">
              <div className="truncate text-sm font-medium text-foreground">{item.label}</div>
              <div className="mt-1 truncate font-mono text-xs text-muted-foreground">
                {item.currentTarget ? `${item.currentTarget} · ` : ""}
                {item.message}
              </div>
            </div>
            <div className="flex shrink-0 items-center gap-2">
              <Badge variant={itemStatusVariant(item.status)}>{itemStatusLabel[item.status]}</Badge>
              <span className="w-10 text-right font-mono text-xs text-muted-foreground">{item.progress}%</span>
            </div>
          </div>
          <Progress value={item.progress} className="h-1.5" />
          <div className="flex flex-wrap items-center gap-x-4 gap-y-1 font-mono text-[11px] text-muted-foreground">
            <span>目标 {item.completedTargets}/{item.totalTargets}</span>
            <span>{frameLabel(item)}</span>
            {item.artifacts.length > 0 ? <span>产物 {item.artifacts.length}</span> : null}
          </div>
          {item.error ? <div className="whitespace-pre-wrap text-xs text-destructive">{item.error}</div> : null}
        </div>
      ))}
    </div>
  );
}

function frameLabel(item: JobProgressItem) {
  if (typeof item.totalFrames === "number" && item.totalFrames > 0) {
    return `帧 ${item.frame ?? 0}/${item.totalFrames}`;
  }
  if (typeof item.frame === "number") {
    return `帧 ${item.frame}`;
  }
  return "帧 --";
}
