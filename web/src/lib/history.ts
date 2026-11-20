import { useSyncExternalStore } from "react";

/** このセッションで行った編集 1 回分（変更履歴と「CLI としてコピー」に使う） */
export interface ChangeEntry {
  id: number;
  at: Date;
  summary: string;
  cli: string[];
  files: string[];
  /** 編集（省略）・取り消し・やり直し */
  kind?: "undo" | "redo";
}

let entries: readonly ChangeEntry[] = [];
let seq = 0;
const listeners = new Set<() => void>();

export function recordChange(e: Omit<ChangeEntry, "id" | "at">): void {
  seq += 1;
  entries = [{ ...e, id: seq, at: new Date() }, ...entries].slice(0, 200);
  for (const l of listeners) l();
}

function subscribe(l: () => void) {
  listeners.add(l);
  return () => listeners.delete(l);
}

export function useChangeHistory(): readonly ChangeEntry[] {
  return useSyncExternalStore(
    subscribe,
    () => entries,
    () => entries,
  );
}

/** クリップボードへ。失敗しても画面は止めない */
export async function copyText(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
}
