import { Terminal, Trash2 } from "lucide-react";
import { Link } from "react-router-dom";
import { Button } from "@/components/ui/button";
import { useLogs } from "@/shared/state/logStore";

export function GlobalLogger() {
  const logs = useLogs();
  const last = logs[logs.length - 1];

  return (
    <div className="border-t bg-muted/20 px-4 py-2 text-xs">
      <div className="flex items-center gap-3">
        <Terminal className="h-4 w-4 text-primary" />
        <div className="min-w-0 flex-1 truncate font-mono text-muted-foreground">
          {last ? `[${last.time}] ${last.msg}` : "暂无运行日志..."}
        </div>
        <Button variant="ghost" size="xs" asChild>
          <Link to="/system-logs">查看日志</Link>
        </Button>
        <Trash2 className="h-3.5 w-3.5 text-muted-foreground/50" />
      </div>
    </div>
  );
}
