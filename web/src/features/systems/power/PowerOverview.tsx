import { AnalysisTable } from "@/analysis/AnalysisTable";
import type { DesignView, PowerPageView } from "@/api/types";
import { useT } from "@/i18n";
import { TablePanel } from "@/kit/table/TablePanel";

/**
 * 電源の概要。解析の表（1 行 = 1 解析。主な結果・判定・足りない入力の数、開くと KPI の全部・棒グラフ・判定の文・生の表。
 * 行の中身は結果に API が添えた `AnalysisRunResult.overview`。電源は棒グラフと隠す表も入る）。
 */
export function PowerOverview({ page, design }: { page: PowerPageView; design?: DesignView }) {
  const table = useT().systems.analysisTable;
  return (
    <TablePanel title={table.title} caption={table.caption}>
      <AnalysisTable items={page.analyses} design={design} />
    </TablePanel>
  );
}
