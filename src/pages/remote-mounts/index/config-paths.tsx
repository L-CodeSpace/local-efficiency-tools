/*
 * 核心职责：提供远程挂载配置目录入口。
 * 业务痛点：用户需要快速打开配置目录，但页面不应暴露多组路径卡片干扰主流程。
 * 能力边界：只负责打开应用固定 rclone 配置目录，不展示或编辑配置内容。
 */

import { openPath } from "@tauri-apps/plugin-opener";
import { FolderOpen } from "lucide-react";
import { toast } from "sonner";
import type { MountUiContext } from "@/api_tauri";
import { Button } from "@/components/ui/button";
import { formatError } from "@/shared/utils/error";

export function ConfigPaths({ context }: { context: MountUiContext | null }) {
  if (!context) return null;

  return (
    <div className="flex justify-end">
      <Button variant="outline" size="sm" onClick={() => openConfigDir(context.configDir)}>
        <FolderOpen className="h-4 w-4" />
        打开配置目录
      </Button>
    </div>
  );
}

async function openConfigDir(configDir: string) {
  try {
    await openPath(configDir);
  } catch (error) {
    const detail = formatError(error);
    toast.error(detail ? `打开配置目录失败：${detail}` : "打开配置目录失败");
  }
}
