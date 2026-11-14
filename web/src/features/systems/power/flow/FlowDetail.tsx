import { ChecksList } from "@/analysis/ChecksList";
import { MissingList } from "@/analysis/MissingList";
import type { AnalysisRunResult, DesignView, FlowNode, FlowRow } from "@/api/types";
import { localized, useT } from "@/i18n";
import { DetailHeader, DetailPane } from "@/kit/detail/DetailPane";
import { DetailSection } from "@/kit/detail/DetailSection";
import { PropertyList } from "@/kit/detail/PropertyList";
import { PropertyRow } from "@/kit/detail/PropertyRow";
import { VerdictPill } from "@/kit/status/VerdictPill";
import { cn } from "@/lib/cn";
import { withNote } from "@/lib/design";
import { systemTone } from "@/lib/systems";

/**
 * フロー図で選んだカードの詳細（右の欄。読むだけ）: 全部の値（解析は主な出力と判定の文・足りない入力）と、つながる
 * カード（押すとそのカードを選ぶ）。
 */
export function FlowDetail({
  node,
  result,
  from,
  to,
  design,
  onSelect,
  onClose,
}: {
  node: FlowNode;
  /** 解析のカードなら、その解析の結果（同じページの解析の一覧から） */
  result: AnalysisRunResult | undefined;
  /** このカードへ線を引くカード */
  from: FlowNode[];
  /** このカードから線を引く先のカード */
  to: FlowNode[];
  design: DesignView | undefined;
  onSelect: (id: string) => void;
  onClose: () => void;
}) {
  const t = useT().systems.flow;
  const analysis = node.kind === "analysis";
  const tone = node.kind === "part" || analysis ? systemTone(node.system) : "gray";
  return (
    <DetailPane
      header={
        <DetailHeader
          title={localized(node.label, node.label_en)}
          subtitle={node.sub}
          tone={tone}
          trailing={node.verdict ? <VerdictPill verdict={node.verdict} /> : undefined}
          onClose={onClose}
        />
      }
    >
      <DetailSection title={analysis ? t.outputs : t.values}>
        {node.rows.length === 0 ? (
          <p className="px-0.5 text-ink-3 text-xs">{t.noOutputs}</p>
        ) : (
          <PropertyList>
            {node.rows.map((r) => (
              <PropertyRow key={r.id} label={localized(r.label, r.label_en)} title={r.id}>
                <RowText row={r} unset={t.unset} derived={t.derived} />
              </PropertyRow>
            ))}
          </PropertyList>
        )}
      </DetailSection>
      {result && result.checks.length > 0 && (
        <DetailSection title={t.checks}>
          <ChecksList checks={result.checks} />
        </DetailSection>
      )}
      {result && <MissingList design={design} missing={result.missing} />}
      {analysis ? (
        <>
          <Links
            title={t.reads}
            nodes={from.filter((n) => n.kind !== "analysis")}
            onSelect={onSelect}
          />
          <Links
            title={t.usesResults}
            nodes={from.filter((n) => n.kind === "analysis")}
            onSelect={onSelect}
          />
          <Links title={t.resultOf} nodes={to} onSelect={onSelect} />
        </>
      ) : (
        <Links title={t.usedBy} nodes={to} onSelect={onSelect} />
      )}
    </DetailPane>
  );
}

function RowText({ row, unset, derived }: { row: FlowRow; unset: string; derived: string }) {
  if (!row.display) return <span className="text-ink-4">{row.set ? "—" : unset}</span>;
  return (
    <span className={cn("font-mono text-[11.5px]", row.derived && "text-ink-3 italic")}>
      {row.derived ? withNote(row.display, derived) : row.display}
      {row.verdict && (
        <span className="ml-2 align-middle">
          <VerdictPill verdict={row.verdict} />
        </span>
      )}
    </span>
  );
}

function Links({
  title,
  nodes,
  onSelect,
}: {
  title: string;
  nodes: FlowNode[];
  onSelect: (id: string) => void;
}) {
  if (nodes.length === 0) return null;
  return (
    <DetailSection title={title}>
      <ul className="flex flex-wrap gap-1.5">
        {nodes.map((n) => (
          <li key={n.id}>
            <button
              type="button"
              onClick={() => onSelect(n.id)}
              className="rounded-chip border border-line bg-panel-2 px-2 py-0.5 text-[11.5px] text-ink-2 hover:border-accent hover:text-accent-ink"
            >
              {localized(n.label, n.label_en)}
            </button>
          </li>
        ))}
      </ul>
    </DetailSection>
  );
}
