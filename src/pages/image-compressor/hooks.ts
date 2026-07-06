import { useEffect, useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  fileAuthorizePath,
  mediaCreatePlan,
  mediaPreviewInputs,
  mediaRuntimeStatus,
  mediaStartJob,
  type ImageOutputFormat,
  type MediaRuntimeStatus,
} from "@/api_tauri";
import { defaultOutDir, joinPath } from "@/shared/utils/path";
import { logError, logInfo, logSuccess } from "@/shared/state/logStore";
import { formatError } from "@/shared/utils/error";
import { useJobFeed } from "@/shared/state/useJobFeed";
import { clearPendingMediaDrop, usePendingMediaDrop } from "@/shared/state/mediaDrop";
import { imageExtensions, plannedImageOutputPaths } from "@/shared/utils/media";

type SelectedImageSource =
  | { type: "files"; paths: string[] }
  | { type: "folder"; path: string };

export function useImageCompressorPage() {
  const [selectedSource, setSelectedSource] = useState<SelectedImageSource | null>(null);
  const [outputDir, setOutputDir] = useState("");
  const [format, setFormat] = useState<ImageOutputFormat>("webp");
  const [quality, setQuality] = useState(82);
  const [cornerRadius, setCornerRadius] = useState("");
  const [maxDepth, setMaxDepth] = useState(1);
  const [folderFilesPreview, setFolderFilesPreview] = useState<string[] | null>(null);
  const [runtime, setRuntime] = useState<MediaRuntimeStatus | null>(null);
  const [runtimeLoading, setRuntimeLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const { jobs, cancelJob } = useJobFeed();
  const pendingDrop = usePendingMediaDrop("image");
  const job = useMemo(
    () => jobs.find((item) => item.kind === "imageCompression") ?? null,
    [jobs],
  );
  const outputArtifacts = useMemo(() => {
    if (!selectedSource) return [];
    if (selectedSource.type === "folder") {
      return plannedImageOutputPaths(
        folderFilesPreview ?? [],
        outputDir,
        format,
        selectedSource.path,
      );
    }
    return plannedImageOutputPaths(selectedSource.paths, outputDir, format);
  }, [folderFilesPreview, format, outputDir, selectedSource]);

  const refreshRuntime = async () => {
    setRuntimeLoading(true);
    try {
      const nextRuntime = await mediaRuntimeStatus();
      setRuntime(nextRuntime);
    } catch (err) {
      const message = formatError(err);
      setError(message);
      logError(message);
    } finally {
      setRuntimeLoading(false);
    }
  };

  useEffect(() => {
    refreshRuntime();
  }, []);

  useEffect(() => {
    if (!pendingDrop) return;
    setError(null);
    if (pendingDrop.source.type === "folder") {
      setSelectedSource({ type: "folder", path: pendingDrop.source.path });
      setOutputDir(joinPath(pendingDrop.source.path, ".out"));
      setFolderFilesPreview(pendingDrop.source.previewPaths);
      logInfo(`已通过拖拽选择图片文件夹：${pendingDrop.source.path}`);
    } else {
      setSelectedSource({ type: "files", paths: pendingDrop.source.paths });
      setOutputDir(defaultOutDir(pendingDrop.source.paths[0]));
      setFolderFilesPreview(null);
      logInfo(`已通过拖拽选择 ${pendingDrop.source.paths.length} 个图片文件`);
    }
    clearPendingMediaDrop(pendingDrop.id);
  }, [pendingDrop]);

  const selectFiles = async () => {
    const selected = await open({
      multiple: true,
      filters: [{ name: "图片文件", extensions: imageExtensions }],
    });
    if (!selected) return;
    const paths = (Array.isArray(selected) ? selected : [selected]).filter((path) => {
      const ext = path.split(".").pop()?.toLowerCase();
      return !!ext && imageExtensions.includes(ext);
    });
    if (paths.length === 0) {
      setError("请选择受支持的图片格式，不支持视频或其他文件。");
      return;
    }
    await fileAuthorizePath({ path: paths[0], label: "图片来源" });
    setOutputDir(defaultOutDir(paths[0]));
    setSelectedSource({ type: "files", paths });
    setFolderFilesPreview(null);
    logInfo(`已选择 ${paths.length} 个图片文件`);
  };

  const selectFolder = async () => {
    const selected = await open({ directory: true });
    if (!selected || typeof selected !== "string") return;
    await fileAuthorizePath({ path: selected, label: "图片文件夹" });
    setOutputDir(joinPath(selected, ".out"));
    setSelectedSource({ type: "folder", path: selected });
    const preview = await mediaPreviewInputs({
      request: { root: selected, kind: "imageCompression", maxDepth },
    });
    setFolderFilesPreview(preview);
    logInfo(`已选择图片文件夹：${selected}`);
  };

  const selectOutDir = async () => {
    const selected = await open({ directory: true });
    if (!selected || typeof selected !== "string") return;
    await fileAuthorizePath({ path: selected, label: "输出目录" });
    setOutputDir(selected);
  };

  const startProcessing = async () => {
    if (!selectedSource) return;
    setBusy(true);
    setError(null);
    try {
      const inputs =
        selectedSource.type === "files"
          ? selectedSource.paths
          : await mediaPreviewInputs({
              request: { root: selectedSource.path, kind: "imageCompression", maxDepth },
            });
      if (selectedSource.type === "folder") setFolderFilesPreview(inputs);
      const nextPlan = await mediaCreatePlan({
        request: {
          kind: "imageCompression",
          inputs,
          outputDir: outputDir.trim() || undefined,
          sourceRoot: selectedSource.type === "folder" ? selectedSource.path : undefined,
          imageFormat: format,
          quality,
          cornerRadius: cornerRadius.trim() || undefined,
          maxDepth,
        },
      });
      await mediaStartJob({ planId: nextPlan.id, confirmationToken: nextPlan.confirmationToken });
      logSuccess("图片处理任务已启动");
    } catch (err) {
      const message = formatError(err);
      setError(message);
      logError(message);
    } finally {
      setBusy(false);
    }
  };

  return {
    selectedSource,
    outputDir,
    setOutputDir,
    format,
    setFormat,
    quality,
    setQuality,
    cornerRadius,
    setCornerRadius,
    maxDepth,
    setMaxDepth,
    folderFilesPreview,
    runtime,
    runtimeLoading,
    busy,
    error,
    job,
    outputArtifacts,
    refreshRuntime,
    selectFiles,
    selectFolder,
    selectOutDir,
    startProcessing,
    cancelJob,
  };
}
