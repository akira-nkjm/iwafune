import { useQuery } from "@tanstack/react-query";
import { q } from "@/api/queries";
import type { BudgetKind } from "@/api/types";
import { localized, useT } from "@/i18n";
import { Empty } from "@/kit/Empty";
import { Loading, SkeletonPanel } from "@/kit/Loading";
import { Panel } from "@/kit/Panel";
import { PanelHeader } from "@/kit/PanelHeader";
import { TickBar } from "@/kit/TickBar";
import { VerdictPill } from "@/kit/VerdictPill";
import { verdictTone } from "@/kit/verdict-tone";

/** バジェット 1 種類（配分と現状）。行ごとの目盛りバーは「成長余裕込み / 配分」（塗りの長さだけに使う比） */
export function BudgetPanel({ kind }: { kind: BudgetKind }) {
  const t = useT();
  const b = useQuery(q.budget(kind));
  if (b.isError)
    return (
      <Panel>
        <Empty>{t.overview.budgetError(kind, b.error.message)}</Empty>
      </Panel>
    );
  if (!b.data)
    return (
      <Loading>
        <SkeletonPanel lines={5} />
      </Loading>
    );
  const v = b.data;
  return (
    <Panel>
      <PanelHeader
        title={t.overview.budgetTitle(localized(v.label, v.label_en))}
        caption={`${v.analysis}${v.mode ? t.overview.budgetMode(v.mode) : ""}`}
        right={
          <VerdictPill
            verdict={v.verdict}
            text={v.margin ? t.overview.budgetMargin(v.margin.display) : undefined}
          />
        }
      />
      <div className="mb-3 flex items-baseline gap-2">
        <span className="num font-semibold text-[20px]">{v.total.display}</span>
        {v.limit && (
          <span className="text-ink-3 text-xs">
            {t.overview.budgetLimit(v.limit.display, v.system_margin_display.display)}
          </span>
        )}
      </div>
      <ul className="space-y-2">
        {v.rows.map((r) => (
          <li
            key={r.node}
            className="grid grid-cols-[110px_minmax(0,1fr)_auto] items-center gap-3 text-xs"
          >
            <span className="truncate font-mono" title={r.label}>
              {r.node}
            </span>
            <TickBar
              ticks={36}
              ratio={r.allocated && r.allocated.si > 0 ? r.with_growth.si / r.allocated.si : null}
              tone={verdictTone[r.verdict]}
              label={`${r.label}: ${r.with_growth.display}${r.allocated ? ` / ${r.allocated.display}` : ""}`}
            />
            <span className="num whitespace-nowrap text-right text-ink-2">
              {r.with_growth.display}
              {r.allocated && <span className="text-ink-4"> / {r.allocated.display}</span>}
            </span>
          </li>
        ))}
      </ul>
      {v.notes.length > 0 && (
        <ul className="mt-3 space-y-0.5 text-[11px] text-ink-3">
          {v.notes.map((n) => (
            <li key={n}>{t.overview.note(n)}</li>
          ))}
        </ul>
      )}
    </Panel>
  );
}
