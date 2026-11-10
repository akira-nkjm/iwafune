import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { q, useDesign } from "@/api/queries";
import type { Severity } from "@/api/types";
import { useT } from "@/i18n";
import { DiagnosticCard, diagnosticKey } from "@/kit/DiagnosticItem";
import { Empty } from "@/kit/Empty";
import { Loading, SkeletonPanel } from "@/kit/Loading";
import { Segmented } from "@/kit/Segmented";
import { targetLink } from "@/lib/target-link";
import { PageBody } from "@/shell/PageBody";
import { PageHeader } from "@/shell/PageHeader";

type Filter = "all" | Severity;

/** プロジェクト / 診断: fune check と同じ検証結果。該当ファイルの行と、対象（部品・モード・設定など）のページへのリンク */
export function DiagnosticsPage() {
  const t = useT();
  const p = t.project;
  const report = useQuery(q.diagnostics());
  const design = useDesign();
  const [filter, setFilter] = useState<Filter>("all");
  const parts = new Set(design.data?.parts.map((p) => p.id) ?? []);
  const r = report.data;
  const list = (r?.diagnostics ?? []).filter((d) => filter === "all" || d.severity === filter);
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
            options={[
              { value: "all", label: p.filter.all },
              { value: "error", label: p.filter.error },
              { value: "warning", label: p.filter.warning },
              { value: "info", label: p.filter.info },
            ]}
          />
        }
      />
      <PageBody>
        {report.isError ? (
          <Empty>{p.cannotLoadDiagnostics(report.error.message)}</Empty>
        ) : !r ? (
          <Loading className="space-y-2">
            <SkeletonPanel lines={2} />
            <SkeletonPanel lines={2} />
            <SkeletonPanel lines={2} />
          </Loading>
        ) : list.length === 0 ? (
          <Empty>{p.noDiagnostics}</Empty>
        ) : (
          <div className="space-y-2">
            {list.map((d) => (
              <DiagnosticCard
                key={diagnosticKey(d)}
                diagnostic={d}
                to={targetLink(d.target, parts)}
              />
            ))}
          </div>
        )}
      </PageBody>
    </>
  );
}
