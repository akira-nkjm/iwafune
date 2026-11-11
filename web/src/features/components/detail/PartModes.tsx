import { Link } from "react-router";
import type { ModeTable, StateAssign } from "@/api/types";
import { savedCellText } from "@/features/modes/matrix/matrix";
import { useT } from "@/i18n";
import { PropertyList } from "@/kit/detail/PropertyList";
import { PropertyRow } from "@/kit/detail/PropertyRow";
import { nodeName } from "@/lib/design";
import { paths } from "@/lib/nav";

/** この部品がモードごとにどの状態か（編集はモードの表で行う） */
export function PartModes({
  table,
  part,
  cells,
}: {
  /** モード表（列の見出しと、保存済みのセルの文言） */
  table: ModeTable;
  part: string;
  /** この部品のモードごとの状態（`ModeRow.cells` と同じ形。GET /parts/{id} の `modes`） */
  cells: Record<string, StateAssign>;
}) {
  const all = useT();
  const t = all.components.partModes;
  const headers = table.modes;
  const row = table.rows.find((r) => r.part === part);
  if (headers.length === 0) return <p className="px-1 text-ink-3 text-xs">{t.none}</p>;
  return (
    <PropertyList>
      {headers.map((m) => {
        const s = cells[m.id];
        // 割合の `%` はサーバー（モード表の shares）が整形したもの。並べ方はモード表と同じ i18n
        const shown = (row && savedCellText(row, m.id, all.modes)) ?? s?.state;
        const name = nodeName(m);
        return (
          <PropertyRow
            key={m.id}
            label={
              name === m.id ? (
                <span className="font-mono">{m.id}</span>
              ) : (
                <>
                  {name} <span className="font-mono text-ink-4">{m.id}</span>
                </>
              )
            }
          >
            <span className={s ? "font-mono" : "text-warn-ink"}>{s ? shown : t.unassigned}</span>
          </PropertyRow>
        );
      })}
      <div className="px-3 py-1 text-right text-[11px]">
        <Link to={paths.modes()} className="text-accent-ink hover:underline">
          {t.edit}
        </Link>
      </div>
    </PropertyList>
  );
}
