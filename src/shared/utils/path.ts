export function splitLines(value: string) {
  return value
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
}

export function basename(path: string) {
  const normalized = normalizeDisplayPath(path);
  return normalized.split(/[/\\]/).filter(Boolean).pop() ?? normalized;
}

export function dirname(path: string) {
  const normalized = normalizeDisplayPath(path);
  const sep = normalized.includes("\\") ? "\\" : "/";
  const index = normalized.lastIndexOf(sep);
  if (index <= 0) return "";
  return normalized.slice(0, index);
}

export function joinPath(root: string, name: string) {
  const normalizedRoot = normalizeDisplayPath(root);
  if (!normalizedRoot) return name;
  const sep = normalizedRoot.includes("\\") ? "\\" : "/";
  return `${normalizedRoot.replace(/[\\/]+$/, "")}${sep}${name}`;
}

export function extension(path: string) {
  const name = basename(path);
  const index = name.lastIndexOf(".");
  return index >= 0 ? name.slice(index + 1).toLowerCase() : "";
}

export function relativePath(root: string, path: string) {
  const normalizedRoot = normalizeDisplayPath(root);
  const normalizedPath = normalizeDisplayPath(path);
  const sep = normalizedRoot.includes("\\") ? "\\" : "/";
  return normalizedPath.startsWith(normalizedRoot + sep)
    ? normalizedPath.slice(normalizedRoot.length + 1)
    : normalizedPath;
}

export function defaultOutDir(path: string) {
  const root = dirname(path);
  return root ? joinPath(root, ".out") : ".out";
}

export function formatBytes(bytes: number) {
  if (bytes === 0) return "-";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(2)} MB`;
}

export function formatDate(ms?: number) {
  if (!ms) return "-";
  return new Date(ms).toLocaleString("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function normalizeDisplayPath(path: string) {
  const uncPrefix = "\\\\?\\UNC\\";
  const localPrefix = "\\\\?\\";
  if (path.startsWith(uncPrefix)) return `\\\\${path.slice(uncPrefix.length)}`;
  if (path.startsWith(localPrefix)) return path.slice(localPrefix.length);
  return path;
}
