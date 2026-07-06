/*
 * 核心职责：视频处理页面路由入口。
 * 业务痛点：路由入口需要保持稳定，但详情面板和参数控件不能继续堆在一个巨型文件里。
 * 能力边界：只负责页面装配和把 hook 状态传给子组件。
 */

import {
  ExternalLink,
  FileImage,
  FileVideo,
  FolderOpen,
  FolderOutput,
  Info,
  Loader2,
  Music,
  Play,
  Scissors,
  Settings2,
  ShieldAlert,
  Video,
} from "lucide-react";
import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";
import { ArtifactPathList } from "@/components/common/ArtifactPathList";
import { RuntimeDependencyPrompt } from "@/components/common/RuntimeDependencyPrompt";
import { TaskProgressBoard } from "@/components/common/TaskProgressBoard";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { basename, relativePath } from "@/shared/utils/path";
import { useVideoCompressorPage } from "./hooks";
import { Av1SettingsPanel } from "./index/av1-settings";
import { VideoProbePanel } from "./index/probe-panel";
import { MiniSlider, SliderBlock, TargetCard } from "./index/target-controls";

export default function VideoCompressorPage() {
  const page = useVideoCompressorPage();

  return (
    <div className="space-y-6 px-6 pb-12 animate-in fade-in duration-300">
      <div className="sticky top-0 z-10 -mx-6 flex flex-col justify-between gap-4 border-b bg-background/95 px-6 py-6 backdrop-blur supports-[backdrop-filter]:bg-background/60 md:flex-row md:items-start">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">视频处理</h1>
          <p className="mt-1 text-muted-foreground">将视频转换为动态 WebP、高压缩比 AV1 或提取音频，支持批量处理。</p>
          {page.runtime ? (
            <p className="mt-2">
              <Badge variant="outline" className="text-muted-foreground">
                FFmpeg: {page.runtime.ffmpegVersion ?? page.runtime.message}
              </Badge>
            </p>
          ) : null}
        </div>
      </div>

      <RuntimeDependencyPrompt
        dependencyName="FFmpeg"
        runtime="ffmpeg"
        ready={Boolean(page.runtime?.ready)}
        loading={page.runtimeLoading}
        message={page.runtime?.ffmpegVersion ?? page.runtime?.message ?? "正在检测 FFmpeg 运行时"}
        sourceName={page.runtime?.sourceName}
        sourceUrl={page.runtime?.sourceUrl}
        downloadSupported={page.runtime?.downloadSupported}
        refreshing={page.runtimeLoading}
        onRefresh={page.refreshRuntime}
      />

      <div className="grid grid-cols-1 gap-6 xl:grid-cols-2">
        <div className="space-y-6">
          <Card>
            <CardHeader className="pb-4">
              <CardTitle className="flex items-center gap-2 text-lg">
                <Settings2 className="h-5 w-5 text-primary" />
                输出格式配置
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-6">
              <div className="grid grid-cols-2 gap-4 sm:grid-cols-4">
                <TargetCard checked={page.targets.webp} onToggle={() => page.toggleTarget("webp")} icon={<FileImage className="mb-2 h-8 w-8 opacity-80" />} label="WebP 动画" />
                <TargetCard checked={page.targets.av1} onToggle={() => page.toggleTarget("av1")} icon={<FileVideo className="mb-2 h-8 w-8 opacity-80" />} label="AV1 视频" />
                <TargetCard checked={page.targets.av1_an} onToggle={() => page.toggleTarget("av1_an")} icon={<Scissors className="mb-2 h-8 w-8 opacity-80" />} label={<>AV1<br />(无音轨)</>} />
                <TargetCard checked={page.targets.mp3} onToggle={() => page.toggleTarget("mp3")} icon={<Music className="mb-2 h-8 w-8 opacity-80" />} label="提取音频" />
              </div>

              {page.targets.webp ? (
                <SliderBlock label="WebP 质量 (1-100)" value={page.webpQ} min={1} max={100} onChange={page.setWebpQ} />
              ) : null}

              <Av1SettingsPanel page={page} />

              {page.targets.webp || page.targets.av1 || page.targets.av1_an ? (
                <div className="space-y-4 border-t pt-4">
                  <div className="flex flex-col items-start text-left">
                    <div className="flex w-full items-center justify-between">
                      <Label className="text-sm font-medium">hqdn3d 去噪参数设置</Label>
                      <div className="flex items-center space-x-2">
                        <Label className="whitespace-nowrap text-xs text-muted-foreground">滑块最大值:</Label>
                        <Input type="number" className="h-6 w-16 px-1 text-xs" value={page.hqdnSliderMax} onChange={(event) => page.setHqdnSliderMax(Number(event.target.value) || 10)} min={1} />
                      </div>
                    </div>
                    <span className="mt-1 text-xs font-normal text-muted-foreground">
                      参数全部为 0 时不启用该滤镜。参数详情请参考：
                      <button onClick={() => openUrl("https://ffmpeg.org/ffmpeg.html")} className="ml-1 text-primary hover:underline">FFmpeg 官方文档</button>
                    </span>
                  </div>
                  <div className="grid grid-cols-2 gap-4 pt-2">
                    <MiniSlider label="空间亮度 (ls)" value={page.hqdnLumaSpatial} max={page.hqdnSliderMax} onChange={page.setHqdnLumaSpatial} help="平滑单帧画面的明暗噪点。" />
                    <MiniSlider label="空间色度 (cs)" value={page.hqdnChromaSpatial} max={page.hqdnSliderMax} onChange={page.setHqdnChromaSpatial} help="平滑单帧画面的色彩噪点。" />
                    <MiniSlider label="时间亮度 (lt)" value={page.hqdnLumaTemporal} max={page.hqdnSliderMax} onChange={page.setHqdnLumaTemporal} help="降低连续帧之间的明暗闪烁。" />
                    <MiniSlider label="时间色度 (ct)" value={page.hqdnChromaTemporal} max={page.hqdnSliderMax} onChange={page.setHqdnChromaTemporal} help="降低连续帧之间的色彩闪烁。" />
                  </div>
                </div>
              ) : null}

              <div className="space-y-3 border-t pt-4">
                <Label htmlFor="video-corner-r">圆角裁剪半径 (px)</Label>
                <Input id="video-corner-r" type="number" placeholder="无圆角 (0)" value={page.cornerRadius} onChange={(event) => page.setCornerRadius(event.target.value)} min={0} className="max-w-[200px]" />
                <p className="text-xs text-muted-foreground">仅影响 WebP 与 AV1 画面的四周圆角裁剪。</p>
              </div>
            </CardContent>
          </Card>
        </div>

        <div className="space-y-6">
          <Card>
            <CardHeader className="pb-4">
              <CardTitle className="flex items-center gap-2 text-lg">
                <FolderOutput className="h-5 w-5 text-primary" />
                输入与输出
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-5">
              <div className="space-y-3">
                <Label>选择要处理的文件或文件夹</Label>
                <div className="flex flex-col gap-3 sm:flex-row">
                  <Button variant={page.selectedFile && !page.isFolder ? "default" : "outline"} onClick={() => page.selectSource(false)} disabled={page.busy} className="w-full sm:w-auto">
                    <Video className="h-4 w-4" /> 单个视频
                  </Button>
                  <Button variant={page.selectedFile && page.isFolder ? "default" : "outline"} onClick={() => page.selectSource(true)} disabled={page.busy} className="w-full sm:w-auto">
                    <FolderOpen className="h-4 w-4" /> 视频文件夹
                  </Button>
                </div>
                {page.selectedSource ? (
                  <div className="mt-2 space-y-3">
                    {page.isFolder ? (
                      <div className="flex items-center space-x-2">
                        <Label className="text-sm font-medium">递归深度</Label>
                        <Input type="number" min={1} max={10} value={page.maxDepth} onChange={(event) => page.setMaxDepth(Number(event.target.value) || 1)} className="h-8 w-24" />
                        <span className="text-xs text-muted-foreground">1 表示只处理当前文件夹，2 包含第一层子文件夹...</span>
                      </div>
                    ) : null}
                    <div className="grid gap-3 lg:grid-cols-[minmax(0,1fr)_minmax(280px,360px)]">
                      <div className="rounded-md border bg-muted/40 p-3">
                        <div className="mb-2 flex items-center justify-between gap-3">
                          <Label className="text-sm font-medium">输入文件</Label>
                          <Badge variant="secondary">{page.inputFiles.length}</Badge>
                        </div>
                        {page.isFolder ? (
                          <div className="mb-2 flex items-center justify-between gap-2 rounded border bg-background px-2 py-1.5 font-mono text-xs">
                            <span className="min-w-0 break-all">{page.selectedFile}</span>
                            <Button variant="ghost" size="icon-xs" title="打开所在位置" onClick={() => revealItemInDir(page.selectedFile)}>
                              <ExternalLink className="h-3 w-3" />
                            </Button>
                          </div>
                        ) : null}
                        {page.inputFiles.length > 0 ? (
                          <div className="max-h-[260px] space-y-1 overflow-y-auto">
                            {page.inputFiles.map((path) => {
                              const active = page.selectedDetailPath === path;
                              return (
                                <div key={path} className={`flex items-start gap-2 rounded-md border px-2 py-2 transition-colors ${active ? "border-primary/40 bg-primary/5" : "border-transparent bg-background/70 hover:border-border"}`}>
                                  <button type="button" onClick={() => page.loadVideoDetails(path)} className="flex min-w-0 flex-1 items-start gap-2 text-left">
                                    <FileVideo className="mt-0.5 h-4 w-4 shrink-0 text-primary" />
                                    <span className="min-w-0">
                                      <span className="block break-all text-sm font-medium">{page.isFolder ? relativePath(page.selectedFile, path) : basename(path)}</span>
                                      <span className="block break-all font-mono text-[11px] text-muted-foreground">{path}</span>
                                    </span>
                                  </button>
                                  <div className="flex shrink-0 gap-1">
                                    <Button variant="ghost" size="icon-xs" title="查看详情" onClick={() => page.loadVideoDetails(path)}>
                                      {page.probeLoading && active ? <Loader2 className="h-3 w-3 animate-spin" /> : <Info className="h-3 w-3" />}
                                    </Button>
                                    <Button variant="ghost" size="icon-xs" title="打开所在位置" onClick={() => revealItemInDir(path)}>
                                      <ExternalLink className="h-3 w-3" />
                                    </Button>
                                  </div>
                                </div>
                              );
                            })}
                          </div>
                        ) : (
                          <div className="rounded-md border border-dashed bg-background px-3 py-8 text-center text-sm text-muted-foreground">
                            {page.isFolder ? "未发现支持的视频文件" : "尚未选择视频文件"}
                          </div>
                        )}
                      </div>

                      <VideoProbePanel
                        detail={page.selectedProbe}
                        selectedPath={page.selectedDetailPath}
                        loading={page.probeLoading}
                        error={page.probeError}
                      />
                    </div>
                  </div>
                ) : null}
              </div>

              <div className="space-y-3 pt-2">
                <Label>自定义输出目录 (可选)</Label>
                <div className="flex gap-2">
                  <Input value={page.outputDir || ""} placeholder="留空则与源文件同级目录" readOnly className="cursor-not-allowed bg-muted/50 font-mono text-sm" />
                  <Button variant="secondary" onClick={page.selectOutDir} disabled={page.busy}>更改</Button>
                  {page.outputDir ? (
                    <>
                      <Button variant="ghost" onClick={() => page.setOutputDir("")} disabled={page.busy}>清除</Button>
                      <Button variant="outline" onClick={() => revealItemInDir(page.outputDir)} disabled={page.busy} title="打开输出目录">
                        <ExternalLink className="h-4 w-4" />
                      </Button>
                    </>
                  ) : null}
                </div>
                {page.selectedSource ? (
                  <div className="space-y-2">
                    <div className="flex items-center justify-between gap-3">
                      <Label>输出产物</Label>
                      <span className="text-xs text-muted-foreground">预计 {page.outputArtifacts.length} 个文件</span>
                    </div>
                    <ArtifactPathList paths={page.outputArtifacts} emptyMessage="未发现预计输出产物" />
                  </div>
                ) : null}
              </div>

              {page.selectedSource ? (
                <Button size="lg" className="mt-4 h-12 w-full text-md" onClick={page.startProcessing} disabled={page.busy || !Object.values(page.targets).some(Boolean)}>
                  <Play className="h-5 w-5" /> 开始转换
                </Button>
              ) : null}
            </CardContent>
          </Card>

          {page.error ? (
            <div className="flex gap-3 rounded-lg border border-destructive/20 bg-destructive/10 p-4 text-destructive animate-in slide-in-from-top-2">
              <ShieldAlert className="h-5 w-5 shrink-0" />
              <div className="text-sm">
                <strong className="mb-1 block font-semibold">执行失败</strong>
                <p className="break-all whitespace-pre-wrap">{page.error}</p>
              </div>
            </div>
          ) : null}

          <TaskProgressBoard job={page.job} onCancel={page.cancelJob} />
        </div>
      </div>
    </div>
  );
}

