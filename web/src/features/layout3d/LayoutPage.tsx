import { useQuery } from "@tanstack/react-query";
import { analysisOutput } from "@/api/analysis-output";
import { q, useDesign } from "@/api/queries";
import { useT } from "@/i18n";
import { SkeletonBlock } from "@/kit/feedback/Loading";
import { QueryState } from "@/kit/feedback/QueryState";
import { PageHeader } from "@/shell/PageHeader";
import { LazyLayout3D } from "./LazyLayout3D";
import type { V3 } from "./scene";

/** 設計 / 配置。質量特性が未取得・入力不足でも配置は表示する。 */
export function LayoutPage() {
  const design = useDesign();
  const mass = useQuery(q.analysis("structure.mass_properties", null));
  const t = useT();
  const c = analysisOutput(mass.data, "structure.mass_properties")?.summary.center_of_mass;
  const com: V3 | null = c ? [c.x, c.y, c.z] : null;
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
          {(d) => <LazyLayout3D variant="placement" design={d} com={com} />}
        </QueryState>
      </div>
    </>
  );
}
