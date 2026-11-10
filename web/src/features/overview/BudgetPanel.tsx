import { useQuery } from "@tanstack/react-query";
import { MissingList } from "@/analysis/MissingList";
import { q, useDesign } from "@/api/queries";
import type { BudgetKind, BudgetRow } from "@/api/types";
import { localized, useT } from "@/i18n";
import { Empty } from "@/kit/Empty";
import { Loading, SkeletonPanel } from "@/kit/Loading";
import { Panel } from "@/kit/Panel";
import { PanelHeader } from "@/kit/PanelHeader";
import { TickBar } from "@/kit/TickBar";
import { VerdictPill } from "@/kit/VerdictPill";
import { verdictTone } from "@/kit/verdict-tone";

/**
 * バジェット 1 種類（配分と現状）。行ごとの目盛りバーは API の塗りの比（`BudgetRow.fill`。成長余裕込み / 配分）。
 * 入力が足りなければ、解析と同じく足りない入力（`missing`）を名指しする。
 */
export function BudgetPanel({ kind }: { kind: BudgetKind }) {
  const t = useT();
  const b = useQuery(q.budget(kind));
  const design = useDesign();
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
          v.verdict && (
            <VerdictPill
              verdict={v.verdict}
              text={v.margin ? t.overview.budgetMargin(v.margin.display) : undefined}
            />
          )
        }
      />
      <MissingList design={design.data} missing={v.missing} />
      {v.total && (
        <div className="mb-3 flex items-baseline gap-2">
          <span className="num font-semibold text-[20px]">{v.total.display}</span>
          {v.limit && (
            <span className="text-ink-3 text-xs">
              {t.overview.budgetLimit(v.limit.display, v.system_margin.display)}
            </span>
          )}
        </div>
      )}
      <ul className="space-y-2">
        {v.rows.map((r) => (
          <BudgetRowItem key={r.node ?? ""} row={r} />
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

/** 配分の 1 行。「その他」の行（どの Group にも属さない部品。`node` が null）の名前は i18n の文言 */
function BudgetRowItem({ row: r }: { row: BudgetRow }) {
  const t = useT().overview;
  const name = r.node ?? t.budgetOther;
  const title = r.label ?? t.budgetOtherTitle;
  return (
    <li className="grid grid-cols-[110px_minmax(0,1fr)_auto] items-center gap-3 text-xs">
      <span className="truncate font-mono" title={title}>
        {name}
      </span>
      <TickBar
        ticks={36}
        ratio={r.fill?.ratio ?? null}
        tone={verdictTone[r.verdict]}
        label={`${title}: ${r.with_growth.display}${r.allocated ? ` / ${r.allocated.display}` : ""}`}
      />
      <span className="num whitespace-nowrap text-right text-ink-2">
        {r.with_growth.display}
        {r.allocated && <span className="text-ink-4"> / {r.allocated.display}</span>}
      </span>
    </li>
  );
}
