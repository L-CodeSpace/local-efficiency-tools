import { Loader2 } from "lucide-react";

export function PageSkeleton() {
  return (
    <div className="flex min-h-[360px] items-center justify-center text-muted-foreground">
      <Loader2 className="mr-2 h-4 w-4 animate-spin" />
      正在加载页面
    </div>
  );
}
