import { Pencil, Plus, Trash2 } from "lucide-react";
import { useState } from "react";
import { useEnumLabel } from "@/api/labels";
import { useDesign } from "@/api/queries";
import type { DesignView, ManeuverText, ManeuverView } from "@/api/types";
import { useT } from "@/i18n";
import { Empty } from "@/kit/feedback/Empty";
import { SkeletonPanel } from "@/kit/feedback/Loading";
import { QueryState } from "@/kit/feedback/QueryState";
import { Panel } from "@/kit/panel/Panel";
import { ChipLink } from "@/kit/selection/Chip";
import { DataTable } from "@/kit/table/DataTable";
import { Td } from "@/kit/table/Td";
import { Th } from "@/kit/table/Th";
import { paths } from "@/lib/nav";
import { OpDialog } from "@/ops/OpDialog";
import { PageBody } from "@/shell/PageBody";
import { PageHeader } from "@/shell/PageHeader";
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
 * 設計 / マヌーバ: maneuvers/ の一覧（実行の順は API が `order` で返す）と、追加・編集・削除。
 * 編集は `fune maneuver add|set|rm` と同じ引数を service に送る。ΔV などの結果は推進系のページ
 */
export function ManeuversPage() {
  const t = useT().maneuvers;
  const design = useDesign();
  const [open, setOpen] = useState<Open>(null);
  const close = (o: boolean) => !o && setOpen(null);
  return (
    <>
      <PageHeader
        eyebrow={t.eyebrow}
        title={t.title}
        subtitle={t.subtitle}
        actions={
          <>
            <ChipLink to={paths.system("propulsion")} mono={false}>
              {t.results}
            </ChipLink>
            <Button variant="outline" onClick={() => setOpen({ kind: "add" })}>
              <Plus size={14} />
              {t.add}
            </Button>
          </>
        }
      />
      <PageBody>
        <QueryState query={design} skeleton={<SkeletonPanel lines={5} />}>
          {(d) =>
            d.maneuvers.length === 0 ? (
              <Empty>{t.empty}</Empty>
            ) : (
              <ManeuversTable design={d} onOpen={setOpen} />
            )
          }
        </QueryState>
      </PageBody>
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
function ManeuversTable({ design: d, onOpen }: { design: DesignView; onOpen: (o: Open) => void }) {
  const t = useT().maneuvers;
  const label = useEnumLabel();
  return (
    <Panel pad={false}>
      <DataTable
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
          <ManeuverRow key={m.id} m={m} label={label} onOpen={onOpen} />
        ))}
      </DataTable>
    </Panel>
  );
}

function ManeuverRow({
  m,
  label,
  onOpen,
}: {
  m: ManeuverView;
  label: (ty: string, value: string) => string;
  onOpen: (o: Open) => void;
}) {
  const t = useT().maneuvers;
  return (
    <tr>
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
          {m.target.map((x) => (
            <span key={x.key}>
              <span className="text-ink-3">{t.targetKeys[x.key] ?? x.key}</span> {x.value.display}
            </span>
          ))}
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
