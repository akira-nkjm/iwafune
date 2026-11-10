import { useMemo } from "react";
import { useParams, useSearchParams } from "react-router";
import { useDesign, useSystemAnalyses } from "@/api/queries";
import type { AnalysisEntry } from "@/api/types";
import { useT } from "@/i18n";
import { Empty } from "@/kit/Empty";
import { SkeletonPanel } from "@/kit/Loading";
import { PageTabs } from "@/kit/PageTabs";
import { QueryState } from "@/kit/QueryState";
import { paths, SEARCH } from "@/lib/nav";
import { PageBody } from "@/shell/PageBody";
import { PageHeader } from "@/shell/PageHeader";
import { systemExtras } from "./extras";
import { SystemHeaderActions } from "./SystemHeaderActions";
import { SystemKpis } from "./SystemKpis";
import { type SystemTab, SystemTabs } from "./SystemTabs";
import { isSystem, systemDescription, systems, systemTitle } from "./systems";
import { useAnalyses } from "./use-analyses";

const TABS: SystemTab[] = ["results", "breakdown", "settings", "series"];

function isTab(s: string | undefined): s is SystemTab {
  return TABS.some((t) => t === s);
}

/** 設計を読むまでのタブの中身（解析のカードの 2 列） */
function SystemBodySkeleton() {
  return (
    <div className="grid gap-4 xl:grid-cols-2">
      <SkeletonPanel lines={5} />
      <SkeletonPanel lines={5} />
    </div>
  );
}

/**
 * 系の解析のページ（電源・姿勢・C&DH・通信・熱・構造）。共通の型: 上にケースの選択と主要 KPI、
 * 下にタブ（結果・内訳・設定・時系列）。その系の解析は GET /analyses の info.system から引く。
 */
export function SystemPage() {
  const t = useT();
  const { system, tab: rawTab } = useParams();
  const [params, setParams] = useSearchParams();
  const design = useDesign();
  const kase = params.get(SEARCH.case);
  const spec = isSystem(system) ? systems[system] : null;
  const catalog = useSystemAnalyses(spec?.key ?? null);
  const states = useAnalyses(catalog.ids, kase);
  const entries = useMemo(
    () => new Map<string, AnalysisEntry>(catalog.entries.map((e) => [e.info.id, e])),
    [catalog.entries],
  );
  if (!spec) return <Empty>{t.systems.unknown(system ?? "")}</Empty>;
  const tab: SystemTab = isTab(rawTab) ? rawTab : "results";
  return (
    <>
      <PageHeader
        eyebrow={t.systems.eyebrow}
        title={systemTitle(spec.key, design.data?.defs)}
        subtitle={systemDescription(spec.key)}
        actions={
          <SystemHeaderActions
            cases={design.data?.cases ?? []}
            kase={kase}
            onCase={(c) => setParams(c ? { [SEARCH.case]: c } : {})}
            ids={catalog.ids}
          />
        }
      />
      <div className="px-6 pb-3">
        <SystemKpis system={spec.key} kpis={spec.kpis} states={states} />
      </div>
      <PageTabs
        tabs={TABS.map((k) => ({
          to: paths.system(spec.key, { tab: k === "results" ? null : k, kase }),
          label: t.systems.tabs[k],
        }))}
      />
      <PageBody>
        <QueryState query={design} skeleton={<SystemBodySkeleton />}>
          {(d) => (
            <SystemTabs
              tab={tab}
              spec={spec}
              design={d}
              states={states}
              entries={entries}
              extra={systemExtras(spec.key, d, states)}
            />
          )}
        </QueryState>
      </PageBody>
    </>
  );
}
