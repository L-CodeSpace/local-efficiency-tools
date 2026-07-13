/*
 * 核心职责：管理 AV1 编码参数状态。
 * 业务痛点：AV1 推荐参数、手动覆盖和编码器速度范围不应挤占视频页面主流程。
 * 能力边界：只处理 AV1 参数状态，不调用 IPC、不启动任务。
 */

import { useEffect, useRef, useState } from "react";
import type { MediaPerformanceProfile, VideoAv1Encoder } from "@/api_tauri";
import {
  av1PresetMeta,
  clampAv1SpeedForEncoder,
  clampNumber,
  resolveEffectiveAv1Encoder,
} from "./model";

export function useAv1Settings(performance: MediaPerformanceProfile | null) {
  const [av1Encoder, setAv1Encoder] = useState<VideoAv1Encoder>("auto");
  const [av1Speed, setAv1Speed] = useState(6);
  const [av1Crf, setAv1Crf] = useState(34);
  const [av1Threads, setAv1Threads] = useState(12);
  const [av1TileColumns, setAv1TileColumns] = useState(2);
  const [av1TileRows, setAv1TileRows] = useState(1);
  const [videoConcurrency, setVideoConcurrency] = useState(1);
  const touched = useRef(false);
  const effectiveAv1Encoder = resolveEffectiveAv1Encoder(av1Encoder, performance);
  const av1Preset = av1PresetMeta(effectiveAv1Encoder);

  const applyRecommendedSettings = (profile = performance) => {
    if (!profile) return;
    touched.current = false;
    const nextEncoder = profile.recommended.av1Encoder;
    setAv1Encoder(nextEncoder);
    setAv1Speed(clampAv1SpeedForEncoder(profile.recommended.av1Speed, nextEncoder));
    setAv1Crf(profile.recommended.av1Crf);
    setAv1Threads(profile.recommended.av1Threads);
    setAv1TileColumns(profile.recommended.av1TileColumns);
    setAv1TileRows(profile.recommended.av1TileRows);
    setVideoConcurrency(profile.recommended.videoConcurrency);
  };

  const markTouched = () => {
    touched.current = true;
  };

  useEffect(() => {
    setAv1Speed((current) => clampAv1SpeedForEncoder(current, effectiveAv1Encoder));
  }, [effectiveAv1Encoder]);

  return {
    av1Encoder,
    effectiveAv1Encoder,
    av1Speed,
    av1Preset,
    av1Crf,
    av1Threads,
    av1TileColumns,
    av1TileRows,
    videoConcurrency,
    av1SettingsTouched: touched,
    applyRecommendedSettings,
    setAv1Encoder(value: VideoAv1Encoder) {
      markTouched();
      setAv1Encoder(value);
      setAv1Speed((current) =>
        clampAv1SpeedForEncoder(current, resolveEffectiveAv1Encoder(value, performance)),
      );
    },
    setAv1Speed(value: number) {
      markTouched();
      setAv1Speed(clampAv1SpeedForEncoder(value, effectiveAv1Encoder));
    },
    setAv1Crf(value: number) {
      markTouched();
      setAv1Crf(value);
    },
    setAv1Threads(value: number) {
      markTouched();
      setAv1Threads(clampNumber(value, 1, 64));
    },
    setAv1TileColumns(value: number) {
      markTouched();
      setAv1TileColumns(clampNumber(value, 0, 6));
    },
    setAv1TileRows(value: number) {
      markTouched();
      setAv1TileRows(clampNumber(value, 0, 6));
    },
    setVideoConcurrency(value: number) {
      markTouched();
      setVideoConcurrency(clampNumber(value, 1, 4));
    },
  };
}
