import { AnalysisTable } from "@/analysis/AnalysisTable";
import type { AnalysisState } from "@/api/queries";
import type { AnalysisEntry, DesignView } from "@/api/types";
import { useT } from "@/i18n";
import { TablePanel } from "@/kit/table/TablePanel";

/**
 * 系の概要（熱と電源のページを除く）。電源の概要と同じ解析の表で、1 行 = 1 解析（行の中身は結果に API が添えた
 * `AnalysisRunResult.overview`）。結果がまだ無い解析は読み込み中のスケルトンの行、問い合わせが失敗した解析はエラーの行。
 */
export function SystemOverview({
  states,
  entries,
  design,
}: {
  states: readonly AnalysisState[];
  entries: ReadonlyMap<string, AnalysisEntry>;
  design?: DesignView;
}) {
  const table = useT().systems.analysisTable;
  if (states.length === 0) return null;
  const items = states.map((s) => {
    const entry = entries.get(s.id);
    return s.result && entry
      ? { entry, result: s.result }
      : { id: s.id, entry, error: s.result ? null : s.error };
  });
  return (
    <TablePanel title={table.title} caption={table.caption}>
      <AnalysisTable items={items} design={design} />
    </TablePanel>
  );
}
