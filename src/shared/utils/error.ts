export function formatError(error: unknown): string {
  if (error && typeof error === "object") {
    const record = error as { message?: unknown; detail?: unknown; code?: unknown };
    const message = typeof record.message === "string" ? record.message : "";
    const detail = typeof record.detail === "string" ? record.detail : "";
    const code = typeof record.code === "string" ? record.code : "";

    if (message && detail) return `${message}：${detail}`;
    if (message) return message;
    if (detail) return detail;
    if (code) return code;
  }

  return String(error);
}
