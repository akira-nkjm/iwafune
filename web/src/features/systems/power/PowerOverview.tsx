import { ChecksList } from "@/analysis/ChecksList";
import { fieldLabel } from "@/analysis/fields";
import { MissingList } from "@/analysis/MissingList";
import { collectTables } from "@/analysis/output";
import { ResultTable } from "@/analysis/ResultTable";
import type { DesignView, PowerPageView } from "@/api/types";
import { useT } from "@/i18n";
import { analysisTitle } from "@/i18n/analysis-names";
import { Empty } from "@/kit/feedback/Empty";
import { Panel } from "@/kit/panel/Panel";
import { PanelHeader } from "@/kit/panel/PanelHeader";
import { nodeName } from "@/lib/design";
import { PowerOutline } from "./PowerOutline";
import { PowerVerdictCard } from "./PowerVerdictCard";

/** 電源の概要。表示値と判定はサービスの専用 DTO、詳細は元の解析表。 */
export function PowerOverview({ page, design }: { page: PowerPageView; design?: DesignView }) {
  const dict = useT();
  const t = dict.systems.power;
  const dataLabel = dict.systems.data;
  const modeName = (id: string | null) => {
    const m = page.modes.find((x) => x.id === id);
    return m ? nodeName(m) : (id ?? undefined);
  };
  const items = [
    { id: "power-verdicts", label: t.verdicts },
    ...page.analyses.map((a) => ({
      id: a.result.id,
      label: analysisTitle(a.entry.info, a.result.id),
    })),
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
        {page.analyses.map(({ entry, result }) => {
          const tables = result.output ? collectTables(result.output) : [];
          return (
            <section id={result.id} key={result.id} className="scroll-mt-6">
              <Panel>
                <PanelHeader title={analysisTitle(entry.info, result.id)} />
                {result.error && <p className="text-neg-ink text-sm">{result.error}</p>}
                <MissingList design={design} missing={result?.missing ?? []} />
                <details className="mt-3">
                  <summary className="cursor-pointer text-sm text-ink-3">{dataLabel}</summary>
                  <div className="mt-3 space-y-4">
                    <ChecksList checks={result?.checks ?? []} />
                    {tables.map((table) => (
                      <div key={table.path} className="overflow-auto">
                        <h3 className="mb-2 text-sm font-medium">{fieldLabel(table.key)}</h3>
                        <ResultTable
                          rows={table.rows}
                          at={table.at}
                          dims={result?.dims}
                          labels={result?.labels}
                        />
                      </div>
                    ))}
                  </div>
                </details>
              </Panel>
            </section>
          );
        })}
      </PowerOutline>
    </div>
  );
}
