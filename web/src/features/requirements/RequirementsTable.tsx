import { Pencil, Trash2 } from "lucide-react";
import { useEnumLabel } from "@/api/labels";
import type { DesignView, RequirementText, RequirementView } from "@/api/types";
import { localized, useT } from "@/i18n";
import { Empty } from "@/kit/feedback/Empty";
import { SkeletonTable } from "@/kit/feedback/Loading";
import { type QueryLike, QueryState } from "@/kit/feedback/QueryState";
import { Panel } from "@/kit/panel/Panel";
import { DataTable } from "@/kit/table/DataTable";
import { Td } from "@/kit/table/Td";
import { Th } from "@/kit/table/Th";
import { Button } from "@/ui/button";
import { VerifiedBy } from "./VerifiedBy";

/** 一覧の行から開くダイアログ */
type OpenFromRow = { kind: "edit"; text: RequirementText } | { kind: "remove"; id: string };

/** 要求の一覧（requirements/ の ID 順）。制約は API が整形した文 */
export function RequirementsTable({
  design,
  onOpen,
}: {
  design: QueryLike<DesignView>;
  onOpen: (o: OpenFromRow) => void;
}) {
  const t = useT().requirements;
  return (
    <QueryState query={design} skeleton={<SkeletonTable rows={6} cols={7} />}>
      {(d) =>
        d.requirements.length === 0 ? (
          <Empty>{t.empty}</Empty>
        ) : (
          <Panel pad={false}>
            <DataTable
              head={
                <>
                  <Th>{t.id}</Th>
                  <Th>{t.statement}</Th>
                  <Th>{t.constraint}</Th>
                  <Th>{t.source}</Th>
                  <Th>{t.verify}</Th>
                  <Th />
                </>
              }
            >
              {d.requirements.map((r) => (
                <RequirementRow key={r.id} r={r} onOpen={onOpen} />
              ))}
            </DataTable>
          </Panel>
        )
      }
    </QueryState>
  );
}

function RequirementRow({ r, onOpen }: { r: RequirementView; onOpen: (o: OpenFromRow) => void }) {
  const t = useT().requirements;
  const label = useEnumLabel();
  // 書かれていない欄は API の JSON に出ない（空の列・false は省く）
  const named = r.quantify.cases ?? [];
  const cases = r.quantify.all_cases
    ? t.allCases
    : named.length > 0
      ? named.join(", ")
      : t.defaultCase;
  return (
    <tr>
      <Td mono className="align-top">
        <span title={r.file}>{r.id}</span>
      </Td>
      {/* 文は折り返す（表のセルは既定で折り返さないので、長い文が隣の列に重なる） */}
      <Td className="w-[40%] min-w-64 whitespace-normal align-top">
        <span className="block font-medium">{r.label}</span>
        <span className="block text-ink-3">{localized(r.statement, r.statement_en)}</span>
      </Td>
      {/* 制約の下に、対象の量を添える */}
      <Td className="align-top">
        <span className="block">
          <span className="text-ink-3">{label("constraint_kind", r.constraint.kind)}</span>{" "}
          <span className="num">{r.constraint.text}</span>
        </span>
        <span className="block text-ink-3 text-xs">{r.quantity}</span>
      </Td>
      <Td mono className="align-top">
        {r.source ?? t.noSource}
      </Td>
      {/* 確かめる先の下に、成り立つべきケースを添える */}
      <Td className="align-top">
        <VerifiedBy verify={r.verify} />
        <span className="block whitespace-normal text-ink-3 text-xs">{cases}</span>
      </Td>
      <Td className="align-top">
        <span className="flex gap-1">
          <Button
            size="sm"
            variant="ghost"
            aria-label={t.edit}
            onClick={() => onOpen({ kind: "edit", text: r.text })}
          >
            <Pencil size={13} />
          </Button>
          <Button
            size="sm"
            variant="ghost"
            aria-label={t.remove}
            onClick={() => onOpen({ kind: "remove", id: r.id })}
          >
            <Trash2 size={13} />
          </Button>
        </span>
      </Td>
    </tr>
  );
}
