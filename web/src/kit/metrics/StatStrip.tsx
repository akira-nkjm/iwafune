import type { ReactNode } from "react";
import { cn } from "@/lib/cn";
import { Loading, SkeletonBlock } from "../feedback/Loading";
import { Panel } from "../panel/Panel";

export interface StatItem {
  key: string;
  label: string;
  value: ReactNode;
  extra?: ReactNode;
  /** 値を待っている（値の代わりに `kit/feedback/Loading` のスケルトンを出す） */
  loading?: boolean;
}

/** 縦の罫線で区切った数値帯 */
export function StatStrip({ items, className }: { items: StatItem[]; className?: string }) {
  return (
    <Panel pad={false} className={cn("overflow-hidden", className)}>
      <dl
        className="grid"
        style={{ gridTemplateColumns: `repeat(${Math.max(1, items.length)}, minmax(0, 1fr))` }}
      >
        {items.map((it) => (
          <div key={it.key} className="-ml-px min-w-0 border-line border-l px-4 py-3">
            <dt className="truncate text-2xs text-ink-3">{it.label}</dt>
            <dd className="mt-1 flex flex-wrap items-center gap-1.5">
              {it.loading ? (
                <Loading>
                  <SkeletonBlock className="h-6 w-20" />
                </Loading>
              ) : (
                <span className="num truncate font-semibold text-[16px] text-ink">{it.value}</span>
              )}
              {it.extra}
            </dd>
          </div>
        ))}
      </dl>
    </Panel>
  );
}
