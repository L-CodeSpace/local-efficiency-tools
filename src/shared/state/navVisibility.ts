import { useSyncExternalStore } from "react";

const storageKey = "local-efficiency-tools-hidden-nav";
let hiddenNavItems = readHidden();
const listeners = new Set<() => void>();

function readHidden() {
  try {
    const parsed = JSON.parse(localStorage.getItem(storageKey) ?? "[]");
    return Array.isArray(parsed) ? parsed.filter((item): item is string => typeof item === "string") : [];
  } catch {
    return [];
  }
}

function emit() {
  localStorage.setItem(storageKey, JSON.stringify(hiddenNavItems));
  for (const listener of listeners) listener();
}

export function useHiddenNavItems() {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => hiddenNavItems,
  );
}

export function toggleHiddenNavItem(id: string) {
  hiddenNavItems = hiddenNavItems.includes(id)
    ? hiddenNavItems.filter((item) => item !== id)
    : [...hiddenNavItems, id];
  emit();
}
