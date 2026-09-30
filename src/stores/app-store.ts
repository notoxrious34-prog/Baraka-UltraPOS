import { useSyncExternalStore } from "react";

export interface ShellNotice {
  id: number;
  text: string;
}

interface ShellStore {
  notices: ShellNotice[];
}

let state: ShellStore = { notices: [] };
let nextId = 1;
const listeners = new Set<() => void>();

function emit(): void {
  for (const listener of listeners) {
    listener();
  }
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

function getSnapshot(): ShellStore {
  return state;
}

/** Push a shell notice (dependency-free baseline store; Zustand arrives later). */
export function pushNotice(text: string): void {
  const trimmed = text.trim();
  if (trimmed.length === 0) {
    return;
  }
  state = { notices: [...state.notices, { id: nextId, text: trimmed }] };
  nextId += 1;
  emit();
}

/** Observe shell notices from any component. */
export function useShellNotices(): ShellNotice[] {
  return useSyncExternalStore(subscribe, getSnapshot).notices;
}
