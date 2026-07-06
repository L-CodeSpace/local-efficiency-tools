import { X } from "lucide-react";
import { type JobSnapshot } from "@/api_tauri";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import { cn } from "@/lib/utils";
import { useJobFeed } from "@/shared/state/useJobFeed";

const statusLabel: Record<JobSnapshot["status"], string> = {
  queued: "排队中",
  running: "运行中",
  succeeded: "已完成",
  failed: "失败",
  cancelled: "已取消",
};

function statusVariant(status: JobSnapshot["status"]) {
  if (status === "succeeded") return "default" as const;
  if (status === "failed") return "destructive" as const;
  return "outline" as const;
}

export function JobFeed({ compact = false }: { compact?: boolean }) {
  const { jobs, loading, cancelJob } = useJobFeed();
  const visibleJobs = compact ? jobs.slice(0, 4) : jobs;

  return (
    <Card>
      <CardHeader className="pb-3">
        <CardTitle className="text-base">任务状态</CardTitle>
      </CardHeader>
      <CardContent className="space-y-3">
        {loading ? <div className="text-sm text-muted-foreground">正在读取任务快照...</div> : null}
        {!loading && visibleJobs.length === 0 ? (
          <div className="text-sm text-muted-foreground">暂无任务。</div>
        ) : null}
        {visibleJobs.map((job) => {
          const canCancel = job.status === "queued" || job.status === "running";
          return (
            <div key={job.id} className="rounded-md border bg-background p-3">
              <div className="flex items-start justify-between gap-3">
                <div className="min-w-0">
                  <div className="truncate text-sm font-medium">{job.title}</div>
                  <div className="mt-1 truncate text-xs text-muted-foreground">{job.message}</div>
                </div>
                <div className="flex shrink-0 items-center gap-2">
                  <Badge variant={statusVariant(job.status)}>{statusLabel[job.status]}</Badge>
                  {canCancel ? (
                    <Button
                      variant="ghost"
                      size="icon-xs"
                      title="取消任务"
                      onClick={() => cancelJob(job.id)}
                    >
                      <X className="h-3 w-3" />
                    </Button>
                  ) : null}
                </div>
              </div>
              <Progress
                value={job.progress}
                className={cn("mt-3 h-2", job.status === "failed" && "opacity-60")}
              />
              {job.error ? (
                <div className="mt-2 text-xs text-destructive">{job.error.message}</div>
              ) : null}
            </div>
          );
        })}
      </CardContent>
    </Card>
  );
}
