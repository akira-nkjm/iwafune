import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { Link } from "react-router";
import { q, useDesign } from "@/api/queries";
import type { Severity } from "@/api/types";
import { useT } from "@/i18n";
import { Empty } from "@/kit/Empty";
import { Loading, SkeletonPanel } from "@/kit/Loading";
import { Panel } from "@/kit/Panel";
import { Segmented } from "@/kit/Segmented";
import { SeverityPill } from "@/kit/SeverityPill";
import { PageBody } from "@/shell/PageBody";
import { PageHeader } from "@/shell/PageHeader";

type Filter = "all" | Severity;

/** プロジェクト / 診断: fune check と同じ検証結果。該当ファイルの行と、部品のページへのリンク */
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
            {list.map((d) => {
              const head = d.part;
              return (
                <Panel
                  key={`${d.code}-${d.path}-${d.file}-${d.line}-${d.column}`}
                  className="space-y-1.5 py-3"
                >
                  <div className="flex flex-wrap items-center gap-2 text-xs">
                    <SeverityPill severity={d.severity} />
                    <span className="font-mono text-ink-3">{d.code}</span>
                    {d.path &&
                      (head && parts.has(head) ? (
                        <Link
                          to={`/design/components/${head}`}
                          className="font-mono text-accent-ink hover:underline"
                        >
                          {d.path}
                        </Link>
                      ) : (
                        <span className="font-mono">{d.path}</span>
                      ))}
                    {d.file && (
                      <span className="ml-auto font-mono text-ink-4">
                        {d.file}
                        {d.line ? `:${d.line}${d.column ? `:${d.column}` : ""}` : ""}
                      </span>
                    )}
                  </div>
                  <p className="text-ink text-md">{d.message}</p>
                  {d.excerpt && (
                    <pre className="overflow-x-auto rounded-control bg-panel-2 px-3 py-2 font-mono text-[11.5px] text-ink-2">
                      {d.excerpt.text.split("\n").map((line, i) => {
                        const n = (d.excerpt?.first_line ?? 1) + i;
                        return (
                          <div key={n} className={n === d.line ? "bg-warn-bg" : undefined}>
                            <span className="mr-3 inline-block w-8 select-none text-right text-ink-4">
                              {n}
                            </span>
                            {line}
                          </div>
                        );
                      })}
                    </pre>
                  )}
                </Panel>
              );
            })}
          </div>
        )}
      </PageBody>
    </>
  );
}
