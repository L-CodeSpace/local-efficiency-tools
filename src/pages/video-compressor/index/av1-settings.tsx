/*
 * 核心职责：展示和编辑 AV1 编码参数。
 * 业务痛点：设备识别、推荐参数和 AV1 高级参数较长，不能堆在页面入口。
 * 能力边界：只渲染 AV1 设置 UI，不处理输入选择、输出路径和任务启动。
 */

import { Cpu, Loader2, RotateCcw } from "lucide-react";
import type { VideoAv1Encoder } from "@/api_tauri";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { useI18n } from "@/shared/i18n";
import type { useVideoCompressorPage } from "../hooks";
import { SliderBlock } from "./target-controls";

type VideoPageState = ReturnType<typeof useVideoCompressorPage>;

export function Av1SettingsPanel({ page }: { page: VideoPageState }) {
  const { t } = useI18n();

  if (!page.targets.av1 && !page.targets.av1_an) return null;

  return (
    <div className="space-y-6 border-t pt-4">
      <div className="rounded-md border bg-muted/40 p-3">
        <div className="mb-3 flex items-center justify-between gap-3">
          <Label className="flex items-center gap-2 text-sm font-medium">
            <Cpu className="h-4 w-4 text-primary" />
            {t("设备识别 / 推荐参数")}
          </Label>
          <div className="flex gap-2">
            <Button variant="outline" size="sm" onClick={page.refreshPerformanceProfile} disabled={page.performanceLoading}>
              {t("刷新")}
            </Button>
            <Button variant="secondary" size="sm" onClick={() => page.applyRecommendedSettings()} disabled={!page.performance}>
              <RotateCcw className="h-4 w-4" /> {t("恢复推荐")}
            </Button>
          </div>
        </div>
        {page.performanceLoading ? (
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="h-4 w-4 animate-spin" />
            {t("正在识别设备和 FFmpeg 编码器")}...
          </div>
        ) : page.performance ? (
          <div className="space-y-3">
            <div className="grid gap-2 text-sm text-muted-foreground md:grid-cols-2">
              <div className="min-w-0">
                <span className="font-medium text-foreground">CPU</span>
                <div className="break-words">{page.performance.device.cpuName}</div>
                <div>{page.performance.device.cpuPhysicalCores} {t("核")} / {page.performance.device.cpuLogicalCores} {t("线程")}</div>
              </div>
              <div className="min-w-0">
                <span className="font-medium text-foreground">GPU / {t("内存")}</span>
                <div className="break-words">{page.performance.device.gpus.map((gpu) => gpu.name).join("，") || t("未识别到独立显卡")}</div>
                <div>{formatGb(page.performance.device.ramTotal)} RAM，{t("可用")} {formatGb(page.performance.device.ramAvailable)}</div>
              </div>
            </div>
            <div className="flex flex-wrap gap-2">
              {page.performance.encoders.map((encoder) => (
                <Badge key={encoder.ffmpegName} variant={encoder.available ? "default" : "outline"}>
                  {encoder.ffmpegName} · {encoder.hardware ? t("硬件") : "CPU"} · {encoder.available ? t("可用") : t("不可用")}
                </Badge>
              ))}
            </div>
            <div className="rounded border bg-background px-3 py-2 text-xs">
              <div className="font-medium text-foreground">{t("当前将使用")}：{formatCurrentAv1Summary(page, t)}</div>
              <div className="mt-1 text-muted-foreground">{page.performance.recommended.summary}</div>
              <div className="mt-1 break-all font-mono text-[11px] text-muted-foreground/80">
                {page.performance.recommended.ffmpegArgs.join(" ")}
              </div>
            </div>
          </div>
        ) : (
          <div className="text-sm text-destructive">{page.performanceError ?? t("设备识别失败")}</div>
        )}
      </div>

      <div className="grid gap-4 md:grid-cols-2">
        <div className="space-y-2">
          <Label className="text-sm font-medium">{t("AV1 编码器")}</Label>
          <Select value={page.av1Encoder} onValueChange={(value) => page.setAv1Encoder(value as VideoAv1Encoder)}>
            <SelectTrigger className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="auto">{t("自动选择")}</SelectItem>
              <SelectItem value="av1Nvenc">NVIDIA AV1 (NVENC)</SelectItem>
              <SelectItem value="libSvtAv1">SVT-AV1 (CPU)</SelectItem>
              <SelectItem value="libAomAv1">libaom-av1 (CPU)</SelectItem>
            </SelectContent>
          </Select>
        </div>
        <div className="space-y-2">
          <Label className="text-sm font-medium">{t("批量并发数")}</Label>
          <Input type="number" min={1} max={4} value={page.videoConcurrency} onChange={(event) => page.setVideoConcurrency(Number(event.target.value))} />
        </div>
        <div className="space-y-2">
          <Label className="text-sm font-medium">CPU {t("线程数")}</Label>
          <Input type="number" min={1} max={64} value={page.av1Threads} onChange={(event) => page.setAv1Threads(Number(event.target.value))} disabled={page.effectiveAv1Encoder === "av1Nvenc"} />
        </div>
        <div className="grid grid-cols-2 gap-3">
          <div className="space-y-2">
            <Label className="text-sm font-medium">Tile {t("列")}</Label>
            <Input type="number" min={0} max={6} value={page.av1TileColumns} onChange={(event) => page.setAv1TileColumns(Number(event.target.value))} disabled={page.effectiveAv1Encoder === "av1Nvenc"} />
          </div>
          <div className="space-y-2">
            <Label className="text-sm font-medium">Tile {t("行")}</Label>
            <Input type="number" min={0} max={6} value={page.av1TileRows} onChange={(event) => page.setAv1TileRows(Number(event.target.value))} disabled={page.effectiveAv1Encoder === "av1Nvenc"} />
          </div>
        </div>
      </div>

      <SliderBlock label={page.av1Preset.label} value={page.av1Speed} min={page.av1Preset.min} max={page.av1Preset.max} onChange={page.setAv1Speed} help={page.av1Preset.help} />
      <SliderBlock label="AV1 质量 / CQ (0-63)" value={page.av1Crf} min={0} max={63} onChange={page.setAv1Crf} help="数值越小画质越好，体积越大；NVENC 使用 CQ，软件编码使用 CRF。" />
    </div>
  );
}

function formatGb(bytes: number) {
  return `${(bytes / 1024 / 1024 / 1024).toFixed(1)} GB`;
}

function formatCurrentAv1Summary(page: VideoPageState, t: (text: string, vars?: Record<string, string | number>) => string) {
  const encoderLabel =
    page.effectiveAv1Encoder === "av1Nvenc"
      ? "NVIDIA AV1 (NVENC)"
      : page.effectiveAv1Encoder === "libSvtAv1"
        ? "SVT-AV1"
        : "libaom-av1";
  const preset = page.effectiveAv1Encoder === "av1Nvenc" ? `p${page.av1Speed}` : String(page.av1Speed);
  const qualityLabel = page.effectiveAv1Encoder === "av1Nvenc" ? "CQ" : "CRF";
  return t("{encoder} · {preset} · {qualityLabel} {quality} · 并发 {concurrency}", {
    encoder: encoderLabel,
    preset,
    qualityLabel,
    quality: page.av1Crf,
    concurrency: page.videoConcurrency,
  });
}
