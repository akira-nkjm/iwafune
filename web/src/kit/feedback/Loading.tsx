import type { ReactNode } from "react";
import { useT } from "@/i18n";
import { cn } from "@/lib/cn";
import { Skeleton } from "@/ui/skeleton";
import { Panel } from "../panel/Panel";

/** スケルトンの行の幅（決まった順に繰り返す。毎回同じ形になり、行ごとに長さが違って見える） */
const WIDTHS = ["w-3/4", "w-1/2", "w-5/6", "w-2/3", "w-2/5", "w-4/5"] as const;
const width = (i: number) => WIDTHS[i % WIDTHS.length];

/** 行と列の key（並べ替えない固定の並びなので、番号から作った名前を使う） */
const KEYS = Array.from({ length: 40 }, (_, i) => `k${i}`);
const keys = (n: number) => KEYS.slice(0, Math.min(n, KEYS.length));

/**
 * 読み込み中の枠。見た目はスケルトン、画面読み上げには「読み込み中…」を伝える。
 * 少し待ってから現れる（index.css の `loading-appear`）ので、速い読み込みではちらつかない。
 * スケルトンを包む要素は箱を作らない（`contents`）ので、`className` の flex・gap はスケルトンにそのまま効く
 * （間を空けるのは `flex flex-col gap-4`。`space-y-*` は子の子に届かないので効かない）。
 */
export function Loading({ children, className }: { children: ReactNode; className?: string }) {
  const t = useT();
  return (
    <div role="status" aria-busy="true" className={cn("loading-appear", className)}>
      <span className="sr-only">{t.common.loading}</span>
      <div aria-hidden="true" className="contents">
        {children}
      </div>
    </div>
  );
}

/**
 * 前の中身を出したまま読み直しているときの、上端の細い帯（動き続ける）。親は `relative` にする。
 * `Loading` と同じく少し待ってから現れ、画面読み上げには「読み込み中…」を伝える。
 */
export function Refreshing({ className }: { className?: string }) {
  const t = useT();
  return (
    <div
      role="status"
      aria-busy="true"
      className={cn(
        "loading-appear pointer-events-none absolute inset-x-0 top-0 z-10 h-0.5 overflow-hidden bg-accent-tint",
        className,
      )}
    >
      <span className="sr-only">{t.common.loading}</span>
      <div aria-hidden="true" className="refresh-bar h-full w-1/3 rounded-full bg-accent" />
    </div>
  );
}

/** 形を決めたスケルトンの塊（KPI の数・図・チップなど。大きさと角丸は `className`） */
export function SkeletonBlock({ className }: { className?: string }) {
  return <Skeleton className={className} />;
}

/** 小さなチップの並び（単位の候補など） */
export function SkeletonChips({ count = 4 }: { count?: number }) {
  return (
    <div className="flex flex-wrap gap-1">
      {keys(count).map((k) => (
        <Skeleton key={k} className="h-6 w-12 rounded-chip" />
      ))}
    </div>
  );
}

/** 文の行 */
export function SkeletonLines({ lines = 3, className }: { lines?: number; className?: string }) {
  return (
    <div className={cn("space-y-2.5", className)}>
      {keys(lines).map((k, i) => (
        <Skeleton key={k} className={cn("h-3", width(i))} />
      ))}
    </div>
  );
}

/** 表（見出しの行と本体の行） */
export function SkeletonTable({
  rows = 8,
  cols = 5,
  className,
}: {
  rows?: number;
  cols?: number;
  className?: string;
}) {
  const grid = { gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))` };
  return (
    <div className={cn("w-full", className)}>
      <div className="grid h-8 items-center gap-4 border-line border-b px-3" style={grid}>
        {keys(cols).map((k) => (
          <Skeleton key={k} className="h-2.5 w-1/2" />
        ))}
      </div>
      {keys(rows).map((rk, r) => (
        <div
          key={rk}
          className="grid h-9 items-center gap-4 border-line-soft border-b px-3"
          style={grid}
        >
          {keys(cols).map((ck, c) => (
            <Skeleton key={ck} className={cn("h-3", width(r + c))} />
          ))}
        </div>
      ))}
    </div>
  );
}

/** パネル（見出しと本文の行） */
export function SkeletonPanel({ lines = 4, className }: { lines?: number; className?: string }) {
  return (
    <Panel className={className}>
      <Skeleton className="mb-4 h-4 w-40" />
      <SkeletonLines lines={lines} />
    </Panel>
  );
}

/** 一覧と詳細の 2 列（部品・カタログの画面） */
export function SkeletonSplit({ rows = 12 }: { rows?: number }) {
  return (
    <div className="grid min-h-0 flex-1 grid-cols-[minmax(0,1fr)_480px] border-line border-t">
      <SkeletonTable rows={rows} cols={5} />
      <div className="space-y-6 border-line border-l p-4">
        <Skeleton className="h-5 w-48" />
        <SkeletonLines lines={5} />
        <SkeletonLines lines={4} />
      </div>
    </div>
  );
}

/** KPI のカードの並び（概要の画面） */
export function SkeletonCards({ count = 4 }: { count?: number }) {
  return (
    <div className="grid grid-cols-[repeat(auto-fill,minmax(230px,1fr))] gap-3">
      {keys(count).map((k) => (
        <Skeleton key={k} className="h-24 rounded-card" />
      ))}
    </div>
  );
}

/**
 * ページ全体の骨組み（見出し・KPI の帯・タブ・本文）。ページの本体のチャンクを読む間に、ルートがこれを出す。
 * 本文の形（表・パネル・図）はページごとに children で渡す。
 */
export function SkeletonPage({
  children,
  kpis = false,
  tabs = false,
  body = true,
}: {
  children?: ReactNode;
  /** 見出しの下の数値帯（系のページ） */
  kpis?: boolean;
  /** 見出しの下のタブ */
  tabs?: boolean;
  /** 本文を PageBody と同じ余白で包む（表のように端まで使う本文は false） */
  body?: boolean;
}) {
  return (
    <Loading className="flex min-h-0 flex-1 flex-col">
      <div className="space-y-2 px-6 pt-5 pb-3">
        <Skeleton className="h-3 w-24" />
        <Skeleton className="h-6 w-56" />
        <Skeleton className="h-3 w-96 max-w-full" />
      </div>
      {kpis && (
        <div className="px-6 pb-3">
          <Skeleton className="h-[62px] rounded-card" />
        </div>
      )}
      {tabs && (
        <div className="flex h-9 items-center gap-5 border-line border-b px-6">
          {keys(4).map((k) => (
            <Skeleton key={k} className="h-3 w-14" />
          ))}
        </div>
      )}
      {body ? <div className="space-y-4 px-6 py-4">{children}</div> : children}
    </Loading>
  );
}

/** 図（システム図のキャンバス） */
export function SkeletonCanvas() {
  return (
    <div className="relative min-h-[420px] flex-1 border-line border-t bg-panel-2/40">
      {[
        "left-[8%] top-[14%]",
        "left-[40%] top-[10%]",
        "left-[70%] top-[18%]",
        "left-[22%] top-[52%]",
        "left-[56%] top-[56%]",
      ].map((pos) => (
        <Skeleton key={pos} className={cn("absolute h-24 w-44 rounded-card", pos)} />
      ))}
    </div>
  );
}
