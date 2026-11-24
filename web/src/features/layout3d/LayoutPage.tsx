import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { useEffect, useMemo } from "react";
import { useSearchParams } from "react-router";
import { FINDING_ANALYSES } from "@/analysis/geometry-findings";
import { analysisOutput } from "@/api/analysis-output";
import { q, useDesign } from "@/api/queries";
import type { AnalysisRunResult, DesignView } from "@/api/types";
import { useT } from "@/i18n";
import { SkeletonBlock } from "@/kit/feedback/Loading";
import { QueryState } from "@/kit/feedback/QueryState";
import { SEARCH, withLayoutConfiguration } from "@/lib/nav";
import { PageHeader } from "@/shell/PageHeader";
import { ConfigurationControls } from "./ConfigurationControls";
import { LazyLayout3D } from "./LazyLayout3D";
import type { V3 } from "./scene";

/** 設計 / 配置。質量特性が未取得・入力不足でも配置は表示する。 */
export function LayoutPage() {
  const design = useDesign();
  const t = useT();
  return (
    <>
      <PageHeader
        eyebrow={t.nav.sections.design}
        title={t.nav.items.layout}
        subtitle={t.systems.layout3d.pageHint}
      />
      <div className="flex min-h-0 flex-1 flex-col px-6 pb-4">
        <QueryState
          query={design}
          className="flex min-h-0 flex-1 flex-col"
          skeleton={<SkeletonBlock className="min-h-0 flex-1 rounded-card" />}
        >
          {(d) => <LayoutContent design={d} />}
        </QueryState>
      </div>
    </>
  );
}

function LayoutContent({ design }: { design: DesignView }) {
  const [params, setParams] = useSearchParams();
  const requested = params.get(SEARCH.configuration);
  const selected = design.configurations.find(({ id }) => id === requested) ?? null;
  const configuration = selected?.id ?? null;
  // 形態を切り替えている間は前の一覧を出し続ける（3D を作り直さず、選択とカメラを保つ。変わるのは姿勢だけ）
  const parts = useQuery({ ...q.parts(configuration), placeholderData: keepPreviousData });
  // 表示中の形態の配置で重心を求める（Base は選択なし）
  const mass = useQuery(q.analysis("structure.mass_properties", null, [], configuration));
  const c = analysisOutput(mass.data, "structure.mass_properties")?.summary.center_of_mass;
  const com: V3 | null = c ? [c.x, c.y, c.z] : null;
  // 表示中の形態での幾何の所見（干渉・円錐の遮り）。3D の所見の表示が使う（入力不足・失敗のときは載せない）
  const interference = useQuery(q.analysis(FINDING_ANALYSES[0], null, [], configuration));
  const cones = useQuery(q.analysis(FINDING_ANALYSES[1], null, [], configuration));
  const findings = useMemo(
    () => [interference.data, cones.data].filter((r): r is AnalysisRunResult => r?.status === "ok"),
    [interference.data, cones.data],
  );

  useEffect(() => {
    if (requested !== null && !configuration)
      setParams(withLayoutConfiguration(params, null), { replace: true });
  }, [requested, configuration, params, setParams]);

  return (
    <>
      <ConfigurationControls
        configurations={design.configurations}
        selected={selected}
        onSelect={(id, options) => setParams(withLayoutConfiguration(params, id), options)}
      />
      <QueryState
        query={parts}
        className="flex min-h-0 flex-1 flex-col"
        skeleton={<SkeletonBlock className="min-h-0 flex-1 rounded-card" />}
      >
        {(rows) => (
          <LazyLayout3D
            variant="placement"
            design={{ ...design, parts: rows }}
            configuration={configuration}
            com={com}
            findings={findings}
          />
        )}
      </QueryState>
    </>
  );
}
