import { useNavigate } from "react-router";
import { useDesign } from "@/api/queries";
import { useT } from "@/i18n";
import { KpiCard } from "@/kit/metrics/KpiCard";
import { withNote } from "@/lib/design";
import { PageBody } from "@/shell/PageBody";
import { PageHeader } from "@/shell/PageHeader";
import { BudgetPanel } from "./BudgetPanel";
import type { OverviewCard } from "./cards";
import { DiagnosticsPanel } from "./DiagnosticsPanel";
import { useOverviewCards } from "./use-overview-cards";

/** 概要: 主要 KPI（質量余裕・電力収支・DoD・RW アンロード間隔・データ余裕・回線余裕・温度）と診断 */
export function OverviewPage() {
  const t = useT();
  const o = t.overview;
  const d = useDesign().data;
  const { cards, conditions } = useOverviewCards();
  const c = d?.project.counts;
  return (
    <>
      <PageHeader
        title={o.title}
        subtitle={
          d && c ? o.subtitle(d.project.name, c.parts, c.modes, c.nets, c.cases) : undefined
        }
      />
      <PageBody>
        <OverviewKpis cards={cards} />
        {conditions.length > 0 && (
          <section>
            <h2 className="mb-3 text-sm font-medium">{t.conditions.eyebrow}</h2>
            <OverviewKpis cards={conditions} />
          </section>
        )}
        <div className="grid gap-4 xl:grid-cols-2">
          <BudgetPanel kind="mass" />
          <BudgetPanel kind="power" />
        </div>
        <DiagnosticsPanel />
      </PageBody>
    </>
  );
}

/** API の KPI を表示し、解析区分のページへ開く。 */
function OverviewKpis({ cards }: { cards: readonly OverviewCard[] }) {
  const t = useT();
  const navigate = useNavigate();
  return (
    <div className="grid grid-cols-[repeat(auto-fill,minmax(230px,1fr))] gap-3">
      {cards.map((x) => (
        <KpiCard
          key={x.key}
          label={x.label}
          value={x.value}
          ratio={x.fill?.ratio ?? null}
          barLabel={x.fill ? withNote(x.label, t.systems.kpiFill[x.fill.basis]) : undefined}
          tone={x.tone}
          footer={x.footer}
          footerRight={x.footerRight}
          loading={x.loading}
          onClick={() => navigate(x.to)}
        />
      ))}
    </div>
  );
}
