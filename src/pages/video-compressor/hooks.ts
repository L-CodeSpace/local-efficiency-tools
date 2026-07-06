import { useEffect, useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  fileAuthorizePath,
  mediaCreatePlan,
  mediaPerformanceProfile,
  mediaPreviewInputs,
  mediaRuntimeStatus,
  mediaStartJob,
  type MediaPerformanceProfile,
  type MediaRuntimeStatus,
} from "@/api_tauri";
import { defaultOutDir, joinPath } from "@/shared/utils/path";
import { logError, logInfo, logSuccess } from "@/shared/state/logStore";
import { useJobFeed } from "@/shared/state/useJobFeed";
import { clearPendingMediaDrop, usePendingMediaDrop } from "@/shared/state/mediaDrop";
import { plannedVideoOutputPaths, videoExtensions } from "@/shared/utils/media";
import { formatError } from "@/shared/utils/error";
import {
  clampAv1SpeedForEncoder,
  selectedTargets,
  type SelectedVideoSource,
  type VideoTargets,
} from "./hooks/model";
import { useVideoProbeDetails } from "./hooks/probe";
import { useAv1Settings } from "./hooks/av1";

export function useVideoCompressorPage() {
  const [selectedSource, setSelectedSource] = useState<SelectedVideoSource | null>(null);
  const [outputDir, setOutputDir] = useState("");
  const [targets, setTargets] = useState<VideoTargets>({ webp: true, av1: true, av1_an: false, mp3: false });
  const [webpQ, setWebpQ] = useState(82);
  const [cornerRadius, setCornerRadius] = useState("");
  const [hqdnLumaSpatial, setHqdnLumaSpatial] = useState(0);
  const [hqdnChromaSpatial, setHqdnChromaSpatial] = useState(0);
  const [hqdnLumaTemporal, setHqdnLumaTemporal] = useState(0);
  const [hqdnChromaTemporal, setHqdnChromaTemporal] = useState(0);
  const [hqdnSliderMax, setHqdnSliderMax] = useState(10);
  const [maxDepth, setMaxDepth] = useState(1);
  const [folderFilesPreview, setFolderFilesPreview] = useState<string[] | null>(null);
  const [runtime, setRuntime] = useState<MediaRuntimeStatus | null>(null);
  const [runtimeLoading, setRuntimeLoading] = useState(true);
  const [performance, setPerformance] = useState<MediaPerformanceProfile | null>(null);
  const [performanceLoading, setPerformanceLoading] = useState(true);
  const [performanceError, setPerformanceError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const probe = useVideoProbeDetails();
  const { jobs, cancelJob } = useJobFeed();
  const pendingDrop = usePendingMediaDrop("video");
  const av1 = useAv1Settings(performance);
  const job = useMemo(
    () => jobs.find((item) => item.kind === "videoTranscode") ?? null,
    [jobs],
  );
  const selectedFile = useMemo(() => {
    if (!selectedSource) return "";
    if (selectedSource.type === "folder" || selectedSource.type === "file") return selectedSource.path;
    return selectedSource.paths[0] ?? "";
  }, [selectedSource]);
  const isFolder = selectedSource?.type === "folder";
  const selectedFiles = selectedSource?.type === "files" ? selectedSource.paths : null;
  const inputFiles = useMemo(() => {
    if (!selectedSource) return [];
    if (selectedSource.type === "folder") return folderFilesPreview ?? [];
    if (selectedSource.type === "files") return selectedSource.paths;
    return [selectedSource.path];
  }, [folderFilesPreview, selectedSource]);
  const outputArtifacts = useMemo(() => {
    if (!selectedSource) return [];
    const videoTargets = selectedTargets(targets);
    if (videoTargets.length === 0) return [];
    if (selectedSource.type === "folder") {
      return plannedVideoOutputPaths(
        folderFilesPreview ?? [],
        outputDir,
        videoTargets,
        selectedSource.path,
      );
    }
    const inputs = selectedSource.type === "files" ? selectedSource.paths : [selectedSource.path];
    return plannedVideoOutputPaths(inputs, outputDir, videoTargets);
  }, [folderFilesPreview, outputDir, selectedSource, targets]);

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

  const refreshPerformanceProfile = async () => {
    setPerformanceLoading(true);
    setPerformanceError(null);
    try {
      const profile = await mediaPerformanceProfile();
      setPerformance(profile);
      if (!av1.av1SettingsTouched.current) {
        av1.applyRecommendedSettings(profile);
      }
    } catch (err) {
      const message = formatError(err);
      setPerformanceError(message);
      logError(message);
    } finally {
      setPerformanceLoading(false);
    }
  };

  useEffect(() => {
    refreshRuntime();
    refreshPerformanceProfile();
  }, []);

  useEffect(() => {
    if (!pendingDrop) return;
    setError(null);
    probe.resetProbeState();
    if (pendingDrop.source.type === "folder") {
      setSelectedSource({ type: "folder", path: pendingDrop.source.path });
      setOutputDir(joinPath(pendingDrop.source.path, ".out"));
      setFolderFilesPreview(pendingDrop.source.previewPaths);
      logInfo(`已通过拖拽选择视频文件夹：${pendingDrop.source.path}`);
    } else if (pendingDrop.source.paths.length === 1) {
      const path = pendingDrop.source.paths[0];
      setSelectedSource({ type: "file", path });
      setOutputDir(defaultOutDir(path));
      setFolderFilesPreview(null);
      probe.setSelectedDetailPath(path);
      void probe.loadVideoDetails(path, true);
      logInfo(`已通过拖拽选择视频文件：${path}`);
    } else {
      setSelectedSource({ type: "files", paths: pendingDrop.source.paths });
      setOutputDir(defaultOutDir(pendingDrop.source.paths[0]));
      setFolderFilesPreview(null);
      logInfo(`已通过拖拽选择 ${pendingDrop.source.paths.length} 个视频文件`);
    }
    clearPendingMediaDrop(pendingDrop.id);
  }, [pendingDrop]);

  const toggleTarget = (target: keyof VideoTargets) => {
    setTargets((current) => ({ ...current, [target]: !current[target] }));
  };

  const selectSource = async (folder: boolean) => {
    const selected = await open({
      directory: folder,
      filters: folder ? undefined : [{ name: "视频文件", extensions: videoExtensions }],
    });
    if (!selected || typeof selected !== "string") return;
    await fileAuthorizePath({ path: selected, label: folder ? "视频文件夹" : "视频来源" });
    probe.resetProbeState();
    setSelectedSource(folder ? { type: "folder", path: selected } : { type: "file", path: selected });
    setOutputDir(folder ? joinPath(selected, ".out") : defaultOutDir(selected));
    if (folder) {
      const preview = await mediaPreviewInputs({
        request: { root: selected, kind: "videoTranscode", maxDepth },
      });
      setFolderFilesPreview(preview);
    } else {
      setFolderFilesPreview(null);
      probe.setSelectedDetailPath(selected);
      void probe.loadVideoDetails(selected, true);
    }
    logInfo(`已选择${folder ? "视频文件夹" : "视频文件"}：${selected}`);
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
        selectedSource.type === "folder"
          ? await mediaPreviewInputs({
              request: { root: selectedSource.path, kind: "videoTranscode", maxDepth },
            })
          : selectedSource.type === "files"
            ? selectedSource.paths
            : [selectedSource.path];
      if (selectedSource.type === "folder") setFolderFilesPreview(inputs);
      const videoTargets = selectedTargets(targets);
      const normalizedAv1Speed = clampAv1SpeedForEncoder(av1.av1Speed, av1.effectiveAv1Encoder);
      const nextPlan = await mediaCreatePlan({
        request: {
          kind: "videoTranscode",
          inputs,
          outputDir: outputDir.trim() || undefined,
          sourceRoot: selectedSource.type === "folder" ? selectedSource.path : undefined,
          videoTargets,
          webpQuality: webpQ,
          av1Encoder: av1.av1Encoder,
          av1Speed: normalizedAv1Speed,
          av1Crf: av1.av1Crf,
          av1Threads: av1.av1Threads,
          av1TileColumns: av1.av1TileColumns,
          av1TileRows: av1.av1TileRows,
          videoConcurrency: av1.videoConcurrency,
          cornerRadius: cornerRadius.trim() || undefined,
          maxDepth,
          hqdnLumaSpatial,
          hqdnChromaSpatial,
          hqdnLumaTemporal,
          hqdnChromaTemporal,
        },
      });
      await mediaStartJob({ planId: nextPlan.id, confirmationToken: nextPlan.confirmationToken });
      logSuccess("视频处理任务已启动");
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
    selectedFile,
    isFolder,
    selectedFiles,
    inputFiles,
    outputDir,
    setOutputDir,
    targets,
    toggleTarget,
    webpQ,
    setWebpQ,
    av1Encoder: av1.av1Encoder,
    effectiveAv1Encoder: av1.effectiveAv1Encoder,
    setAv1Encoder: av1.setAv1Encoder,
    av1Speed: av1.av1Speed,
    setAv1Speed: av1.setAv1Speed,
    av1Preset: av1.av1Preset,
    currentAv1Summary: av1.currentAv1Summary,
    av1Crf: av1.av1Crf,
    setAv1Crf: av1.setAv1Crf,
    av1Threads: av1.av1Threads,
    setAv1Threads: av1.setAv1Threads,
    av1TileColumns: av1.av1TileColumns,
    setAv1TileColumns: av1.setAv1TileColumns,
    av1TileRows: av1.av1TileRows,
    setAv1TileRows: av1.setAv1TileRows,
    videoConcurrency: av1.videoConcurrency,
    setVideoConcurrency: av1.setVideoConcurrency,
    cornerRadius,
    setCornerRadius,
    hqdnLumaSpatial,
    setHqdnLumaSpatial,
    hqdnChromaSpatial,
    setHqdnChromaSpatial,
    hqdnLumaTemporal,
    setHqdnLumaTemporal,
    hqdnChromaTemporal,
    setHqdnChromaTemporal,
    hqdnSliderMax,
    setHqdnSliderMax,
    maxDepth,
    setMaxDepth,
    folderFilesPreview,
    runtime,
    runtimeLoading,
    performance,
    performanceLoading,
    performanceError,
    busy,
    error,
    selectedDetailPath: probe.selectedDetailPath,
    selectedProbe: probe.selectedProbe,
    probeLoading: probe.probeLoading,
    probeError: probe.probeError,
    job,
    outputArtifacts,
    refreshRuntime,
    refreshPerformanceProfile,
    applyRecommendedSettings: av1.applyRecommendedSettings,
    selectSource,
    selectOutDir,
    loadVideoDetails: probe.loadVideoDetails,
    startProcessing,
    cancelJob,
  };
}
