import { Link } from "react-router";
import { useDesign } from "@/api/queries";
import { systemOrder, systemTitle } from "@/features/systems/systems";
import { useT } from "@/i18n";
import { DataTable } from "@/kit/DataTable";
import { Empty } from "@/kit/Empty";
import { Loading, SkeletonPanel } from "@/kit/Loading";
import { Panel } from "@/kit/Panel";
import { Td } from "@/kit/Td";
import { Th } from "@/kit/Th";
import { PageBody } from "@/shell/PageBody";
import { PageHeader } from "@/shell/PageHeader";

/** ミッション / 解析ケース: cases/ の一覧。各系のページをそのケースで開ける */
export function CasesPage() {
  const t = useT();
  const m = t.mission;
  const design = useDesign();
  const d = design.data;
  return (
    <>
      <PageHeader eyebrow={m.eyebrow} title={m.cases} subtitle={m.casesSubtitle} />
      <PageBody>
        {!d ? (
          design.isError ? (
            <Empty>{t.common.cannotLoadDesign(design.error.message)}</Empty>
          ) : (
            <Loading>
              <SkeletonPanel lines={5} />
            </Loading>
          )
        ) : d.cases.length === 0 ? (
          <Empty>{m.noCases}</Empty>
        ) : (
          <Panel pad={false}>
            <DataTable
              head={
                <>
                  <Th>ID</Th>
                  <Th>{m.name}</Th>
                  <Th>{m.description}</Th>
                  <Th>{m.modes}</Th>
                  <Th>{m.file}</Th>
                  <Th>{m.openWith}</Th>
                </>
              }
            >
              {d.cases.map((c) => (
                <tr key={c.id}>
                  <Td mono>{c.id}</Td>
                  <Td>{c.label}</Td>
                  <Td className="text-ink-2">{c.description ?? "—"}</Td>
                  <Td mono>{c.modes.join(" → ")}</Td>
                  <Td mono className="text-ink-4">
                    {c.file}
                  </Td>
                  <Td>
                    <span className="flex flex-wrap gap-1">
                      {systemOrder.map((s) => (
                        <Link
                          key={s}
                          to={`/systems/${s}?case=${encodeURIComponent(c.id)}`}
                          className="rounded-chip border border-line px-1.5 py-0.5 text-[11px] hover:bg-panel-2"
                        >
                          {systemTitle(s, d.defs)}
                        </Link>
                      ))}
                    </span>
                  </Td>
                </tr>
              ))}
            </DataTable>
          </Panel>
        )}
      </PageBody>
    </>
  );
}
