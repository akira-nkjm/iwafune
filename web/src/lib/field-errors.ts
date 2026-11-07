import { useSyncExternalStore } from "react";

/**
 * 編集が検証で弾かれたときの、パラメーターのパスごとのエラー（POST /ops の 422 の diagnostics）。
 * 入力欄はここを見て、その欄の下に理由を出す。次に同じパスの編集が通ったら消える。
 */
let errors: ReadonlyMap<string, string> = new Map();
const listeners = new Set<() => void>();

function emit() {
  for (const l of listeners) l();
}

export function setFieldErrors(list: readonly { path: string; message: string }[]): void {
  const next = new Map(errors);
  for (const e of list) next.set(e.path, e.message);
  errors = next;
  emit();
}

export function clearFieldErrors(paths: readonly string[]): void {
  if (!paths.some((p) => errors.has(p))) return;
  const next = new Map(errors);
  for (const p of paths) next.delete(p);
  errors = next;
  emit();
}

function subscribe(l: () => void) {
  listeners.add(l);
  return () => listeners.delete(l);
}

export function fieldError(path: string | null | undefined): string | null {
  return path ? (errors.get(path) ?? null) : null;
}

export function useFieldError(path: string | null | undefined): string | null {
  return useSyncExternalStore(
    subscribe,
    () => fieldError(path),
    () => null,
  );
}
