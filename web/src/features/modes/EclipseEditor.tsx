import { useState } from "react";
import { useCommandText } from "@/api/ops";
import type { ModeTable } from "@/api/types";
import { useT } from "@/i18n";
import { NativeSelect } from "@/kit/forms/NativeSelect";
import { Button } from "@/ui/button";
import { type CellText, cellText } from "./draft/draft";
import { ModeCellForm } from "./matrix/ModeCellForm";

/** 蝕の上書きを別の編集欄で表示する。空なら日照時を継承し、保存時は CLI と同じ service を通す。 */
export function EclipseEditor({ table }: { table: ModeTable }) {
  const t = useT().modes;
  const [mode, setMode] = useState(table.modes[0]?.id ?? "");
  const [part, setPart] = useState(table.rows[0]?.part ?? "");
  const row = table.rows.find((r) => r.part === part);
  const saved = row?.eclipse_cells[mode];
  return (
    <details className="mx-6 mb-6 rounded-card border border-line bg-panel p-4">
      <summary className="cursor-pointer font-semibold text-sm">{t.eclipse}</summary>
      <EclipseAssignments
        table={table}
        onSelect={(mode, part) => {
          setMode(mode);
          setPart(part);
        }}
      />
      <div className="mt-3 flex flex-wrap gap-4">
        <NativeSelect
          aria-label={t.title}
          value={mode}
          onChange={(e) => setMode(e.target.value)}
          options={table.modes.map((m) => ({ value: m.id, label: m.label }))}
        />
        <NativeSelect
          aria-label={t.part}
          value={part}
          onChange={(e) => setPart(e.target.value)}
          options={table.rows.map((r) => ({ value: r.part, label: r.part }))}
        />
        {row && (
          <EclipseCell
            key={`${mode}-${part}-${JSON.stringify(saved)}`}
            mode={mode}
            part={part}
            states={row.states}
            initial={saved ? cellText(saved) : null}
            inherited={row.cells[mode]?.state}
          />
        )}
      </div>
    </details>
  );
}

function EclipseCell({
  mode,
  part,
  states,
  initial,
  inherited,
}: {
  mode: string;
  part: string;
  states: string[];
  initial: CellText | null;
  inherited?: string;
}) {
  const t = useT().modes;
  const [value, setValue] = useState(initial);
  const edit = useCommandText();
  return (
    <div className="w-72 space-y-3">
      {!value && (
        <p className="text-ink-3 text-xs">
          {t.inherit}: {inherited ?? "—"}
        </p>
      )}
      <ModeCellForm states={states} value={value} onChange={setValue} />
      <Button
        disabled={edit.isPending}
        onClick={() =>
          edit.mutate({
            command: {
              command: "mode_set",
              cells: [
                {
                  mode,
                  part,
                  state: value?.state ?? null,
                  duty: value?.duty ?? {},
                  active: value?.active ?? null,
                  spare_state: value?.spare_state ?? null,
                  eclipse: true,
                },
              ],
            },
            summary: `${t.eclipse}: ${mode} · ${part}`,
          })
        }
      >
        {t.saveEclipse}
      </Button>
      {edit.error && (
        <p role="alert" className="text-neg-ink text-xs">
          {edit.error.message}
        </p>
      )}
    </div>
  );
}

/** 蝕で使う状態の行列。値は API の割り当てを表示するだけで、評価は行わない。 */
function EclipseAssignments({
  table,
  onSelect,
}: {
  table: ModeTable;
  onSelect: (mode: string, part: string) => void;
}) {
  const t = useT().modes;
  return (
    <div className="mt-3 overflow-auto">
      <table className="w-full text-xs">
        <thead>
          <tr>
            <th className="p-2 text-left">{t.part}</th>
            {table.modes.map((m) => (
              <th key={m.id} className="p-2">
                {m.label}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {table.rows.map((row) => (
            <tr key={row.part}>
              <th className="p-2 text-left font-mono">{row.part}</th>
              {table.modes.map((m) => {
                const override = row.eclipse_cells[m.id];
                const state = override?.state ?? row.cells[m.id]?.state;
                return (
                  <td key={m.id} className="p-2 text-center">
                    <button
                      type="button"
                      className="rounded px-2 py-1 hover:bg-panel-2"
                      title={override ? t.eclipse : t.inherit}
                      onClick={() => onSelect(m.id, row.part)}
                    >
                      {state ?? "—"}
                      {!override && <span className="ml-1 text-ink-3">({t.inherit})</span>}
                    </button>
                  </td>
                );
              })}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
