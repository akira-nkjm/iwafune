import type { OverviewChart, Verdict } from "@/api/types";
import { localized, useT } from "@/i18n";
import { VerdictPill } from "@/kit/status/VerdictPill";
import { cn } from "@/lib/cn";

/** 塗りの色: 判定があれば判定の色、無ければアクセント */
const fillClass: Record<Verdict | "none", string> = {
  ok: "bg-pos",
  warning: "bg-warn",
  violation: "bg-neg",
  none: "bg-accent",
};

/**
 * 解析の小さな棒グラフ（1 本 = 1 行）。どの解析のどの値を出すか・塗りの長さ・上限や必要値の線の位置・表示文字列は
 * API（`AnalysisOverview.chart`）で、ここは描くだけ。
 */
export function OverviewBars({ chart }: { chart: OverviewChart }) {
  // 言語を切り替えたら描き直す（`localized` は今の言語で選ぶ）
  useT();
  return (
    <figure className="min-w-0">
      <figcaption className="mb-2 font-medium text-ink-2 text-xs">
        {localized(chart.label.label, chart.label.label_en)}
      </figcaption>
      <ul className="space-y-2">
        {chart.bars.map((b) => {
          const name = localized(b.label.label, b.label.label_en);
          const value = b.value?.display ?? "—";
          const note = b.note ? localized(b.note.label, b.note.label_en) : null;
          return (
            <li
              key={b.key}
              className="grid grid-cols-[minmax(0,9rem)_minmax(6rem,1fr)] items-center gap-x-3 gap-y-1 text-xs sm:grid-cols-[minmax(0,9rem)_minmax(6rem,1fr)_minmax(0,auto)]"
            >
              <span className="truncate text-ink-2" title={name}>
                {name}
              </span>
              <span
                role="img"
                aria-label={note ? `${name}: ${value}（${note}）` : `${name}: ${value}`}
                className="relative h-2 rounded-full bg-panel-2"
              >
                <span
                  className={cn(
                    "absolute inset-y-0 left-0 rounded-full",
                    fillClass[b.verdict ?? "none"],
                  )}
                  style={{ width: `${b.fill * 100}%` }}
                />
                {b.reference && (
                  <span
                    aria-hidden="true"
                    className="absolute -inset-y-1 w-0.5 rounded-full bg-ink"
                    style={{ left: `calc(${b.reference.at * 100}% - 1px)` }}
                  />
                )}
              </span>
              <span className="col-span-2 flex flex-wrap items-center gap-x-2 gap-y-0.5 sm:col-span-1">
                <span className="num font-medium text-ink">{value}</span>
                {b.reference && (
                  <span className="num text-ink-3">
                    / {localized(b.reference.label.label, b.reference.label.label_en)}{" "}
                    {b.reference.value.display}
                  </span>
                )}
                {note && <span className="text-ink-3">{note}</span>}
                {b.verdict && <VerdictPill verdict={b.verdict} />}
              </span>
            </li>
          );
        })}
      </ul>
    </figure>
  );
}
