/*
 * 核心职责：展示 ffprobe 视频详情面板。
 * 业务痛点：视频详情 UI 与转码页面混在一起会让页面入口膨胀。
 * 能力边界：只展示已加载的媒体详情，不触发探测请求。
 */

import type { ReactNode } from "react";
import { Loader2 } from "lucide-react";
import type { MediaProbeInfo, MediaProbeStream } from "@/api_tauri";
import { Badge } from "@/components/ui/badge";
import { Label } from "@/components/ui/label";

export function VideoProbePanel({ detail, selectedPath, loading, error }: { detail: MediaProbeInfo | null; selectedPath: string | null; loading: boolean; error: string | null }) {
  const videoStreams = detail?.streams.filter((stream) => stream.codecType === "video") ?? [];
  const audioStreams = detail?.streams.filter((stream) => stream.codecType === "audio") ?? [];
  const otherStreams = detail?.streams.filter((stream) => stream.codecType !== "video" && stream.codecType !== "audio") ?? [];

  return (
    <div className="min-h-[260px] rounded-md border bg-background p-3">
      <div className="mb-3 flex items-center justify-between gap-3">
        <Label className="text-sm font-medium">视频详情</Label>
        {detail ? <Badge variant="outline">{detail.streams.length} 个流</Badge> : null}
      </div>

      {!selectedPath ? (
        <div className="flex h-[210px] items-center justify-center rounded-md border border-dashed text-center text-sm text-muted-foreground">
          选择一个文件查看详情
        </div>
      ) : loading ? (
        <div className="flex h-[210px] items-center justify-center gap-2 text-sm text-muted-foreground">
          <Loader2 className="h-4 w-4 animate-spin" />
          正在读取媒体信息
        </div>
      ) : error ? (
        <div className="rounded-md border border-destructive/20 bg-destructive/10 p-3 text-sm text-destructive">
          <strong className="mb-1 block font-semibold">详情读取失败</strong>
          <p className="break-all whitespace-pre-wrap">{error}</p>
        </div>
      ) : detail ? (
        <div className="space-y-4">
          <div>
            <div className="break-all text-sm font-semibold">{detail.name}</div>
            <div className="mt-1 break-all font-mono text-[11px] text-muted-foreground">{detail.path}</div>
            <div className="mt-2 rounded bg-muted/60 px-2 py-1 text-xs text-muted-foreground">{detail.rawSummary}</div>
          </div>

          <DetailGrid
            rows={[
              ["容器", detail.formatLongName ?? detail.formatName],
              ["时长", formatDuration(detail.durationSeconds)],
              ["大小", formatBytes(detail.sizeBytes)],
              ["码率", formatBitrate(detail.bitrateBps)],
            ]}
          />

          <StreamSection title="视频流" streams={videoStreams} />
          <StreamSection title="音频流" streams={audioStreams} />
          <StreamSection title="其他流" streams={otherStreams} />
        </div>
      ) : (
        <div className="flex h-[210px] items-center justify-center rounded-md border border-dashed text-center text-sm text-muted-foreground">
          暂无详情
        </div>
      )}
    </div>
  );
}

function DetailGrid({ rows }: { rows: Array<[string, ReactNode]> }) {
  return (
    <div className="grid grid-cols-2 gap-2">
      {rows.map(([label, value]) => (
        <div key={label} className="rounded border bg-muted/20 p-2">
          <div className="text-[11px] text-muted-foreground">{label}</div>
          <div className="mt-1 break-all text-sm font-medium">{value ?? "--"}</div>
        </div>
      ))}
    </div>
  );
}

function StreamSection({ title, streams }: { title: string; streams: MediaProbeStream[] }) {
  if (streams.length === 0) return null;

  return (
    <div className="space-y-2">
      <div className="text-xs font-semibold text-muted-foreground">{title}</div>
      {streams.map((stream) => (
        <div key={`${stream.index}-${stream.codecType ?? "stream"}`} className="rounded border p-2 text-sm">
          <div className="mb-2 flex items-center justify-between gap-2">
            <span className="font-medium">#{stream.index} {stream.codecName ?? stream.codecType ?? "stream"}</span>
            {stream.language ? <Badge variant="secondary">{stream.language}</Badge> : null}
          </div>
          <DetailGrid rows={streamRows(stream)} />
        </div>
      ))}
    </div>
  );
}

function streamRows(stream: MediaProbeStream): Array<[string, ReactNode]> {
  if (stream.codecType === "video") {
    return [
      ["编码", stream.codecLongName ?? stream.codecName],
      ["分辨率", stream.width && stream.height ? `${stream.width} x ${stream.height}` : undefined],
      ["帧率", stream.frameRate],
      ["像素格式", stream.pixelFormat],
      ["时长", formatDuration(stream.durationSeconds)],
      ["码率", formatBitrate(stream.bitrateBps)],
    ];
  }

  if (stream.codecType === "audio") {
    return [
      ["编码", stream.codecLongName ?? stream.codecName],
      ["采样率", stream.sampleRate ? `${stream.sampleRate} Hz` : undefined],
      ["声道", stream.channels ? `${stream.channels}` : stream.channelLayout],
      ["布局", stream.channelLayout],
      ["时长", formatDuration(stream.durationSeconds)],
      ["码率", formatBitrate(stream.bitrateBps)],
    ];
  }

  return [
    ["类型", stream.codecType],
    ["编码", stream.codecLongName ?? stream.codecName],
    ["标题", stream.title],
    ["语言", stream.language],
  ];
}

function formatDuration(seconds?: number) {
  if (seconds === undefined || !Number.isFinite(seconds)) return "--";
  const total = Math.max(0, Math.round(seconds));
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const rest = total % 60;
  if (hours > 0) return `${String(hours).padStart(2, "0")}:${String(minutes).padStart(2, "0")}:${String(rest).padStart(2, "0")}`;
  return `${String(minutes).padStart(2, "0")}:${String(rest).padStart(2, "0")}`;
}

function formatBytes(bytes?: number) {
  if (bytes === undefined || !Number.isFinite(bytes)) return "--";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return unit === 0 ? `${bytes} ${units[unit]}` : `${value.toFixed(1)} ${units[unit]}`;
}

function formatBitrate(bitsPerSecond?: number) {
  if (bitsPerSecond === undefined || !Number.isFinite(bitsPerSecond)) return "--";
  if (bitsPerSecond >= 1_000_000) return `${(bitsPerSecond / 1_000_000).toFixed(2)} Mbps`;
  if (bitsPerSecond >= 1_000) return `${(bitsPerSecond / 1_000).toFixed(0)} Kbps`;
  return `${bitsPerSecond} bps`;
}

