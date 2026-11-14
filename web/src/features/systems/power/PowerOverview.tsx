import { AnalysisTable } from "@/analysis/AnalysisTable";
import type { DesignView, PowerPageView } from "@/api/types";
import { useT } from "@/i18n";
import { Empty } from "@/kit/feedback/Empty";
import { Panel } from "@/kit/panel/Panel";
import { PanelHeader } from "@/kit/panel/PanelHeader";
import { TablePanel } from "@/kit/table/TablePanel";
import { nodeName } from "@/lib/design";
import { PowerOutline } from "./PowerOutline";
import { PowerVerdictCard } from "./PowerVerdictCard";

/**
 * 電源の概要。上段は判定ごとの全モードの最悪（サービスの専用 DTO）、下段は解析の表（1 行 = 1 解析。主な結果・判定・
 * 足りない入力の数、開くと KPI の全部・棒グラフ・判定の文・生の表。行の中身は API の `PowerAnalysisView.overview`）。
 */
export function PowerOverview({ page, design }: { page: PowerPageView; design?: DesignView }) {
  const dict = useT();
  const t = dict.systems.power;
  const table = dict.systems.analysisTable;
  const modeName = (id: string | null) => {
    const m = page.modes.find((x) => x.id === id);
    return m ? nodeName(m) : (id ?? undefined);
  };
  const items = [
    { id: "power-verdicts", label: t.verdicts },
    { id: "power-analyses", label: table.title },
  ];
  return (
    <div className="space-y-5">
      <PowerOutline items={items}>
        <section id="power-verdicts" className="scroll-mt-6">
          <Panel>
            <PanelHeader title={t.verdicts} caption={t.verdictsCaption} />
            {page.verdicts.length > 0 ? (
              <div className="grid gap-3 md:grid-cols-2">
                {page.verdicts.map((v) => (
                  <PowerVerdictCard key={v.key} item={v} modeName={modeName(v.mode)} />
                ))}
              </div>
            ) : (
              <Empty>{t.unavailable}</Empty>
            )}
          </Panel>
        </section>
        <section id="power-analyses" className="scroll-mt-6">
          <TablePanel title={table.title} caption={table.caption}>
            <AnalysisTable items={page.analyses} design={design} />
          </TablePanel>
        </section>
      </PowerOutline>
    </div>
  );
}
