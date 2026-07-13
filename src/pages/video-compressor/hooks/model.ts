/*
 * 核心职责：提供视频处理 hook 的纯类型和参数计算函数。
 * 业务痛点：页面状态 hook 不应混入 AV1 预设、目标格式和数值夹取等纯规则。
 * 能力边界：不访问 React 状态、不调用 IPC、不产生副作用。
 */

import type {
  MediaPerformanceProfile,
  VideoAv1Encoder,
  VideoTarget,
} from "@/api_tauri";

export type VideoTargets = {
  webp: boolean;
  av1: boolean;
  av1_an: boolean;
  mp3: boolean;
};

export type SelectedVideoSource =
  | { type: "file"; path: string }
  | { type: "files"; paths: string[] }
  | { type: "folder"; path: string };

export function selectedTargets(targets: VideoTargets): VideoTarget[] {
  const values: VideoTarget[] = [];
  if (targets.webp) values.push("animatedWebp");
  if (targets.av1) values.push("av1WithAudio");
  if (targets.av1_an) values.push("av1VideoOnly");
  if (targets.mp3) values.push("audioMp3");
  return values;
}

export function clampNumber(value: number, min: number, max: number) {
  if (!Number.isFinite(value)) return min;
  return Math.min(max, Math.max(min, Math.round(value)));
}

export function resolveEffectiveAv1Encoder(
  encoder: VideoAv1Encoder,
  performance: MediaPerformanceProfile | null,
): VideoAv1Encoder {
  if (encoder !== "auto") return encoder;
  const recommended = performance?.recommended.av1Encoder;
  return recommended && recommended !== "auto" ? recommended : "libAomAv1";
}

export function av1PresetMeta(encoder: VideoAv1Encoder) {
  if (encoder === "av1Nvenc") {
    return {
      label: "NVENC 预设 (p1-p7)",
      min: 1,
      max: 7,
      help: "p1 最快，p7 质量最高，推荐 p5。",
    };
  }
  if (encoder === "libSvtAv1") {
    return {
      label: "SVT-AV1 预设 (0-8)",
      min: 0,
      max: 8,
      help: "数值越大越快，压缩率越低，推荐 6。",
    };
  }
  return {
    label: "libaom cpu-used (0-8)",
    min: 0,
    max: 8,
    help: "数值越大越快，压缩率越低，推荐 6。",
  };
}

export function clampAv1SpeedForEncoder(value: number, encoder: VideoAv1Encoder) {
  const meta = av1PresetMeta(encoder);
  return clampNumber(value, meta.min, meta.max);
}
