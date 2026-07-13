/*
 * 核心职责：全局拖拽导入路由组件。
 * 业务痛点：窗口拖拽事件、媒体分类和页面跳转混在一起会让全局组件难以审计。
 * 能力边界：只负责事件订阅、分类调用和路由跳转。
 */

import { useCallback, useEffect, useMemo, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { FileImage, FileVideo, Loader2, Upload } from "lucide-react";
import { useNavigate } from "react-router-dom";
import { toast } from "sonner";
import {
  fileAuthorizePath,
  mediaPreviewInputs,
} from "@/api_tauri";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { mediaKindForPath } from "@/shared/utils/media";
import {
  setPendingMediaDrop,
  type DroppedMediaKind,
} from "@/shared/state/mediaDrop";
import { logError, logInfo } from "@/shared/state/logStore";
import {
  countKind,
  formatError,
  hasKind,
  sourceForKind,
  summarizeDrop,
  type DropChoice,
  type DropClassification,
} from "./GlobalDropRouter/drop-helpers";
import { useI18n } from "@/shared/i18n";

const maxDropPreviewDepth = 1;

export function GlobalDropRouter() {
  const { t } = useI18n();
  const navigate = useNavigate();
  const [dragActive, setDragActive] = useState(false);
  const [busy, setBusy] = useState(false);
  const [choice, setChoice] = useState<DropChoice | null>(null);

  const routeDrop = useCallback(
    (kind: DroppedMediaKind, classification: DropClassification) => {
      const source = sourceForKind(kind, classification);
      if (!source) {
        toast.error(t("未发现可处理的图片或视频"));
        return;
      }

      setPendingMediaDrop({
        id: `${Date.now()}-${Math.random().toString(36).slice(2)}`,
        kind,
        source,
      });
      navigate(kind === "image" ? "/image-compressor" : "/video-compressor");
      logInfo(`已通过拖拽导入${kind === "image" ? "图片" : "视频"}来源`);
    },
    [navigate, t],
  );

  const handleDrop = useCallback(
    async (paths: string[]) => {
      if (paths.length === 0) return;
      setBusy(true);
      try {
        const classification = await classifyDrop(paths);
        const hasImages = hasKind("image", classification);
        const hasVideos = hasKind("video", classification);

        if (hasImages && hasVideos) {
          setChoice({
            classification,
            summary: summarizeDrop(paths),
          });
          return;
        }

        if (hasImages) {
          routeDrop("image", classification);
          return;
        }

        if (hasVideos) {
          routeDrop("video", classification);
          return;
        }

        toast.info(t("未发现可处理的图片或视频"));
        logInfo("拖拽导入未发现可处理的图片或视频");
      } catch (err) {
        const message = formatError(err);
        toast.error(message);
        logError(message);
      } finally {
        setBusy(false);
      }
    },
    [routeDrop, t],
  );

  useEffect(() => {
    const unlistenPromise = getCurrentWindow().onDragDropEvent((event) => {
      const payload = event.payload;
      if (payload.type === "enter" || payload.type === "over") {
        setDragActive(true);
        return;
      }
      if (payload.type === "leave") {
        setDragActive(false);
        return;
      }
      if (payload.type === "drop") {
        setDragActive(false);
        void handleDrop(payload.paths);
      }
    });

    return () => {
      unlistenPromise.then((unlisten) => unlisten()).catch(() => undefined);
    };
  }, [handleDrop]);

  const imageCount = useMemo(
    () => (choice ? countKind("image", choice.classification) : 0),
    [choice],
  );
  const videoCount = useMemo(
    () => (choice ? countKind("video", choice.classification) : 0),
    [choice],
  );

  return (
    <>
      {(dragActive || busy) ? (
        <div className="pointer-events-none fixed inset-0 z-40 flex items-center justify-center bg-background/80 backdrop-blur-sm">
          <div className="flex min-w-80 flex-col items-center gap-3 border bg-popover px-8 py-6 shadow-md">
            {busy ? (
              <Loader2 className="h-8 w-8 animate-spin text-primary" />
            ) : (
              <Upload className="h-8 w-8 text-primary" />
            )}
            <div className="text-base font-semibold">
              {busy ? t("正在识别拖拽内容") : t("释放以导入文件或文件夹")}
            </div>
            <div className="text-sm text-muted-foreground">
              {t("支持图片、视频和包含媒体的文件夹")}
            </div>
          </div>
        </div>
      ) : null}

      <Dialog open={Boolean(choice)} onOpenChange={(open) => !open && setChoice(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("选择处理类型")}</DialogTitle>
            <DialogDescription>
              {choice?.summary ? t(choice.summary.text, choice.summary.vars) : t("拖拽内容中同时发现图片和视频，请选择要导入的处理页面。")}
            </DialogDescription>
          </DialogHeader>

          <div className="grid gap-3 sm:grid-cols-2">
            <button
              className="flex items-center gap-3 border p-4 text-left transition-colors hover:bg-accent hover:text-accent-foreground"
              onClick={() => {
                if (!choice) return;
                routeDrop("image", choice.classification);
                setChoice(null);
              }}
            >
              <FileImage className="h-6 w-6 text-primary" />
              <span>
                <span className="block font-medium">{t("处理图片")}</span>
                <span className="text-sm text-muted-foreground">{t("{count} 个图片来源", { count: imageCount })}</span>
              </span>
            </button>
            <button
              className="flex items-center gap-3 border p-4 text-left transition-colors hover:bg-accent hover:text-accent-foreground"
              onClick={() => {
                if (!choice) return;
                routeDrop("video", choice.classification);
                setChoice(null);
              }}
            >
              <FileVideo className="h-6 w-6 text-primary" />
              <span>
                <span className="block font-medium">{t("处理视频")}</span>
                <span className="text-sm text-muted-foreground">{t("{count} 个视频来源", { count: videoCount })}</span>
              </span>
            </button>
          </div>

          <DialogFooter>
            <Button variant="outline" onClick={() => setChoice(null)}>
              {t("取消")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}

async function classifyDrop(paths: string[]): Promise<DropClassification> {
  const classification: DropClassification = {
    imageFiles: [],
    videoFiles: [],
    imageFolders: [],
    videoFolders: [],
    droppedCount: paths.length,
  };

  for (const path of paths) {
    await fileAuthorizePath({ path, label: "拖拽导入来源" });
    const kind = mediaKindForPath(path);
    if (kind === "image") {
      classification.imageFiles.push(path);
      continue;
    }
    if (kind === "video") {
      classification.videoFiles.push(path);
      continue;
    }

    const [imagePreview, videoPreview] = await Promise.all([
      previewFolder(path, "image"),
      previewFolder(path, "video"),
    ]);
    if (imagePreview.length > 0) {
      classification.imageFolders.push({ path, previewPaths: imagePreview });
    }
    if (videoPreview.length > 0) {
      classification.videoFolders.push({ path, previewPaths: videoPreview });
    }
  }

  return classification;
}

async function previewFolder(path: string, kind: DroppedMediaKind) {
  try {
    return await mediaPreviewInputs({
      request: {
        root: path,
        kind: kind === "image" ? "imageCompression" : "videoTranscode",
        maxDepth: maxDropPreviewDepth,
      },
    });
  } catch {
    // 【合理吞噬】拖拽目录预览只是为了判断导入目标，失败时按空目录降级，不影响单文件拖拽主流程。
    return [];
  }
}

