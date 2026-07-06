/*
 * 核心职责：定义全局拖拽路由辅助类型和选择规则。
 * 业务痛点：拖拽分类后的来源选择和摘要格式化不应堆在全局组件里。
 * 能力边界：只处理纯数据选择和错误格式化，不订阅窗口事件。
 */

import type { DroppedMediaKind, DroppedMediaSource } from "@/shared/state/mediaDrop";
import { basename } from "@/shared/utils/path";

export type FolderCandidate = {
  path: string;
  previewPaths: string[];
};

export type DropClassification = {
  imageFiles: string[];
  videoFiles: string[];
  imageFolders: FolderCandidate[];
  videoFolders: FolderCandidate[];
  droppedCount: number;
};

export type DropChoice = {
  classification: DropClassification;
  summary: string;
};


export function sourceForKind(kind: DroppedMediaKind, classification: DropClassification): DroppedMediaSource | null {
  const files = kind === "image" ? classification.imageFiles : classification.videoFiles;
  const folders = kind === "image" ? classification.imageFolders : classification.videoFolders;

  if (classification.droppedCount === 1 && files.length === 0 && folders.length === 1) {
    return { type: "folder", path: folders[0].path, previewPaths: folders[0].previewPaths };
  }

  const paths = unique([...files, ...folders.flatMap((folder) => folder.previewPaths)]);
  return paths.length > 0 ? { type: "files", paths } : null;
}

export function hasKind(kind: DroppedMediaKind, classification: DropClassification) {
  return countKind(kind, classification) > 0;
}

export function countKind(kind: DroppedMediaKind, classification: DropClassification) {
  const files = kind === "image" ? classification.imageFiles : classification.videoFiles;
  const folders = kind === "image" ? classification.imageFolders : classification.videoFolders;
  return files.length + folders.reduce((count, folder) => count + folder.previewPaths.length, 0);
}

function unique(paths: string[]) {
  return [...new Set(paths)];
}

export function summarizeDrop(paths: string[]) {
  if (paths.length === 1) {
    return `“${basename(paths[0])}” 中同时发现图片和视频，请选择要导入的处理页面。`;
  }
  return `已拖入 ${paths.length} 个项目，其中同时发现图片和视频，请选择要导入的处理页面。`;
}

export function formatError(error: unknown) {
  if (error && typeof error === "object" && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return String(error);
}
