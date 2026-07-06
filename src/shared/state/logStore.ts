import { useSyncExternalStore } from "react";

export type AppLog = {
  time: string;
  msg: string;
};

let logs: AppLog[] = [];
const listeners = new Set<() => void>();

function emit() {
  for (const listener of listeners) listener();
}

export const logStore = {
  subscribe(listener: () => void) {
    listeners.add(listener);
    return () => listeners.delete(listener);
  },
  getSnapshot() {
    return logs;
  },
  add(msg: string) {
    logs = [...logs.slice(-199), { time: new Date().toLocaleTimeString("zh-CN"), msg }];
    emit();
  },
  clear() {
    logs = [];
    emit();
  },
};

export function useLogs() {
  return useSyncExternalStore(logStore.subscribe, logStore.getSnapshot);
}

export function logInfo(message: string) {
  logStore.add(message);
}

export function logSuccess(message: string) {
  logStore.add(`✅ ${message}`);
}

export function logError(message: string) {
  logStore.add(`❌ ${message}`);
}
