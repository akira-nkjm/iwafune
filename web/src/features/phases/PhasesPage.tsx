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
import { PhaseOverridesDialog } from "./PhaseOverrides";
import { emptyPhaseText } from "./phase-form";

/** 開いているダイアログ（追加・編集・削除） */
type Open =
  | { kind: "add" }
  | { kind: "edit"; text: PhaseText }
  | { kind: "remove"; id: string }
  | { kind: "overrides"; id: string }
  | null;

/**
 * 前提条件 / フェーズ: phases/ の一覧（並びは API が `position` で返す）と、追加・編集・削除。
 * 編集は `fune phase add|set|rm` と同じ引数を service に送る。軌道と姿勢則の上書きは「上書きを編集」のダイアログで編集する
 * （`PhaseOverridesDialog`。書き込みは `fune phase copy-orbit|unset-orbit|unset-law|set-field` と同じ）。URL の `?phase=<ID>` で選んだ行を強調する（診断のリンクが開く）。
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
      {open?.kind === "overrides" && (
        <PhaseOverridesDialog phaseId={open.id} onOpenChange={close} />
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
      {/* 長くなる欄（条件・モード・上書き）は折り返す。折り返さないと表が横にはみ出し、右端の編集と削除が隠れる */}
      <Td className="max-w-56 whitespace-normal text-ink-2">{p.start ?? "—"}</Td>
      <Td className="max-w-56 whitespace-normal text-ink-2">{p.end ?? "—"}</Td>
      <Td mono>{p.duration?.display ?? "—"}</Td>
      <Td mono className="max-w-48 whitespace-normal">
        {p.modes.length > 0 ? p.modes.join(" ") : "—"}
      </Td>
      <Td className="max-w-64 whitespace-normal">
        <Overrides p={p} onEdit={() => onOpen({ kind: "overrides", id: p.id })} />
      </Td>
      <Td mono className="max-w-48 whitespace-normal text-ink-3">
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

/**
 * 軌道と姿勢則の上書きの要約（軌道を置き換えているか、姿勢則を上書きしているモード。上書きが無ければ「なし」）と、
 * 編集のダイアログを開くボタン。値は表に並べず、ダイアログで見る（並べると行が長くなり、キーの名前も生で出る）
 */
function Overrides({ p, onEdit }: { p: PhaseView; onEdit: () => void }) {
  const t = useT().phases;
  const none = !p.orbit_overridden && p.laws.length === 0;
  return (
    <span className="flex flex-col items-start gap-0.5">
      {none && <span className="text-ink-4">{t.none}</span>}
      {p.orbit_overridden && (
        <span>{p.orbit_from ? t.orbitFrom(p.orbit_from) : t.orbitReplaced}</span>
      )}
      {p.laws.length > 0 && <span>{t.lawFor(p.laws.map((l) => l.mode).join(", "))}</span>}
      <Button size="sm" variant="ghost" onClick={onEdit}>
        <Pencil size={13} />
        {t.editOverrides}
      </Button>
    </span>
  );
}
