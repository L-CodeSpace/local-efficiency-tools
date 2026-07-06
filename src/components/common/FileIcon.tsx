import {
  File,
  FileArchive,
  FileAudio,
  FileCode,
  FileImage,
  FileText,
  FileVideo,
  Folder,
  Settings,
} from "lucide-react";
import { type FileEntry } from "@/api_tauri";
import { extension } from "@/shared/utils/path";

export const textExtensions = new Set([
  "txt",
  "md",
  "json",
  "toml",
  "yaml",
  "yml",
  "xml",
  "env",
  "log",
  "sh",
  "bat",
  "cmd",
  "ps1",
  "rs",
  "ts",
  "tsx",
  "js",
  "jsx",
  "py",
  "go",
  "cpp",
  "c",
  "h",
  "java",
  "cs",
  "html",
  "css",
  "svg",
]);

export function isTextEntry(entry: FileEntry) {
  return !entry.isDir && textExtensions.has(extension(entry.name));
}

export function FileIcon({ entry, className = "h-4 w-4" }: { entry: FileEntry; className?: string }) {
  if (entry.isDir) return <Folder className={`${className} fill-blue-500/20 text-blue-500`} />;
  const ext = extension(entry.name);
  if (["jpg", "jpeg", "png", "gif", "bmp", "webp", "ico", "tiff", "tif", "heic", "heif", "avif", "svg", "cr2", "nef", "arw"].includes(ext)) {
    return <FileImage className={`${className} text-blue-500`} />;
  }
  if (["mp4", "avi", "mov", "mkv", "webm", "wmv", "m4v", "ts", "m2ts", "vob", "rmvb", "rm"].includes(ext)) {
    return <FileVideo className={`${className} text-purple-500`} />;
  }
  if (["mp3", "wav", "flac", "ogg", "aac"].includes(ext)) return <FileAudio className={`${className} text-pink-500`} />;
  if (["zip", "rar", "7z", "tar", "gz"].includes(ext)) return <FileArchive className={`${className} text-amber-500`} />;
  if (["rs", "ts", "tsx", "js", "jsx", "py", "go", "cpp", "c", "java", "cs"].includes(ext)) {
    return <FileCode className={`${className} text-indigo-500`} />;
  }
  if (["json", "toml", "yaml", "yml", "xml", "env"].includes(ext)) return <Settings className={`${className} text-slate-500`} />;
  if (["md", "txt", "log", "pdf"].includes(ext)) return <FileText className={`${className} text-slate-500`} />;
  return <File className={`${className} text-muted-foreground`} />;
}
