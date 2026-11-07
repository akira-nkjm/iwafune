import { useNavigate } from "react-router";
import { useDesign } from "@/api/queries";
import { useT } from "@/i18n";
import { KpiCard } from "@/kit/KpiCard";
import { PageBody } from "@/shell/PageBody";
import { PageHeader } from "@/shell/PageHeader";
import { BudgetPanel } from "./BudgetPanel";
import { DiagnosticsPanel } from "./DiagnosticsPanel";
import { useOverviewCards } from "./use-overview-cards";

/** 概要: 主要 KPI（質量余裕・電力収支・DoD・RW アンロード間隔・データ余裕・回線余裕・温度）と診断 */
export function OverviewPage() {
  const o = useT().overview;
  const navigate = useNavigate();
  const d = useDesign().data;
  const cards = useOverviewCards();
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
        <div className="grid grid-cols-[repeat(auto-fill,minmax(230px,1fr))] gap-3">
          {cards.map((x) => (
            <KpiCard
              key={x.label}
              label={x.label}
              value={x.value}
              ratio={x.ratio}
              tone={x.tone}
              footer={x.footer}
              footerRight={x.footerRight}
              loading={x.loading}
              onClick={() => navigate(x.to)}
            />
          ))}
        </div>
        <div className="grid gap-4 xl:grid-cols-2">
          <BudgetPanel kind="mass" />
          <BudgetPanel kind="power" />
        </div>
        <DiagnosticsPanel />
      </PageBody>
    </>
  );
}
