import { createContext, useContext, useLayoutEffect, useMemo, useSyncExternalStore } from "react";
import { type Seg, segmentsOf } from "./jumps";
import type { Pt } from "./layout-geometry";

/** 描いている線 1 本（経路とネット） */
interface DrawnLine {
  key: string;
  points: Pt[];
  nets: readonly string[];
}

/**
 * いま画面に描いている線の経路を、線どうしで分け合う場所。手で引いた線やカードを動かした線の経路は、その線の部品
 * （RoutedEdge）が描くときに初めて決まるので、各線が自分の経路をここに置き、ほかの線はそれを見て交差に飛び越しを
 * 入れる。登録の知らせはまとめて 1 回にする（線の数だけ描き直しが続かないように）。
 */
export class LineStore {
  private lines = new Map<string, DrawnLine>();
  private listeners = new Set<() => void>();
  private version = 0;
  private queued = false;

  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };
  snapshot = () => this.version;

  set(id: string, line: DrawnLine | null) {
    const prior = this.lines.get(id);
    if (line ? prior?.key === line.key : !prior) return;
    if (line) this.lines.set(id, line);
    else this.lines.delete(id);
    if (this.queued) return;
    this.queued = true;
    queueMicrotask(() => {
      this.queued = false;
      this.version += 1;
      for (const l of this.listeners) l();
    });
  }

  /** `id` の線とネットを共有しない、ほかの線の線分 */
  others(id: string, nets: readonly string[]): Seg[] {
    return [...this.lines]
      .filter(([other, l]) => other !== id && !l.nets.some((n) => nets.includes(n)))
      .flatMap(([, l]) => segmentsOf(l.points));
  }
}

export const LineStoreContext = createContext<LineStore | null>(null);

/**
 * この線の経路を登録し、交差に飛び越しを入れるときに避ける相手（ほかのネットの線の線分）を返す。
 * 同じネットの線どうしはつながっているので飛び越さない。
 */
export function useOtherLines(id: string, points: Pt[], nets: readonly string[]): Seg[] {
  const store = useContext(LineStoreContext);
  const key = points.map((p) => `${p.x},${p.y}`).join(" ");
  const version = useSyncExternalStore(
    store?.subscribe ?? noopSubscribe,
    store?.snapshot ?? zero,
    store?.snapshot ?? zero,
  );
  useLayoutEffect(() => {
    store?.set(id, { key, points, nets });
  });
  useLayoutEffect(() => () => store?.set(id, null), [store, id]);
  // biome-ignore lint/correctness/useExhaustiveDependencies: version は store の中身が変わった印
  return useMemo(() => store?.others(id, nets) ?? [], [store, id, nets, version]);
}

const noopSubscribe = () => () => {};
const zero = () => 0;
