import { ChevronRight } from "lucide-react";
import { useId, useState } from "react";
import type { AnalysisEntry, AnalysisOverview, AnalysisRunResult, DesignView } from "@/api/types";
import { localized, useT } from "@/i18n";
import { analysisTitle } from "@/i18n/analysis-names";
import { Pill } from "@/kit/status/Pill";
import { VerdictPill } from "@/kit/status/VerdictPill";
import { DataTable } from "@/kit/table/DataTable";
import { Td } from "@/kit/table/Td";
import { Th } from "@/kit/table/Th";
import { cn } from "@/lib/cn";
import { AnalysisRowDetail } from "./AnalysisRowDetail";
import { kpiText } from "./kpi";

/** 解析の表の 1 行: 解析の定義・結果と、API が添えた概要（主な結果・判定のまとまり・棒グラフ・隠す表） */
export interface AnalysisTableItem {
  entry: AnalysisEntry;
  result: AnalysisRunResult;
  overview: AnalysisOverview;
}

/** 開いている行の ID の列で、`id` の開閉を入れ替える */
export function toggleOpen(open: readonly string[], id: string): string[] {
  return open.includes(id) ? open.filter((x) => x !== id) : [...open, id];
}

const COLUMNS = 5;

/**
 * 系の概要の解析の表（1 行 = 1 解析: 名前・定常か時系列か・主な結果・判定・足りない入力の数）。行を押すと
 * その場で開く（`AnalysisRowDetail`）。どの系でも同じ部品で、値・判定・出す行は API のまま並べる。
 */
export function AnalysisTable({
  items,
  design,
  initialOpen = [],
}: {
  items: readonly AnalysisTableItem[];
  design?: DesignView;
  /** はじめから開いておく行（解析の ID） */
  initialOpen?: readonly string[];
}) {
  const t = useT().systems.analysisTable;
  const [open, setOpen] = useState<readonly string[]>(initialOpen);
  return (
    <DataTable
      head={
        <>
          <Th>{t.analysis}</Th>
          <Th>{t.kind}</Th>
          <Th>{t.main}</Th>
          <Th>{t.verdict}</Th>
          <Th align="right">{t.missing}</Th>
        </>
      }
    >
      {items.map((item) => (
        <AnalysisTableRow
          key={item.result.id}
          item={item}
          design={design}
          open={open.includes(item.result.id)}
          onToggle={() => setOpen((o) => toggleOpen(o, item.result.id))}
        />
      ))}
    </DataTable>
  );
}

const statusTone = { missing_inputs: "warn", failed: "neg" } as const;

/** 解析の表の 1 行と、開いたときの中身（行の下に全幅で出す）。開閉は行の先頭のボタン（`aria-expanded`）か行を押す */
export function AnalysisTableRow({
  item,
  design,
  open,
  onToggle,
}: {
  item: AnalysisTableItem;
  design?: DesignView;
  open: boolean;
  onToggle: () => void;
}) {
  const dict = useT();
  const t = dict.systems;
  const detailId = useId();
  const { entry, result, overview } = item;
  const missing = result.missing.length;
  return (
    <>
      <tr
        onClick={onToggle}
        className={cn("cursor-pointer hover:bg-panel-2", open && "bg-panel-2")}
      >
        <Td className="whitespace-normal">
          <button
            type="button"
            aria-expanded={open}
            aria-controls={open ? detailId : undefined}
            onClick={(e) => {
              e.stopPropagation();
              onToggle();
            }}
            className="inline-flex items-center gap-1.5 rounded-sm text-left font-medium text-ink focus-visible:outline-2 focus-visible:outline-accent"
          >
            <ChevronRight
              size={14}
              aria-hidden
              className={cn("shrink-0 text-ink-3 transition-transform", open && "rotate-90")}
            />
            {analysisTitle(entry.info, result.id)}
          </button>
          <div className="pl-5 font-mono text-2xs text-ink-3">{result.id}</div>
        </Td>
        <Td>
          <Pill dot={false}>{entry.info.kind === "steady" ? t.steady : t.timeSeries}</Pill>
        </Td>
        <Td className="whitespace-normal">
          <MainResults kpis={overview.main} />
        </Td>
        <Td>
          <RowVerdict result={result} />
        </Td>
        <Td align="right">
          {missing > 0 ? (
            <Pill tone="warn">{t.analysisTable.missingCount(missing)}</Pill>
          ) : (
            <span className="text-ink-3">—</span>
          )}
        </Td>
      </tr>
      {open && (
        <tr id={detailId}>
          <td colSpan={COLUMNS} className="px-3 pt-2 pb-4">
            <AnalysisRowDetail result={result} overview={overview} design={design} />
          </td>
        </tr>
      )}
    </>
  );
}

/** 行の主な結果（API の `AnalysisOverview.main`）: 名前と値を折り返して並べる */
function MainResults({ kpis }: { kpis: AnalysisOverview["main"] }) {
  const t = useT().systems;
  if (kpis.length === 0)
    return <span className="text-ink-3 text-xs">{t.analysisTable.noResults}</span>;
  return (
    <ul className="flex flex-wrap gap-x-4 gap-y-0.5 text-xs">
      {kpis.map((k) => (
        <li key={k.key} className="flex items-baseline gap-1.5">
          <span className="text-ink-3">{localized(k.label, k.label_en)}</span>
          <span className="num font-medium text-ink">
            {kpiText(k, (n) => t.kpiTally(n.checked, n.flagged))}
          </span>
        </li>
      ))}
    </ul>
  );
}

/** 行の判定: 解析の判定、無ければ実行の状態（入力不足・失敗）、どちらも無ければ「—」 */
function RowVerdict({ result }: { result: AnalysisRunResult }) {
  const t = useT().systems;
  if (result.verdict) return <VerdictPill verdict={result.verdict} />;
  if (result.status !== "ok")
    return <Pill tone={statusTone[result.status]}>{t.status[result.status]}</Pill>;
  return <span className="text-ink-3">—</span>;
}
