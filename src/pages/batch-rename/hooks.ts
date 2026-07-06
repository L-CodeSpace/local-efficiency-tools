import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  batchRenameExecute,
  batchRenamePreview,
  fileAuthorizePath,
  jobsCancel,
  type JobSnapshot,
  type RenamePreviewItem,
} from "@/api_tauri";
import { logError, logInfo, logSuccess } from "@/shared/state/logStore";
import { formatError } from "@/shared/utils/error";

export function useBatchRenamePage() {
  const [root, setRoot] = useState("");
  const [pattern, setPattern] = useState("");
  const [replacement, setReplacement] = useState("");
  const [maxDepth, setMaxDepth] = useState(1);
  const [preserveExtension, setPreserveExtension] = useState(true);
  const [autoResolveCollision, setAutoResolveCollision] = useState(true);
  const [collisionStartIndex, setCollisionStartIndex] = useState(1);
  const [planId, setPlanId] = useState("");
  const [confirmationToken, setConfirmationToken] = useState("");
  const [items, setItems] = useState<RenamePreviewItem[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [job, setJob] = useState<JobSnapshot | null>(null);

  const pickDir = async () => {
    const selected = await open({ directory: true });
    if (!selected || typeof selected !== "string") return;
    await fileAuthorizePath({ path: selected, label: "批量重命名目录" });
    setRoot(selected);
    logInfo(`已选择批量重命名目录：${selected}`);
  };

  const applyPreset = (nextPattern: string, nextReplacement: string) => {
    setPattern(nextPattern);
    setReplacement(nextReplacement);
  };

  const preview = async () => {
    if (!root) return;
    setBusy(true);
    setError(null);
    try {
      const plan = await batchRenamePreview({
        request: {
          root,
          pattern,
          replacement,
          maxDepth,
          preserveExtension,
          useRegex: true,
          autoResolveCollision,
          collisionStartIndex,
        },
      });
      setPlanId(plan.id);
      setConfirmationToken(plan.confirmationToken);
      setItems(plan.items);
      logSuccess(`已生成重命名预览：${plan.items.length} 项`);
    } catch (err) {
      const message = formatError(err);
      setError(message);
      logError(message);
    } finally {
      setBusy(false);
    }
  };

  const toggleItem = (originalPath: string) => {
    setItems((current) =>
      current.map((item) =>
        item.originalPath === originalPath ? { ...item, selected: !item.selected } : item,
      ),
    );
  };

  const toggleAll = (selected: boolean) => {
    setItems((current) =>
      current.map((item) => ({
        ...item,
        selected: selected && item.newName !== item.originalName && !item.collision,
      })),
    );
  };

  const execute = async () => {
    const selectedOriginalPaths = items.filter((item) => item.selected && !item.collision).map((item) => item.originalPath);
    if (!planId || selectedOriginalPaths.length === 0) return;
    setBusy(true);
    setError(null);
    try {
      const nextJob = await batchRenameExecute({
        request: { planId, confirmationToken, selectedOriginalPaths },
      });
      setJob(nextJob);
      logSuccess(`已启动批量重命名：${selectedOriginalPaths.length} 项`);
    } catch (err) {
      const message = formatError(err);
      setError(message);
      logError(message);
    } finally {
      setBusy(false);
    }
  };

  const cancelJob = async (jobId: string) => {
    setJob(await jobsCancel({ jobId }));
  };

  return {
    root,
    setRoot,
    pattern,
    setPattern,
    replacement,
    setReplacement,
    maxDepth,
    setMaxDepth,
    preserveExtension,
    setPreserveExtension,
    autoResolveCollision,
    setAutoResolveCollision,
    collisionStartIndex,
    setCollisionStartIndex,
    items,
    busy,
    error,
    job,
    pickDir,
    applyPreset,
    preview,
    toggleItem,
    toggleAll,
    execute,
    cancelJob,
  };
}
