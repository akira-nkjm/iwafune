import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router";
import { q, useDesign } from "@/api/queries";
import { useT } from "@/i18n";
import { DiagnosticItem, diagnosticKey } from "@/kit/DiagnosticItem";
import { Panel } from "@/kit/Panel";
import { PanelHeader } from "@/kit/PanelHeader";
import { targetLink } from "@/lib/target-link";

/** 概要の診断（先頭の 8 件と件数）。対象のパスから直しに行くページへ飛べる。すべては プロジェクト / 診断 */
export function DiagnosticsPanel() {
  const o = useT().overview;
  const report = useQuery(q.diagnostics()).data;
  const parts = new Set(useDesign().data?.parts.map((p) => p.id) ?? []);
  return (
    <Panel>
      <PanelHeader
        title={o.diagnostics}
        caption={
          report
            ? o.diagCounts(report.counts.errors, report.counts.warnings, report.counts.infos)
            : undefined
        }
        right={
          <Link to="/project/diagnostics" className="text-accent-ink text-xs hover:underline">
            {o.seeAll}
          </Link>
        }
      />
      {report && report.diagnostics.length === 0 && (
        <p className="text-pos-ink text-xs">{o.noProblems}</p>
      )}
      <ul className="space-y-1">
        {report?.diagnostics.slice(0, 8).map((x) => (
          <DiagnosticItem key={diagnosticKey(x)} diagnostic={x} to={targetLink(x.target, parts)} />
        ))}
      </ul>
    </Panel>
  );
}
