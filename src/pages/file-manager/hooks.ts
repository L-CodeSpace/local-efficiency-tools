import { useCallback, useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  fileAuthorizePath,
  fileExecuteOperation,
  fileGetLocations,
  fileListDir,
  fileListRoots,
  filePreviewOperation,
  fileReadText,
  type AuthorizedRoot,
  type FileEntry,
  type FileOperationRequest,
} from "@/api_tauri";
import { logError, logInfo, logSuccess } from "@/shared/state/logStore";
import { formatError } from "@/shared/utils/error";

export function useFileManagerPage() {
  const [roots, setRoots] = useState<AuthorizedRoot[]>([]);
  const [path, setPath] = useState("");
  const [entries, setEntries] = useState<FileEntry[]>([]);
  const [exeDir, setExeDir] = useState("");
  const [cwd, setCwd] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refreshRoots = useCallback(async () => {
    setRoots(await fileListRoots());
  }, []);

  const navigate = useCallback(
    async (nextPath: string) => {
      if (!nextPath) return;
      setLoading(true);
      setError(null);
      try {
        const list = await fileListDir({ path: nextPath });
        setPath(nextPath);
        setEntries(list);
        await refreshRoots();
      } catch (err) {
        const message = formatError(err);
        setError(message);
        logError(message);
      } finally {
        setLoading(false);
      }
    },
    [refreshRoots],
  );

  useEffect(() => {
    fileGetLocations()
      .then(async (locations) => {
        setExeDir(locations.executableDir);
        setCwd(locations.currentDir);
        await fileAuthorizePath({ path: locations.executableDir, label: "程序目录" }).catch(() => undefined);
        await navigate(locations.currentDir);
      })
      .catch((err) => setError(formatError(err)));
  }, [navigate]);

  const pickDir = async () => {
    const selected = await open({ directory: true });
    if (!selected || typeof selected !== "string") return;
    await fileAuthorizePath({ path: selected, label: "用户选择目录" });
    logInfo(`已选择目录：${selected}`);
    await navigate(selected);
  };

  const readText = async (entry: FileEntry) => fileReadText({ path: entry.path });

  const executeOperation = async (request: FileOperationRequest) => {
    const plan = await filePreviewOperation({ request });
    await fileExecuteOperation({ planId: plan.id, confirmationToken: plan.confirmationToken });
    logSuccess(plan.summary);
    await navigate(path);
  };

  return {
    roots,
    path,
    setPath,
    entries,
    exeDir,
    cwd,
    loading,
    error,
    navigate,
    pickDir,
    readText,
    executeOperation,
  };
}
