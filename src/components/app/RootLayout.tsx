import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { NavLink, Outlet } from "react-router-dom";
import { Toaster } from "@/components/ui/sonner";
import { cn } from "@/lib/utils";
import { logError, logInfo } from "@/shared/state/logStore";
import { useJobFeed } from "@/shared/state/useJobFeed";
import { useHiddenNavItems } from "@/shared/state/navVisibility";
import { useI18n } from "@/shared/i18n";
import { GlobalDropRouter } from "./GlobalDropRouter";
import { GlobalLogger } from "./GlobalLogger";
import { ThemeProvider } from "./ThemeProvider";
import { appNavItems } from "./nav";

type AppLogEvent = {
  level?: string;
  message?: string;
};

export function RootLayout() {
  return (
    <ThemeProvider>
      <Shell />
    </ThemeProvider>
  );
}

function Shell() {
  useBackendLogEvents();
  const { t } = useI18n();
  const hiddenNavItems = useHiddenNavItems();
  const { jobs } = useJobFeed();
  const visibleNavItems = appNavItems.filter((item) => item.id === "settings" || !hiddenNavItems.includes(item.id));

  const renderIndicator = (id: string) => {
    const job = jobs.find((item) => {
      if (id === "image-compressor") return item.kind === "imageCompression" && item.status === "running";
      if (id === "video-compressor") return item.kind === "videoTranscode" && item.status === "running";
      if (id === "batch-rename") return item.kind === "batchRename" && item.status === "running";
      return false;
    });
    if (!job) return null;
    return (
      <div className="ml-auto flex items-center gap-1.5">
        <span className="font-mono text-[10px] font-medium text-green-600 dark:text-green-400">{job.progress}%</span>
        <div className="relative flex h-2 w-2">
          <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-green-500 opacity-75" />
          <span className="relative inline-flex h-2 w-2 rounded-full bg-green-500" />
        </div>
      </div>
    );
  };

  return (
    <div className="flex h-screen w-full overflow-hidden bg-background text-foreground">
      <aside className="flex w-64 shrink-0 flex-col border-r bg-muted/30">
        <div className="flex items-center gap-3 border-b p-6">
          <div className="text-2xl">🛠️</div>
          <div className="text-lg font-bold tracking-tight">{t("本地效率工具")}</div>
        </div>

        <nav className="flex-1 space-y-2 overflow-y-auto p-4" role="navigation">
          {visibleNavItems.map((item) => {
            const Icon = item.icon;
            return (
              <NavLink
                key={item.id}
                to={item.href}
                className={({ isActive }) =>
                  cn(
                    "flex w-full items-center gap-3 rounded-xl px-4 py-3 transition-all",
                    isActive
                      ? "bg-primary text-primary-foreground shadow-sm"
                      : "text-muted-foreground hover:bg-muted hover:text-foreground",
                  )
                }
              >
                {({ isActive }) => (
                  <>
                    <Icon className="h-5 w-5 shrink-0" />
                    <div className="min-w-0 flex-1 text-left">
                      <div className="flex items-center justify-between">
                        <div className="truncate text-sm font-medium leading-tight">{t(item.label)}</div>
                        {renderIndicator(item.id)}
                      </div>
                      <div className={cn("mt-0.5 truncate text-xs", isActive ? "text-primary-foreground/80" : "text-muted-foreground/70")}>
                        {t(item.description)}
                      </div>
                    </div>
                  </>
                )}
              </NavLink>
            );
          })}
        </nav>

        <div className="border-t p-4 text-center text-xs text-muted-foreground">{t("本地效率工具 v1.0")}</div>
      </aside>

      <div className="flex min-w-0 flex-1 flex-col overflow-hidden bg-background">
        <main className="relative flex-1 overflow-y-auto">
          <div className="mx-auto w-full max-w-6xl">
            <Outlet />
          </div>
        </main>
        <GlobalLogger />
        <Toaster position="bottom-right" />
        <GlobalDropRouter />
      </div>
    </div>
  );
}

function useBackendLogEvents() {
  useEffect(() => {
    const unlistenPromise = listen<AppLogEvent>("app://log", (event) => {
      const message = event.payload?.message?.trim();
      if (!message) {
        return;
      }
      if (event.payload?.level === "error") {
        logError(message);
      } else {
        logInfo(message);
      }
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten()).catch(() => undefined);
    };
  }, []);
}
