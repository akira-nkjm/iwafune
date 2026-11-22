import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router";
import { MissingList } from "@/analysis/MissingList";
import { q, useDesign } from "@/api/queries";
import type { BudgetKind, BudgetRow } from "@/api/types";
import { localized, useT } from "@/i18n";
import { Empty } from "@/kit/feedback/Empty";
import { Loading, SkeletonPanel } from "@/kit/feedback/Loading";
import { TickBar } from "@/kit/metrics/TickBar";
import { Panel } from "@/kit/panel/Panel";
import { PanelHeader } from "@/kit/panel/PanelHeader";
import { VerdictPill } from "@/kit/status/VerdictPill";
import { verdictTone } from "@/kit/status/verdict-tone";
import { paths } from "@/lib/nav";
import { MassTotals, PropellantRow } from "./BudgetPropellant";
import { BudgetUncountedNote } from "./BudgetUncounted";
import { wetTotal } from "./cards";

/**
 * バジェット 1 種類（配分と現状）。行ごとの目盛りバーは API の塗りの比（`BudgetRow.fill`。成長余裕込み / 配分）。
 * 入力が足りなければ、解析と同じく足りない入力（`missing`）を名指しする。
 * 質量は推薬の行（推進系の解析の積む推薬）と、乾燥・推薬込みの両方の合計を出す。見出しの判定は推薬込みがあればそれ
 * （打ち上げの質量。概要のカードと同じ）、無ければ乾燥。
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
  const wet = wetTotal(v.propellant);
  const head = wet ? { verdict: wet.verdict, margin: wet.margin } : v;
  return (
    <Panel>
      <PanelHeader
        title={t.overview.budgetTitle(localized(v.label, v.label_en))}
        caption={`${v.analysis}${v.mode ? t.overview.budgetMode(v.mode) : ""}`}
        right={
          head.verdict && (
            <VerdictPill
              verdict={head.verdict}
              text={head.margin ? t.overview.budgetMargin(head.margin.display) : undefined}
            />
          )
        }
      />
      <MissingList design={design.data} missing={v.missing} />
      {v.total && v.propellant && (
        <div className="mb-3 space-y-1">
          <MassTotals
            dry={v.total}
            dryVerdict={v.verdict}
            dryMargin={v.margin}
            propellant={v.propellant}
          />
          {v.limit && (
            <span className="text-ink-3 text-xs">
              {t.overview.budgetLimit(v.limit.display, v.system_margin.display)}
            </span>
          )}
          <LimitSource requirement={v.limit_requirement} />
        </div>
      )}
      {v.total && !v.propellant && (
        <div className="mb-3 flex items-baseline gap-2">
          <span className="num font-semibold text-[20px]">{v.total.display}</span>
          {v.limit && (
            <span className="text-ink-3 text-xs">
              {t.overview.budgetLimit(v.limit.display, v.system_margin.display)}
            </span>
          )}
          <LimitSource requirement={v.limit_requirement} />
        </div>
      )}
      <ul className="space-y-2">
        {v.rows.map((r) => (
          <BudgetRowItem key={r.node ?? ""} row={r} />
        ))}
        {v.propellant && <PropellantRow propellant={v.propellant} design={design.data} />}
      </ul>
      <BudgetUncountedNote uncounted={v.uncounted} />
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

/** 上限を決めた要求への案内（上限は要求だけが持つので、編集は要求の画面で行う） */
function LimitSource({ requirement }: { requirement: string | null }) {
  const t = useT();
  if (!requirement) return null;
  return (
    <Link to={paths.requirements("list")} className="text-accent-ink text-xs hover:underline">
      {t.overview.limitFrom(requirement)}
    </Link>
  );
}

/** 配分の 1 行。「その他」の行（どの Group にも属さない部品。`node` が null）の名前は i18n の文言 */
function BudgetRowItem({ row: r }: { row: BudgetRow }) {
  const t = useT().overview;
  const name = r.label ? localized(r.label, r.label_en) : (r.node ?? t.budgetOther);
  const title = r.node ?? t.budgetOtherTitle;
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
