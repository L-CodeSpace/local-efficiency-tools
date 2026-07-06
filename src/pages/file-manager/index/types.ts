/*
 * 核心职责：定义文件管理页面弹窗状态。
 * 业务痛点：弹窗联合类型被多个操作共享，留在入口会增加页面文件体量。
 * 能力边界：只承载类型，不包含 UI 或副作用。
 */

import type { FileEntry } from "@/api_tauri";

export type Modal =
  | { kind: "none" }
  | { kind: "rename"; entry: FileEntry }
  | { kind: "new_file" }
  | { kind: "new_dir" }
  | { kind: "edit"; entry: FileEntry; content: string }
  | { kind: "delete"; entry: FileEntry };
