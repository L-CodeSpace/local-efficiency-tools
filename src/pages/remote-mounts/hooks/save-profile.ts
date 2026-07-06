/*
 * 核心职责：封装远程挂载配置保存流程。
 * 业务痛点：挂载路径冲突处理和 IPC 保存逻辑较长，不能挤占页面状态 hook。
 * 能力边界：只处理保存一次 profile 的交互流程，不管理页面整体状态。
 */

import { message } from "@tauri-apps/plugin-dialog";
import { toast } from "sonner";
import { mountsSaveProfile, type MountUiContext } from "@/api_tauri";
import { logError, logSuccess } from "@/shared/state/logStore";
import { formatError } from "@/shared/utils/error";
import { basename } from "@/shared/utils/path";
import type { TranslationVars } from "@/shared/i18n";
import { formToInput } from "./form";
import { mountTargetConflict } from "./format";
import type { MountFormState } from "./types";

const MANUAL_MOUNT_PATH_BUTTON = "手动修改";

type SaveProfileAttemptOptions = {
  currentForm: MountFormState;
  currentMountPointEdited: boolean;
  uiContext: MountUiContext | null;
  setDialogOpen: (open: boolean) => void;
  setError: (message: string | null) => void;
  setForm: (form: MountFormState) => void;
  setMountPointEdited: (edited: boolean) => void;
  refresh: () => Promise<void>;
  t: (text: string, vars?: TranslationVars) => string;
};

export async function saveProfileAttempt(options: SaveProfileAttemptOptions): Promise<void> {
  const {
    currentForm,
    currentMountPointEdited,
    uiContext,
    setDialogOpen,
    setError,
    setForm,
    setMountPointEdited,
    refresh,
    t,
  } = options;

  try {
    await mountsSaveProfile({ input: formToInput(currentForm, uiContext, currentMountPointEdited) });
    setDialogOpen(false);
    toast.success(t("挂载配置已保存"));
    logSuccess(`挂载配置已保存：${currentForm.name.trim()}`);
    await refresh();
  } catch (err) {
    const conflict = mountTargetConflict(err);
    if (conflict) {
      const autoButton = t("改用 {name}", { name: basename(conflict.suggested) });
      const manualButton = t(MANUAL_MOUNT_PATH_BUTTON);
      const result = await message(
        t("挂载目录已存在：\n{target}\n\n是否自动改为：\n{suggested}", {
          target: conflict.target,
          suggested: conflict.suggested,
        }),
        {
          title: t("挂载目录已存在"),
          kind: "warning",
          buttons: { ok: autoButton, cancel: manualButton },
        },
      );
      if (result === autoButton || result === "Ok") {
        const nextForm = { ...currentForm, mountPoint: conflict.suggested };
        setMountPointEdited(true);
        setForm(nextForm);
        return saveProfileAttempt({
          ...options,
          currentForm: nextForm,
          currentMountPointEdited: true,
        });
      }

      setError(t("请手动修改挂载路径后再保存。"));
      return;
    }

    const messageText = formatError(err);
    setError(messageText);
    toast.error(messageText);
    logError(messageText);
  }
}
