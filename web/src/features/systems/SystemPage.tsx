import { useMemo } from "react";
import { useParams, useSearchParams } from "react-router";
import { useAnalyses, useDesign, useSystemAnalyses } from "@/api/queries";
import type { AnalysisEntry, DefsView, System } from "@/api/types";
import { useT } from "@/i18n";
import { Empty } from "@/kit/feedback/Empty";
import { SkeletonPage, SkeletonPanel } from "@/kit/feedback/Loading";
import { QueryState } from "@/kit/feedback/QueryState";
import { PageTabs } from "@/kit/selection/PageTabs";
import { systemTitle } from "@/lib/design";
import { paths, SEARCH } from "@/lib/nav";
import { PageBody } from "@/shell/PageBody";
import { PageHeader } from "@/shell/PageHeader";
import { systemExtras } from "./extras";
import { hasExtraTab } from "./preload";
import { SystemHeaderActions } from "./SystemHeaderActions";
import { hasModes, type SystemTab, SystemTabs, tabFillsBody } from "./SystemTabs";

const TABS: SystemTab[] = [
  "results",
  "maneuvers",
  "heat",
  "analyses",
  "modes",
  "series",
  "settings",
];

/** 系にそのタブがあるか（モードの見方は持つ系だけ、熱画像・解析結果のような追加のタブは `preload.ts` の `EXTRA_TABS` の系だけ） */
function shows(system: System, tab: SystemTab, defs: DefsView | undefined): boolean {
  if (tab === "modes") return hasModes(system, defs);
  if (tab === "heat" || tab === "analyses" || tab === "maneuvers") return hasExtraTab(system, tab);
  return true;
}

function isTab(s: string | undefined): s is SystemTab {
  return TABS.some((t) => t === s);
}

type DesignQuery = ReturnType<typeof useDesign>;

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
 * 系の解析のページ（解析が未登録の系も含む）。どの系があるかは API（`DefsView.systems`。設計と一緒に届く）で、
 * 届くまではページの骨組み、読めなければ理由と再試行を出す。
 */
export function SystemPage() {
  const t = useT();
  const { system: param } = useParams();
  const design = useDesign();
  const systems = design.data?.defs.systems;
  const system = systems?.find((s) => s === param);
  if (system) return <SystemView system={system} design={design} />;
  if (systems) return <Empty>{t.systems.unknown(param ?? "")}</Empty>;
  if (!design.error)
    return (
      <SkeletonPage kpis tabs>
        <SystemBodySkeleton />
      </SkeletonPage>
    );
  return (
    <>
      <PageHeader eyebrow={t.systems.eyebrow} title={param ?? ""} />
      <PageBody>
        <QueryState query={design} skeleton={<SystemBodySkeleton />}>
          {() => null}
        </QueryState>
      </PageBody>
    </>
  );
}

/**
 * 1 つの系のページ。共通の型: 上にケースの選択、下にタブ（結果・内訳・設定・時系列）。
 * その系の解析は GET /analyses の info.area から引く。
 */
function SystemView({ system, design }: { system: System; design: DesignQuery }) {
  const t = useT();
  const { tab: rawTab } = useParams();
  const [params, setParams] = useSearchParams();
  const kase = params.get(SEARCH.case);
  const catalog = useSystemAnalyses(system);
  const states = useAnalyses(catalog.ids, kase);
  const entries = useMemo(
    () => new Map<string, AnalysisEntry>(catalog.entries.map((e) => [e.info.id, e])),
    [catalog.entries],
  );
  const tab: SystemTab =
    isTab(rawTab) && shows(system, rawTab, design.data?.defs) ? rawTab : "results";
  const tabs = (d: NonNullable<DesignQuery["data"]>) => (
    <SystemTabs
      tab={tab}
      system={system}
      kase={kase}
      design={d}
      noAnalyses={!catalog.loading && catalog.entries.length === 0}
      states={states}
      entries={entries}
      extra={systemExtras(system, d, states, kase)}
    />
  );
  return (
    <>
      <PageHeader
        eyebrow={t.systems.eyebrow}
        title={systemTitle(system, design.data?.defs)}
        subtitle={t.systems.specs[system].description}
        actions={
          <SystemHeaderActions
            cases={design.data?.cases ?? []}
            kase={kase}
            defaultCaseId={
              states.find((s) => s.result?.case_source === "default")?.result?.case ?? null
            }
            onCase={(c) => setParams(c ? { [SEARCH.case]: c } : {})}
            ids={catalog.ids}
          />
        }
      />
      <PageTabs
        tabs={TABS.filter((k) => shows(system, k, design.data?.defs)).map((k) => ({
          to: paths.system(system, { tab: k === "results" ? null : k, kase }),
          label: t.systems.tabs[k],
        }))}
      />
      {/* 部品 × モードの表は、本文の高さいっぱいを使い、表の中でスクロールする（見出しと合計の行を止めたままにする） */}
      <PageBody
        className={
          tabFillsBody(system, tab, design.data?.defs)
            ? "flex flex-col overflow-hidden p-0"
            : undefined
        }
      >
        <QueryState query={design} skeleton={<SystemBodySkeleton />}>
          {tabs}
        </QueryState>
      </PageBody>
    </>
  );
}
