import { useMemo } from "react";
import { useParams, useSearchParams } from "react-router";
import { useAnalyses, useDesign, usePowerPage, useSystemAnalyses } from "@/api/queries";
import type { AnalysisEntry, PowerPageView, System } from "@/api/types";
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
import { SystemHeaderActions } from "./SystemHeaderActions";
import { hasModes, type SystemTab, SystemTabs } from "./SystemTabs";

const TABS: SystemTab[] = ["results", "modes", "series", "settings"];

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
  const genericStates = useAnalyses(system === "power" ? [] : catalog.ids, kase);
  const powerPage = usePowerPage(kase, system === "power");
  const states =
    system === "power"
      ? (powerPage.data?.analyses ?? []).map((a) => ({
          id: a.result.id,
          result: a.result,
          loading: false,
          updating: powerPage.isFetching,
          error: null,
        }))
      : genericStates;
  const entries = useMemo(
    () => new Map<string, AnalysisEntry>(catalog.entries.map((e) => [e.info.id, e])),
    [catalog.entries],
  );
  const tab: SystemTab =
    isTab(rawTab) && (rawTab !== "modes" || hasModes(system)) ? rawTab : "results";
  const tabs = (d: NonNullable<DesignQuery["data"]>, page?: PowerPageView) => (
    <SystemTabs
      tab={tab}
      system={system}
      design={d}
      powerPage={page}
      noAnalyses={!catalog.loading && catalog.entries.length === 0}
      states={states}
      entries={entries}
      extra={systemExtras(system, d, states)}
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
        tabs={TABS.filter((k) => k !== "modes" || hasModes(system)).map((k) => ({
          to: paths.system(system, { tab: k === "results" ? null : k, kase }),
          label: t.systems.tabs[k],
        }))}
      />
      <PageBody>
        <QueryState query={design} skeleton={<SystemBodySkeleton />}>
          {(d) =>
            system === "power" ? (
              <QueryState query={powerPage} skeleton={<SystemBodySkeleton />}>
                {(page) => tabs(d, page)}
              </QueryState>
            ) : (
              tabs(d)
            )
          }
        </QueryState>
      </PageBody>
    </>
  );
}
