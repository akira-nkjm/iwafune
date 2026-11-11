import type { ReactNode } from "react";
import { Loading, SkeletonBlock } from "../feedback/Loading";
import { Panel } from "../panel/Panel";
import { TickBar, type TickTone } from "./TickBar";

/** KPI カード: ラベル / 右上の注記 / 大きい数字 / 目盛りバー / フッター */
export function KpiCard({
  label,
  caption,
  value,
  ratio,
  tone = "accent",
  marker,
  barLabel,
  footer,
  footerRight,
  loading,
  onClick,
}: {
  label: string;
  caption?: ReactNode;
  value: ReactNode;
  /** 0..1。null は測れるものが無い（破線） */
  ratio: number | null;
  tone?: TickTone;
  marker?: number;
  barLabel?: string;
  footer?: ReactNode;
  footerRight?: ReactNode;
  loading?: boolean;
  onClick?: () => void;
}) {
  const body = (
    <>
      <div className="flex items-baseline justify-between gap-2">
        <span className="font-medium text-ink-2 text-xs">{label}</span>
        {caption && <span className="truncate text-2xs text-ink-3">{caption}</span>}
      </div>
      {loading ? (
        <Loading>
          <SkeletonBlock className="h-7 w-32" />
        </Loading>
      ) : (
        <div className="num line-clamp-2 font-semibold text-[24px] text-ink leading-tight [overflow-wrap:anywhere]">
          {value}
        </div>
      )}
      <TickBar ratio={loading ? 0 : ratio} tone={tone} marker={marker} label={barLabel} />
      {/* 左右が収まらなければ折り返す（切り詰めて重ねない） */}
      <div className="flex flex-wrap items-baseline justify-between gap-x-3 gap-y-0.5 text-[11.5px]">
        <span className="min-w-0 text-ink-3">{footer}</span>
        <span className="num font-medium text-ink">{footerRight}</span>
      </div>
    </>
  );
  return (
    <Panel className="flex flex-col gap-2.5">
      {onClick ? (
        <button type="button" onClick={onClick} className="flex flex-col gap-2.5 text-left">
          {body}
        </button>
      ) : (
        body
      )}
    </Panel>
  );
}
