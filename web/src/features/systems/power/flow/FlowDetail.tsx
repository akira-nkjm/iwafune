import { ChevronDown, ChevronUp } from "lucide-react";
import { useState } from "react";
import { ChecksList } from "@/analysis/ChecksList";
import { MissingList } from "@/analysis/MissingList";
import type { AnalysisRunResult, DesignView, FlowNode, FlowRow } from "@/api/types";
import { localized, useT } from "@/i18n";
import { DetailHeader, DetailPane } from "@/kit/detail/DetailPane";
import { DetailSection } from "@/kit/detail/DetailSection";
import { PropertyList } from "@/kit/detail/PropertyList";
import { PropertyRow } from "@/kit/detail/PropertyRow";
import { VerdictPill } from "@/kit/status/VerdictPill";
import { systemTone } from "@/lib/systems";
import { RowValue } from "./FlowCard";

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
          subtitle={node.members.length > 0 ? t.members(node.members.length) : node.sub}
          tone={tone}
          mono={false}
          trailing={node.verdict ? <VerdictPill verdict={node.verdict} /> : undefined}
          onClose={onClose}
        />
      }
    >
      <Values node={node} />
      {node.members.length > 0 && (
        <DetailSection title={t.memberList}>
          <PropertyList>
            {node.members.map((m) => (
              <MemberRow key={m.id} id={m.id} name={localized(m.label, m.label_en)} />
            ))}
          </PropertyList>
        </DetailSection>
      )}
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

/** 値の一覧。書かれた値（解析は主な出力）を全部と、書かれていない値（既定値を使う）は開いて見る */
function Values({ node }: { node: FlowNode }) {
  const t = useT().systems.flow;
  const [open, setOpen] = useState(false);
  const set = node.rows.filter((r) => r.set);
  const unset = node.rows.filter((r) => !r.set);
  return (
    <>
      <DetailSection title={node.kind === "analysis" ? t.outputs : t.values}>
        {set.length === 0 ? (
          <p className="px-0.5 text-ink-3 text-xs">{t.noOutputs}</p>
        ) : (
          <Rows rows={set} />
        )}
      </DetailSection>
      {unset.length > 0 && (
        <DetailSection
          title={t.unsetSection(unset.length)}
          right={
            <button
              type="button"
              onClick={() => setOpen(!open)}
              aria-expanded={open}
              className="flex items-center gap-0.5 text-[11.5px] text-accent-ink hover:underline"
            >
              {open ? <ChevronUp size={12} aria-hidden /> : <ChevronDown size={12} aria-hidden />}
              {open ? t.fewer : t.showAll}
            </button>
          }
        >
          {open && <Rows rows={unset} />}
        </DetailSection>
      )}
    </>
  );
}

/** 部品の一覧の 1 行: ID（等幅）と、ID と違えば表示名 */
function MemberRow({ id, name }: { id: string; name: string }) {
  return (
    <div className="flex items-baseline gap-3 px-3 py-1 text-xs">
      <span className="shrink-0 font-mono text-ink">{id}</span>
      {name !== id && <span className="min-w-0 truncate text-ink-2">{name}</span>}
    </div>
  );
}

function Rows({ rows }: { rows: FlowRow[] }) {
  return (
    <PropertyList>
      {rows.map((r) => (
        <PropertyRow key={r.id} label={localized(r.label, r.label_en)}>
          <span className="flex items-center gap-2">
            <RowValue row={r} />
            {r.verdict && <VerdictPill verdict={r.verdict} />}
          </span>
        </PropertyRow>
      ))}
    </PropertyList>
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
