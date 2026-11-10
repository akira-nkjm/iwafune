import { useDesign } from "@/api/queries";
import type { DesignView } from "@/api/types";
import { useT } from "@/i18n";
import { ChipLink } from "@/kit/Chip";
import { DataTable } from "@/kit/DataTable";
import { Empty } from "@/kit/Empty";
import { SkeletonPanel } from "@/kit/Loading";
import { Panel } from "@/kit/Panel";
import { QueryState } from "@/kit/QueryState";
import { Td } from "@/kit/Td";
import { Th } from "@/kit/Th";
import { systemTitle } from "@/lib/design";
import { paths } from "@/lib/nav";
import { PageBody } from "@/shell/PageBody";
import { PageHeader } from "@/shell/PageHeader";

/** ミッション / 解析ケース: cases/ の一覧。各系のページをそのケースで開ける */
export function CasesPage() {
  const m = useT().mission;
  const design = useDesign();
  return (
    <>
      <PageHeader eyebrow={m.eyebrow} title={m.cases} subtitle={m.casesSubtitle} />
      <PageBody>
        <QueryState query={design} skeleton={<SkeletonPanel lines={5} />}>
          {(d) => (d.cases.length === 0 ? <Empty>{m.noCases}</Empty> : <CasesTable design={d} />)}
        </QueryState>
      </PageBody>
    </>
  );
}

/** ケースの表。最後の列は、そのケースで開く系のページへのリンク */
function CasesTable({ design: d }: { design: DesignView }) {
  const m = useT().mission;
  return (
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
                {d.defs.systems.map((s) => (
                  <ChipLink key={s} to={paths.system(s, { kase: c.id })} mono={false}>
                    {systemTitle(s, d.defs)}
                  </ChipLink>
                ))}
              </span>
            </Td>
          </tr>
        ))}
      </DataTable>
    </Panel>
  );
}
