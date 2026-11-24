import { useQuery } from "@tanstack/react-query";
import { useEffect } from "react";
import { useSearchParams } from "react-router";
import { analysisOutput } from "@/api/analysis-output";
import { q, useDesign } from "@/api/queries";
import type { DesignView } from "@/api/types";
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
  const parts = useQuery(q.parts(configuration));
  const mass = useQuery({ ...q.analysis("structure.mass_properties", null), enabled: !selected });
  const c =
    !selected && analysisOutput(mass.data, "structure.mass_properties")?.summary.center_of_mass;
  const com: V3 | null = c ? [c.x, c.y, c.z] : null;

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
          />
        )}
      </QueryState>
    </>
  );
}
