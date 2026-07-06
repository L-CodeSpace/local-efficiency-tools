import {
  FileImage,
  Film,
  FolderTree,
  Globe2,
  HardDrive,
  ListChecks,
  Settings,
  Terminal,
} from "lucide-react";

export type AppRouteId =
  | "image-compressor"
  | "video-compressor"
  | "dns-manager"
  | "file-manager"
  | "remote-mounts"
  | "batch-rename"
  | "system-logs"
  | "settings";

export const appNavItems = [
  {
    id: "image-compressor",
    href: "/image-compressor",
    label: "图片压缩",
    description: "WebP / AVIF",
    icon: FileImage,
  },
  {
    id: "video-compressor",
    href: "/video-compressor",
    label: "视频处理",
    description: "转码 / 提取音频",
    icon: Film,
  },
  {
    id: "dns-manager",
    href: "/dns-manager",
    label: "DNS 管理",
    description: "Hosts 记录",
    icon: Globe2,
  },
  {
    id: "file-manager",
    href: "/file-manager",
    label: "文件管理",
    description: "浏览 / 编辑",
    icon: FolderTree,
  },
  {
    id: "remote-mounts",
    href: "/remote-mounts",
    label: "远程挂载",
    description: "FTP / SFTP / WebDAV",
    icon: HardDrive,
  },
  {
    id: "batch-rename",
    href: "/batch-rename",
    label: "批量重命名",
    description: "Regex 预览",
    icon: ListChecks,
  },
  {
    id: "system-logs",
    href: "/system-logs",
    label: "系统日志",
    description: "运行历史",
    icon: Terminal,
  },
  {
    id: "settings",
    href: "/settings",
    label: "设置",
    description: "主题 / 偏好",
    icon: Settings,
  },
] as const;
