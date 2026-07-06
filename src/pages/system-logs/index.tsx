import { useEffect, useRef } from "react";
import { Terminal, Trash2 } from "lucide-react";
import { JobFeed } from "@/components/common/JobFeed";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { ScrollArea } from "@/components/ui/scroll-area";
import { useI18n } from "@/shared/i18n";
import { logStore, useLogs } from "@/shared/state/logStore";

export default function SystemLogsPage() {
  const { t } = useI18n();
  const logs = useLogs();
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    scrollRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs]);

  return (
    <div className="flex h-full flex-col px-6 pb-6 animate-in fade-in duration-300">
      <div className="sticky top-0 z-10 -mx-6 mb-6 flex shrink-0 flex-col justify-between gap-4 border-b bg-background/95 px-6 py-6 backdrop-blur supports-[backdrop-filter]:bg-background/60 md:flex-row md:items-start">
        <div>
          <h1 className="flex items-center gap-2 text-3xl font-bold tracking-tight">
            <Terminal className="h-8 w-8 text-primary" />
            {t("系统运行日志")}
          </h1>
          <p className="mt-1 text-muted-foreground">{t("查看全局应用运行状态和历史记录")}</p>
        </div>
        <Button variant="outline" className="shrink-0 text-destructive hover:bg-destructive/10 hover:text-destructive" onClick={() => logStore.clear()} disabled={logs.length === 0}>
          <Trash2 className="h-4 w-4" />
          {t("清空日志")}
        </Button>
      </div>

      <div className="grid min-h-0 flex-1 gap-6 lg:grid-cols-[1fr_360px]">
        <Card className="flex min-h-0 flex-col overflow-hidden border-primary/10 bg-muted/10 shadow-sm">
          <ScrollArea className="h-full w-full p-4">
            <div className="font-mono text-[13px] leading-relaxed">
              {logs.length === 0 ? (
                <div className="flex h-[300px] flex-col items-center justify-center text-muted-foreground opacity-60">
                  <Terminal className="mb-4 h-12 w-12" />
                  <p>{t("暂无运行日志")}...</p>
                </div>
              ) : (
                <div className="space-y-1">
                  {logs.map((log, index) => {
                    const isError = log.msg.includes("❌") || log.msg.includes("错误");
                    const isSuccess = log.msg.includes("✅") || log.msg.includes("成功");
                    return (
                      <div key={`${log.time}-${index}`} className="group flex items-start gap-3 break-all rounded-md px-2 py-1.5 transition-colors hover:bg-background/80">
                        <span className="shrink-0 text-muted-foreground/60">[{log.time}]</span>
                        <span className={isError ? "font-medium text-destructive" : isSuccess ? "text-green-500" : "text-foreground/90"}>{log.msg}</span>
                      </div>
                    );
                  })}
                  <div ref={scrollRef} className="h-4" />
                </div>
              )}
            </div>
          </ScrollArea>
        </Card>
        <JobFeed compact />
      </div>
    </div>
  );
}
