import { Pencil } from "lucide-react";
import { useState } from "react";
import { useDesign } from "@/api/queries";
import type { CaseView, DesignView } from "@/api/types";
import { useT } from "@/i18n";
import { Empty } from "@/kit/feedback/Empty";
import { SkeletonPanel } from "@/kit/feedback/Loading";
import { QueryState } from "@/kit/feedback/QueryState";
import { Panel } from "@/kit/panel/Panel";
import { ChipLink } from "@/kit/selection/Chip";
import { DataTable } from "@/kit/table/DataTable";
import { Td } from "@/kit/table/Td";
import { Th } from "@/kit/table/Th";
import { systemTitle } from "@/lib/design";
import { paths } from "@/lib/nav";
import { PageBody } from "@/shell/PageBody";
import { PageHeader } from "@/shell/PageHeader";
import { Button } from "@/ui/button";
import { AddCaseButton } from "./AddCaseButton";
import { CaseEpochDialog } from "./CaseEpochDialog";
import { CasePhaseDialog } from "./CasePhaseDialog";
import { CasesFromGeometryButton } from "./CasesFromGeometryDialog";

/** 前提条件 / 解析ケース: cases/ の一覧。各系のページをそのケースで開ける */
export function CasesPage() {
  const m = useT().conditions;
  const design = useDesign();
  return (
    <>
      <PageHeader
        eyebrow={m.eyebrow}
        title={m.cases}
        subtitle={m.casesSubtitle}
        actions={
          <>
            <AddCaseButton design={design.data} />
            <CasesFromGeometryButton design={design.data} />
          </>
        }
      />
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
  const m = useT().conditions;
  const [editing, setEditing] = useState<CaseView | null>(null);
  const [editingEpoch, setEditingEpoch] = useState<CaseView | null>(null);
  return (
    <Panel pad={false}>
      <DataTable
        head={
          <>
            <Th>ID</Th>
            <Th>{m.name}</Th>
            <Th>{m.description}</Th>
            <Th>{m.modes}</Th>
            <Th>{m.phase}</Th>
            <Th>{m.epoch}</Th>
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
            <Td mono>
              <span className="flex items-center gap-1">
                {c.phase ?? "—"}
                <Button
                  size="sm"
                  variant="ghost"
                  aria-label={m.setPhase}
                  onClick={() => setEditing(c)}
                >
                  <Pencil size={13} />
                </Button>
              </span>
            </Td>
            <Td mono>
              <span className="flex items-center gap-1">
                {c.epoch ?? "—"}
                <Button
                  size="sm"
                  variant="ghost"
                  aria-label={m.setEpoch}
                  onClick={() => setEditingEpoch(c)}
                >
                  <Pencil size={13} />
                </Button>
              </span>
            </Td>
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
      {editing && (
        <CasePhaseDialog
          kase={editing}
          phases={d.phases}
          onOpenChange={(open) => !open && setEditing(null)}
        />
      )}
      {editingEpoch && (
        <CaseEpochDialog
          kase={editingEpoch}
          onOpenChange={(open) => !open && setEditingEpoch(null)}
        />
      )}
    </Panel>
  );
}
