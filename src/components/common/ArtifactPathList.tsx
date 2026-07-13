import { ExternalLink } from "lucide-react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { Button } from "@/components/ui/button";
import { useI18n } from "@/shared/i18n";
import { dirname } from "@/shared/utils/path";

export function ArtifactPathList({
  paths,
  emptyMessage,
}: {
  paths: string[];
  emptyMessage: string;
}) {
  const { t } = useI18n();

  return (
    <div className="rounded-md border bg-muted p-3 font-mono text-sm">
      {paths.length > 0 ? (
        <div className="max-h-[140px] space-y-1 overflow-y-auto">
          {paths.map((path, index) => (
            <div key={`${path}-${index}`} className="flex items-center justify-between gap-2">
              <span className="break-all">{path}</span>
              <Button variant="ghost" size="icon-xs" title={t("打开所在位置")} onClick={() => revealAvailableLocation(path)}>
                <ExternalLink className="h-3 w-3" />
              </Button>
            </div>
          ))}
        </div>
      ) : (
        <div className="text-xs text-muted-foreground">{emptyMessage}</div>
      )}
    </div>
  );
}

async function revealAvailableLocation(path: string) {
  let candidate = path;
  const tried = new Set<string>();
  while (candidate && !tried.has(candidate)) {
    tried.add(candidate);
    try {
      await revealItemInDir(candidate);
      return;
    } catch {
      const parent = dirname(candidate);
      if (!parent || parent === candidate) return;
      candidate = parent;
    }
  }
}
