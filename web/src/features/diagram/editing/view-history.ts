import { useQueryClient } from "@tanstack/react-query";
import { useCallback, useSyncExternalStore } from "react";
import { keys } from "@/api/keys";
import { useApplyOps } from "@/api/ops";
import type { DesignView, Op, ViewEdge, ViewLayout } from "@/api/types";
import { getT } from "@/i18n";

/** 系統図の配置の図（view/electrical.toml） */
export const VIEW = "electrical";
/** 覚えておく編集の数 */
const DEPTH = 100;

export type ViewOp = Extract<
  Op,
  { op: "set_view_layout" | "set_view_edge" | "unset_view_edge" | "reset_view" }
>;

/**
 * 図の配置の編集（Op）を元に戻す Op。`before` は編集する前の配置。位置を書いたノードは前の位置に（前に無ければ
 * 消し）、線の経路は前の経路に（前に無ければ消し）戻す。全体のリセットは、消した位置と経路を全部書き戻す。
 * 複数の Op は逆の順に戻す。戻す Op も普通の編集と同じ Op なので、CLI でも同じことができる。
 */
export function inverseOf(before: ViewLayout | undefined, ops: readonly ViewOp[]): Op[] {
  const nodes = before?.nodes ?? {};
  const edges: Record<string, ViewEdge> = before?.edge ?? {};
  const pos = (id: string) => {
    const n = nodes[id];
    return n ? [{ id, x: n.x, y: n.y, w: n.w ?? null, h: n.h ?? null }] : [];
  };
  const back = (op: ViewOp): Op[] => {
    switch (op.op) {
      case "set_view_layout": {
        const touched = [...op.nodes.map((n) => n.id), ...op.remove];
        return [
          {
            op: "set_view_layout",
            view: op.view,
            nodes: touched.flatMap(pos),
            remove: op.nodes.map((n) => n.id).filter((id) => !nodes[id]),
          },
        ];
      }
      case "set_view_edge":
      case "unset_view_edge": {
        const prior = edges[op.key];
        if (prior) return [{ op: "set_view_edge", view: op.view, key: op.key, edge: prior }];
        return op.op === "set_view_edge"
          ? [{ op: "unset_view_edge", view: op.view, key: op.key }]
          : [];
      }
      case "reset_view":
        return [
          {
            op: "set_view_layout",
            view: op.view,
            nodes: Object.keys(nodes).flatMap(pos),
            remove: [],
          },
          ...Object.entries(edges).map(
            ([key, edge]): Op => ({ op: "set_view_edge", view: op.view, key, edge }),
          ),
        ];
    }
  };
  return [...ops].reverse().flatMap(back);
}

/** 編集 1 回分: 説明、した Op、戻す Op */
interface Entry {
  summary: string;
  forward: ViewOp[];
  inverse: Op[];
}

/** 元に戻す・やり直すの履歴（このタブの中だけ。読み直すと消える） */
const stacks: { undo: Entry[]; redo: Entry[] } = { undo: [], redo: [] };
let version = 0;
const listeners = new Set<() => void>();
const changed = () => {
  version += 1;
  for (const l of listeners) l();
};
const subscribe = (l: () => void) => {
  listeners.add(l);
  return () => {
    listeners.delete(l);
  };
};

/**
 * 系統図の配置の編集を送り、元に戻せるように覚える。`send` は編集する前の配置から戻す Op を作って積む。
 * 新しく編集したら、やり直しの履歴は消す。送れなかった編集は履歴に入れない。
 */
export function useViewEdits() {
  const qc = useQueryClient();
  const { mutate } = useApplyOps();
  const send = useCallback(
    (ops: ViewOp[], summary: string, onError?: () => void) => {
      const before = qc.getQueryData<DesignView>(keys.design)?.views.find((v) => v.id === VIEW);
      const entry: Entry = { summary, forward: ops, inverse: inverseOf(before, ops) };
      stacks.undo = [...stacks.undo, entry].slice(-DEPTH);
      stacks.redo = [];
      changed();
      mutate(
        { ops, summary },
        {
          onError: () => {
            stacks.undo = stacks.undo.filter((e) => e !== entry);
            changed();
            onError?.();
          },
        },
      );
    },
    [qc, mutate],
  );
  /** 線の経路を書く・消す（`edge` を省くと消す）。`key` が null なら全体をリセット */
  const editRoute = useCallback(
    (key: string | null, edge?: ViewEdge, onError?: () => void) => {
      const op: ViewOp =
        key === null
          ? { op: "reset_view", view: VIEW }
          : edge
            ? { op: "set_view_edge", view: VIEW, key, edge }
            : { op: "unset_view_edge", view: VIEW, key };
      const t = getT().diagram;
      const summary = key === null ? t.resetLayout : edge ? t.editRoute : t.resetRoute;
      send([op], summary, onError);
    },
    [send],
  );
  return { send, editRoute };
}

/**
 * 元に戻す・やり直す（⌘Z・⇧⌘Z。DiagramPage）。覚えた戻す Op（やり直すときは元の Op）を普通の編集として送る。
 * 送れなかったら履歴を元に戻す。
 */
export function useViewUndo() {
  const { mutate } = useApplyOps();
  useSyncExternalStore(
    subscribe,
    () => version,
    () => version,
  );
  const step = useCallback(
    (from: "undo" | "redo") => {
      const entry = stacks[from].at(-1);
      if (!entry) return false;
      const to = from === "undo" ? "redo" : "undo";
      stacks[from] = stacks[from].slice(0, -1);
      stacks[to] = [...stacks[to], entry];
      changed();
      const t = getT().diagram;
      mutate(
        {
          ops: from === "undo" ? entry.inverse : entry.forward,
          summary: from === "undo" ? t.undoSummary(entry.summary) : t.redoSummary(entry.summary),
        },
        {
          onError: () => {
            stacks[to] = stacks[to].filter((e) => e !== entry);
            stacks[from] = [...stacks[from], entry];
            changed();
          },
        },
      );
      return true;
    },
    [mutate],
  );
  return {
    undo: () => step("undo"),
    redo: () => step("redo"),
    canUndo: stacks.undo.length > 0,
    canRedo: stacks.redo.length > 0,
  };
}
