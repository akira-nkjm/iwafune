import { Pencil, Plus, Trash2 } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useSearchParams } from "react-router";
import { useEnumLabel } from "@/api/labels";
import { useDesign } from "@/api/queries";
import type { DesignView, ManeuverText, ManeuverView } from "@/api/types";
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
import { Button } from "@/ui/button";
import { ManeuverDialog } from "./ManeuverDialog";
import { emptyManeuverText } from "./maneuver-form";

/** 開いているダイアログ（追加・編集・削除） */
type Open =
  | { kind: "add" }
  | { kind: "edit"; text: ManeuverText }
  | { kind: "remove"; id: string }
  | null;

/**
 * 推進のページのマヌーバのタブ: maneuvers/ の一覧（実行の順は API が `order` で返す）と、追加・編集・削除。
 * 編集は `fune maneuver add|set|rm` と同じ引数を service に送る。ΔV などの結果は同じページの別のタブ。
 * URL の `?maneuver=<ID>` で選んだ行を強調する（診断のリンクが開く）。表はページの本文の中でスクロールし、見出しを止める。
 */
export function ManeuversTab() {
  const t = useT().maneuvers;
  const design = useDesign();
  const [params] = useSearchParams();
  const selected = params.get(SEARCH.maneuver);
  const [open, setOpen] = useState<Open>(null);
  const close = (o: boolean) => !o && setOpen(null);
  return (
    <>
      <div className="flex shrink-0 items-center justify-between gap-3 px-6 pt-4 pb-3">
        <p className="text-ink-3 text-sm">{t.subtitle}</p>
        <Button variant="outline" onClick={() => setOpen({ kind: "add" })}>
          <Plus size={14} />
          {t.add}
        </Button>
      </div>
      <div className="flex min-h-0 flex-1 flex-col px-6 pb-4">
        <QueryState query={design} skeleton={<SkeletonPanel lines={5} />}>
          {(d) =>
            d.maneuvers.length === 0 ? (
              <Empty>{t.empty}</Empty>
            ) : (
              <ManeuversTable design={d} selected={selected} onOpen={setOpen} />
            )
          }
        </QueryState>
      </div>
      {design.data && open?.kind === "add" && (
        <ManeuverDialog
          design={design.data}
          initial={emptyManeuverText()}
          adding
          onOpenChange={close}
        />
      )}
      {design.data && open?.kind === "edit" && (
        <ManeuverDialog
          design={design.data}
          initial={open.text}
          adding={false}
          onOpenChange={close}
        />
      )}
      {open?.kind === "remove" && (
        <OpDialog
          open
          destructive
          onOpenChange={close}
          title={t.removeDialog.title(open.id)}
          description={t.removeDialog.description}
          ops={[{ op: "remove_maneuver", id: open.id }]}
          summary={t.removeDialog.summary(open.id)}
          confirmLabel={t.removeDialog.confirm}
        />
      )}
    </>
  );
}

/** マヌーバの表（実行の順）。目標の量は API が整形した表示文字列 */
function ManeuversTable({
  design: d,
  selected,
  onOpen,
}: {
  design: DesignView;
  selected: string | null;
  onOpen: (o: Open) => void;
}) {
  const t = useT().maneuvers;
  const label = useEnumLabel();
  return (
    <Panel pad={false} className="flex min-h-0 flex-col">
      <DataTable
        stickyHead
        className="min-h-0 flex-1"
        head={
          <>
            <Th>{t.order}</Th>
            <Th>{t.id}</Th>
            <Th>{t.name}</Th>
            <Th>{t.category}</Th>
            <Th>{t.method}</Th>
            <Th>{t.thruster}</Th>
            <Th>{t.after}</Th>
            <Th>{t.target}</Th>
            <Th>{t.constraints}</Th>
            <Th>{t.file}</Th>
            <Th />
          </>
        }
      >
        {d.maneuvers.map((m) => (
          <ManeuverRow
            key={m.id}
            m={m}
            selected={m.id === selected}
            label={label}
            onOpen={onOpen}
          />
        ))}
      </DataTable>
    </Panel>
  );
}

function ManeuverRow({
  m,
  selected,
  label,
  onOpen,
}: {
  m: ManeuverView;
  selected: boolean;
  label: (ty: string, value: string) => string;
  onOpen: (o: Open) => void;
}) {
  const t = useT().maneuvers;
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
      <Td mono>{m.order}</Td>
      <Td mono>{m.id}</Td>
      <Td>{m.label}</Td>
      <Td>{label("maneuver_category", m.category)}</Td>
      <Td>{label("maneuver_method", m.method)}</Td>
      <Td mono>{m.thruster ?? "—"}</Td>
      <Td mono className="text-ink-3">
        {m.after ?? "—"}
      </Td>
      <Td>
        <span className="flex flex-col gap-0.5">
          {m.body && (
            <span>
              <span className="text-ink-3">{t.targetKeys.body}</span>{" "}
              {label("central_body", m.body)}
            </span>
          )}
          {m.target.map((x) => (
            <span key={x.key}>
              <span className="text-ink-3">{t.targetKeys[x.key] ?? x.key}</span> {x.value.display}
            </span>
          ))}
          {m.delta_v && (
            <span>
              <span className="text-ink-3">{t.targetKeys.delta_v}</span> {m.delta_v.display}
            </span>
          )}
          {m.text.depart && m.text.arrive && (
            <span>
              <span className="text-ink-3">{t.targetKeys.transfer}</span> {m.text.depart} →{" "}
              {m.text.arrive}
              {m.text.departure_by && <> ({label("departure_by", m.text.departure_by)})</>}
            </span>
          )}
        </span>
      </Td>
      <Td mono className="text-ink-3">
        {m.constraints.length > 0 ? m.constraints.join(", ") : t.none}
      </Td>
      <Td mono className="text-ink-4">
        {m.file}
      </Td>
      <Td>
        <span className="flex gap-1">
          <Button
            size="sm"
            variant="ghost"
            aria-label={t.edit}
            onClick={() => onOpen({ kind: "edit", text: m.text })}
          >
            <Pencil size={13} />
          </Button>
          <Button
            size="sm"
            variant="ghost"
            aria-label={t.remove}
            onClick={() => onOpen({ kind: "remove", id: m.id })}
          >
            <Trash2 size={13} />
          </Button>
        </span>
      </Td>
    </tr>
  );
}
