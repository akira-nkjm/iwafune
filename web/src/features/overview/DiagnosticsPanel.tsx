import { useQuery } from "@tanstack/react-query";
import { Link } from "react-router";
import { q } from "@/api/queries";
import { useT } from "@/i18n";
import { Panel } from "@/kit/Panel";
import { PanelHeader } from "@/kit/PanelHeader";
import { SeverityPill } from "@/kit/SeverityPill";

/** 概要の診断（先頭の 8 件と件数）。すべては プロジェクト / 診断 */
export function DiagnosticsPanel() {
  const o = useT().overview;
  const report = useQuery(q.diagnostics()).data;
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
          <li
            key={`${x.code}-${x.path}-${x.file}-${x.line}`}
            className="flex items-start gap-2 text-xs"
          >
            <SeverityPill severity={x.severity} />
            <span className="min-w-0">
              {x.path && <span className="mr-1 font-mono text-ink-3">{x.path}</span>}
              {x.message}
              {x.file && (
                <span className="ml-1 font-mono text-ink-4">
                  {x.file}
                  {x.line ? `:${x.line}` : ""}
                </span>
              )}
            </span>
          </li>
        ))}
      </ul>
    </Panel>
  );
}
