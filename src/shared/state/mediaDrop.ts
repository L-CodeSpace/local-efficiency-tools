import { useSyncExternalStore } from "react";

export type DroppedMediaKind = "image" | "video";

export type DroppedMediaSource =
  | { type: "files"; paths: string[] }
  | { type: "folder"; path: string; previewPaths: string[] };

export type PendingMediaDrop = {
  id: string;
  kind: DroppedMediaKind;
  source: DroppedMediaSource;
};

let pendingDrop: PendingMediaDrop | null = null;
const listeners = new Set<() => void>();

function emit() {
  for (const listener of listeners) listener();
}

export function setPendingMediaDrop(nextDrop: PendingMediaDrop) {
  pendingDrop = nextDrop;
  emit();
}

export function clearPendingMediaDrop(id?: string) {
  if (id && pendingDrop?.id !== id) return;
  pendingDrop = null;
  emit();
}

export function usePendingMediaDrop(kind: DroppedMediaKind) {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => (pendingDrop?.kind === kind ? pendingDrop : null),
  );
}
