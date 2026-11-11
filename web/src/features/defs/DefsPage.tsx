import { useParams } from "react-router";
import { useDesign } from "@/api/queries";
import { useT } from "@/i18n";
import { SkeletonTable } from "@/kit/feedback/Loading";
import { QueryState } from "@/kit/feedback/QueryState";
import { PageTabs } from "@/kit/selection/PageTabs";
import { paths } from "@/lib/nav";
import { PageBody } from "@/shell/PageBody";
import { PageHeader } from "@/shell/PageHeader";
import { AttributesTab } from "./AttributesTab";
import { KindsTab } from "./KindsTab";
import { UnitsTab } from "./UnitsTab";

/** 設計 / 単位と定義: 単位の一覧・属性定義・種別定義 */
export function DefsPage() {
  const { tab } = useParams();
  const design = useDesign();
  const td = useT().defs;
  const d = design.data;
  const t = tab === "attributes" || tab === "kinds" ? tab : "units";
  return (
    <>
      <PageHeader eyebrow={td.eyebrow} title={td.title} subtitle={td.subtitle} />
      <PageTabs
        tabs={[
          { to: paths.defs(), label: td.tabUnits },
          {
            to: paths.defs("attributes"),
            label: td.tabAttributes,
            count: d?.defs.attributes.length,
          },
          { to: paths.defs("kinds"), label: td.tabKinds, count: d?.defs.kinds.length },
        ]}
      />
      <PageBody>
        {t === "units" ? (
          <UnitsTab design={d} />
        ) : (
          <QueryState query={design} skeleton={<SkeletonTable rows={8} cols={5} />}>
            {(d) => (t === "attributes" ? <AttributesTab design={d} /> : <KindsTab design={d} />)}
          </QueryState>
        )}
      </PageBody>
    </>
  );
}
