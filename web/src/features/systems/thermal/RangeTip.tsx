import type { ThermalRangeMargin, Verdict } from "@/api/types";
import { useT } from "@/i18n";
import { cn } from "@/lib/cn";
import { Formatted } from "@/quantity/Formatted";
import { PENDING, Temp } from "@/quantity/Temp";

/** 温度差（余裕。K のまま） */
function Delta({ si }: { si: number }) {
  return <Formatted si={si} dimension="Temperature" unit="K" fallback={PENDING} />;
}

export const verdictText = {
  ok: "text-ink-2",
  warning: "text-warn-ink",
  violation: "text-neg-ink",
} as const satisfies Record<Verdict, string>;

/**
 * 印や棒にカーソルを置いたとき（フォーカスでも）の吹き出し。温度と、範囲ごとの余裕・判定は API の値をそのまま出す
 * （判定や余裕はここで計算しない）。
 */
export function RangeTip({
  id,
  title,
  temperature,
  ranges,
  verdict,
  className,
}: {
  id: string;
  title: string;
  /** 温度（定常は 1 つ、非定常は最低〜最高） */
  temperature: React.ReactNode;
  ranges: readonly ThermalRangeMargin[];
  verdict: Verdict;
  className?: string;
}) {
  const t = useT();
  const R = t.systems.rangeChart;
  return (
    <div
      id={id}
      role="tooltip"
      className={cn(
        "pointer-events-none absolute top-full z-20 mt-1 w-max max-w-[min(32rem,90vw)] whitespace-nowrap rounded-control border border-line bg-panel p-2 text-xs shadow-pop",
        className,
      )}
    >
      <p className="font-medium text-ink">
        {title} <span className={verdictText[verdict]}>· {t.verdict[verdict]}</span>
      </p>
      <p className="text-ink-2">{temperature}</p>
      {ranges.length === 0 ? (
        <p className="mt-1 text-ink-3">{R.noRanges}</p>
      ) : (
        <table className="mt-1 w-full text-left">
          <thead className="text-[11px] text-ink-3">
            <tr>
              <th className="font-medium" />
              <th className="pr-3 font-medium">{R.range}</th>
              <th className="pr-3 font-medium">{R.marginLow}</th>
              <th className="font-medium">{R.marginHigh}</th>
            </tr>
          </thead>
          <tbody>
            {ranges.map((r) => (
              <tr key={r.kind} className={verdictText[r.verdict]}>
                <th className="pr-2 font-medium">{R.kinds[r.kind]}</th>
                <td className="pr-3">
                  <Temp si={r.range.min} />
                  {t.systems.rangeSep}
                  <Temp si={r.range.max} />
                </td>
                <td className="pr-3">
                  <Delta si={r.margin_low} />
                </td>
                <td>
                  <Delta si={r.margin_high} />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
