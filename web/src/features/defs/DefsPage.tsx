import { useParams } from "react-router";
import { useDesign } from "@/api/queries";
import { useT } from "@/i18n";
import { Empty } from "@/kit/Empty";
import { Loading, SkeletonTable } from "@/kit/Loading";
import { PageTabs } from "@/kit/PageTabs";
import { PageBody } from "@/shell/PageBody";
import { PageHeader } from "@/shell/PageHeader";
import { AttributesTab } from "./AttributesTab";
import { KindsTab } from "./KindsTab";
import { UnitsTab } from "./UnitsTab";

/** 設計 / 単位と定義: 単位の一覧・属性定義・種別定義 */
export function DefsPage() {
  const { tab } = useParams();
  const design = useDesign();
  const tr = useT();
  const td = tr.defs;
  const d = design.data;
  const t = tab === "attributes" || tab === "kinds" ? tab : "units";
  return (
    <>
      <PageHeader eyebrow={td.eyebrow} title={td.title} subtitle={td.subtitle} />
      <PageTabs
        tabs={[
          { to: "/design/defs", label: td.tabUnits },
          {
            to: "/design/defs/attributes",
            label: td.tabAttributes,
            count: d?.defs.attributes.length,
          },
          { to: "/design/defs/kinds", label: td.tabKinds, count: d?.defs.kinds.length },
        ]}
      />
      <PageBody>
        {t === "units" ? (
          <UnitsTab design={d} />
        ) : !d ? (
          design.isError ? (
            <Empty>{tr.common.cannotLoadDesign(design.error.message)}</Empty>
          ) : (
            <Loading>
              <SkeletonTable rows={8} cols={5} />
            </Loading>
          )
        ) : t === "attributes" ? (
          <AttributesTab design={d} />
        ) : (
          <KindsTab design={d} />
        )}
      </PageBody>
    </>
  );
}
