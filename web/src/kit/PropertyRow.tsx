import type { ReactNode } from "react";
import { cn } from "@/lib/cn";

const GRID = {
  /** ラベル + 値 */
  pair: "grid-cols-[minmax(100px,30%)_minmax(0,1fr)]",
  /** ラベル + 値 + 出所 + 操作 */
  values: "grid-cols-[minmax(100px,28%)_minmax(0,1fr)_160px_28px]",
  /** ラベル + 値 + 操作 */
  action: "grid-cols-[minmax(100px,30%)_minmax(0,1fr)_28px]",
} as const;

export type PropertyLayout = keyof typeof GRID;

/**
 * ラベルと値の 1 行（詳細パネル・インスペクタ・カタログで共通）。列の幅は layout が決め、行ごとに書かない。
 * source・action は values / action のときだけ使う。
 */
export function PropertyRow({
  label,
  title,
  children,
  source,
  action,
  layout = "pair",
  indent,
  className,
}: {
  label: ReactNode;
  title?: string;
  children?: ReactNode;
  source?: ReactNode;
  action?: ReactNode;
  layout?: PropertyLayout;
  /** 状態ごとの値の子の行（ラベルを字下げし、等幅にする） */
  indent?: boolean;
  className?: string;
}) {
  return (
    <div className={cn("grid items-center gap-2 px-3 py-1 text-xs", GRID[layout], className)}>
      <span
        className={cn(
          "truncate",
          indent ? "pl-3 font-mono text-[11.5px] text-ink-3" : "text-ink-2",
        )}
        title={title}
      >
        {label}
      </span>
      <div className="min-w-0">{children}</div>
      {layout === "values" && <div className="min-w-0">{source}</div>}
      {layout !== "pair" && (action ?? <span />)}
    </div>
  );
}
