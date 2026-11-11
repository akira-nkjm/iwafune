import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { useEnumValues } from "@/api/labels";
import { q, useDesign } from "@/api/queries";
import { useT } from "@/i18n";
import { Empty } from "@/kit/feedback/Empty";
import { SkeletonPanel } from "@/kit/feedback/Loading";
import { QueryState } from "@/kit/feedback/QueryState";
import { Segmented } from "@/kit/selection/Segmented";
import { DiagnosticCard, diagnosticKey } from "@/kit/status/DiagnosticItem";
import { targetLink } from "@/lib/target-link";
import { PageBody } from "@/shell/PageBody";
import { PageHeader } from "@/shell/PageHeader";

/** 絞り込み: すべてか、重さ（API の `Severity`。名前と並びは `labels.enum.severity`・`enum_order.severity`） */
const ALL = "all";

/** プロジェクト / 診断: fune check と同じ検証結果。該当ファイルの行と、対象（部品・モード・設定など）のページへのリンク */
export function DiagnosticsPage() {
  const t = useT();
  const p = t.project;
  const report = useQuery(q.diagnostics());
  const design = useDesign();
  const severities = useEnumValues("severity");
  const [filter, setFilter] = useState<string>(ALL);
  const parts = new Set(design.data?.parts.map((p) => p.id) ?? []);
  const r = report.data;
  return (
    <>
      <PageHeader
        eyebrow={p.eyebrow}
        title={p.diagnostics}
        subtitle={
          r ? p.diagSubtitle(r.counts.errors, r.counts.warnings, r.counts.infos) : undefined
        }
        actions={
          <Segmented
            label={p.filterBySeverity}
            value={filter}
            onChange={setFilter}
            options={[{ value: ALL, label: t.common.all }, ...severities]}
          />
        }
      />
      <PageBody>
        <QueryState
          query={report}
          failure={p.cannotLoadDiagnostics}
          className="flex flex-col gap-2"
          skeleton={
            <>
              <SkeletonPanel lines={2} />
              <SkeletonPanel lines={2} />
              <SkeletonPanel lines={2} />
            </>
          }
        >
          {(report) => {
            const list = report.diagnostics.filter((d) => filter === ALL || d.severity === filter);
            if (list.length === 0) return <Empty>{p.noDiagnostics}</Empty>;
            return (
              <div className="space-y-2">
                {list.map((d) => (
                  <DiagnosticCard
                    key={diagnosticKey(d)}
                    diagnostic={d}
                    to={targetLink(d.target, parts)}
                  />
                ))}
              </div>
            );
          }}
        </QueryState>
      </PageBody>
    </>
  );
}
