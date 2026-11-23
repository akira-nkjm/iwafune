import { Pencil, Plus, Trash2 } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useSearchParams } from "react-router";
import { useDesign } from "@/api/queries";
import type { PhaseText, PhaseView } from "@/api/types";
import { useT } from "@/i18n";
import { Empty } from "@/kit/feedback/Empty";
import { SkeletonPanel } from "@/kit/feedback/Loading";
import { QueryState } from "@/kit/feedback/QueryState";
import { Panel } from "@/kit/panel/Panel";
import { DataTable } from "@/kit/table/DataTable";
import { Td } from "@/kit/table/Td";
import { Th } from "@/kit/table/Th";
import { SEARCH } from "@/lib/nav";
import { OpDialog } from "@/ops/OpDialog";
import { PageBody } from "@/shell/PageBody";
import { PageHeader } from "@/shell/PageHeader";
import { Button } from "@/ui/button";
import { PhaseDialog } from "./PhaseDialog";
import { emptyPhaseText } from "./phase-form";

/** 開いているダイアログ（追加・編集・削除） */
type Open =
  | { kind: "add" }
  | { kind: "edit"; text: PhaseText }
  | { kind: "remove"; id: string }
  | null;

/**
 * 前提条件 / フェーズ: phases/ の一覧（並びは API が `position` で返す）と、追加・編集・削除。
 * 編集は `fune phase add|set|rm` と同じ引数を service に送る。軌道と姿勢則の上書きは表示だけ
 * （値の編集は `fune phase set-field`）。URL の `?phase=<ID>` で選んだ行を強調する（診断のリンクが開く）。
 */
export function PhasesPage() {
  const m = useT().conditions;
  const t = useT().phases;
  const design = useDesign();
  const [params] = useSearchParams();
  const selected = params.get(SEARCH.phase);
  const [open, setOpen] = useState<Open>(null);
  const close = (o: boolean) => !o && setOpen(null);
  return (
    <>
      <PageHeader
        eyebrow={m.eyebrow}
        title={t.title}
        subtitle={t.subtitle}
        actions={
          <Button variant="outline" onClick={() => setOpen({ kind: "add" })}>
            <Plus size={14} />
            {t.add}
          </Button>
        }
      />
      <PageBody>
        <QueryState query={design} skeleton={<SkeletonPanel lines={5} />}>
          {(d) =>
            d.phases.length === 0 ? (
              <Empty>{t.empty}</Empty>
            ) : (
              <>
                <PhasesTable phases={d.phases} selected={selected} onOpen={setOpen} />
                <p className="mt-3 text-ink-3 text-xs">{t.overridesNote}</p>
              </>
            )
          }
        </QueryState>
      </PageBody>
      {open?.kind === "add" && (
        <PhaseDialog initial={emptyPhaseText()} adding onOpenChange={close} />
      )}
      {open?.kind === "edit" && (
        <PhaseDialog initial={open.text} adding={false} onOpenChange={close} />
      )}
      {open?.kind === "remove" && (
        <OpDialog
          open
          destructive
          onOpenChange={close}
          title={t.removeDialog.title(open.id)}
          description={t.removeDialog.description}
          ops={[{ op: "remove_phase", id: open.id }]}
          summary={t.removeDialog.summary(open.id)}
          confirmLabel={t.removeDialog.confirm}
        />
      )}
    </>
  );
}

function PhasesTable({
  phases,
  selected,
  onOpen,
}: {
  phases: PhaseView[];
  selected: string | null;
  onOpen: (o: Open) => void;
}) {
  const t = useT().phases;
  return (
    <Panel pad={false}>
      <DataTable
        head={
          <>
            <Th>{t.position}</Th>
            <Th>{t.id}</Th>
            <Th>{t.name}</Th>
            <Th>{t.start}</Th>
            <Th>{t.end}</Th>
            <Th>{t.duration}</Th>
            <Th>{t.modes}</Th>
            <Th>{t.overrides}</Th>
            <Th>{t.cases}</Th>
            <Th>{t.file}</Th>
            <Th />
          </>
        }
      >
        {phases.map((p) => (
          <PhaseRow key={p.id} p={p} selected={p.id === selected} onOpen={onOpen} />
        ))}
      </DataTable>
    </Panel>
  );
}

function PhaseRow({
  p,
  selected,
  onOpen,
}: {
  p: PhaseView;
  selected: boolean;
  onOpen: (o: Open) => void;
}) {
  const t = useT().phases;
  // 診断のリンクで選ばれた行は、見える位置へ出す
  const ref = useRef<HTMLTableRowElement>(null);
  useEffect(() => {
    if (selected) ref.current?.scrollIntoView({ block: "nearest" });
  }, [selected]);
  return (
    <tr
      ref={ref}
      aria-current={selected || undefined}
      className={selected ? "bg-panel-2" : undefined}
    >
      <Td mono>{p.position}</Td>
      <Td mono>{p.id}</Td>
      <Td>{p.label}</Td>
      <Td className="text-ink-2">{p.start ?? "—"}</Td>
      <Td className="text-ink-2">{p.end ?? "—"}</Td>
      <Td mono>{p.duration?.display ?? "—"}</Td>
      <Td mono>{p.modes.length > 0 ? p.modes.join(" ") : "—"}</Td>
      <Td>
        <Overrides p={p} />
      </Td>
      <Td mono className="text-ink-3">
        {p.cases.length > 0 ? p.cases.join(" ") : "—"}
      </Td>
      <Td mono className="text-ink-4">
        {p.file}
      </Td>
      <Td>
        <span className="flex gap-1">
          <Button
            size="sm"
            variant="ghost"
            aria-label={t.edit}
            onClick={() => onOpen({ kind: "edit", text: p.text })}
          >
            <Pencil size={13} />
          </Button>
          <Button
            size="sm"
            variant="ghost"
            aria-label={t.remove}
            onClick={() => onOpen({ kind: "remove", id: p.id })}
          >
            <Trash2 size={13} />
          </Button>
        </span>
      </Td>
    </tr>
  );
}

/** 軌道と姿勢則の上書き（API が整形した値の表示。上書きが無ければ「なし」） */
function Overrides({ p }: { p: PhaseView }) {
  const t = useT().phases;
  if (p.orbit.length === 0 && p.law.length === 0)
    return <span className="text-ink-4">{t.none}</span>;
  const line = (label: string, fields: PhaseView["orbit"]) =>
    fields.length > 0 && (
      <span>
        <span className="text-ink-3">{label}</span>{" "}
        {fields.map((f) => `${f.key} ${f.display}`).join(", ")}
      </span>
    );
  return (
    <span className="flex flex-col gap-0.5">
      {line(t.orbit, p.orbit)}
      {line(t.law, p.law)}
    </span>
  );
}
