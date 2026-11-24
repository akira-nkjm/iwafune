import { choiceLabel } from "@/analysis/MissingPickDialog";
import { type FieldEdit, useSettingEdit } from "@/api/field-edit";
import type { SettingView } from "@/api/types";
import { useT } from "@/i18n";
import { Empty } from "@/kit/feedback/Empty";
import { CommitInput } from "@/kit/forms/CommitInput";
import { NativeSelect } from "@/kit/forms/NativeSelect";
import { Switch } from "@/kit/selection/Switch";
import { DataTable } from "@/kit/table/DataTable";
import { Td } from "@/kit/table/Td";
import { Th } from "@/kit/table/Th";
import { EditableQuantity } from "@/quantity/EditableQuantity";
import { quantityInputText } from "@/quantity/format";

/** 表の 1 行（設計の設定 `SettingView` と、フェーズの上書きの欄が同じ形で渡せる）。`key` は欄のパスで、書き込みにそのまま使う */
export type SettingRow = Pick<
  SettingView,
  "key" | "field" | "value" | "display" | "file" | "choices"
>;

/** API が選択肢を返した欄（軌道の `source`・`maneuver`）: 選んで入れる（未入力は空の選択肢。選び直すとその値を書く） */
function ChoiceCell({ row, onPick }: { row: SettingRow; onPick: (value: string) => void }) {
  const t = useT();
  const current = row.value.type === "text" || row.value.type === "enum" ? row.value.value : "";
  return (
    <NativeSelect
      aria-label={row.key}
      value={current}
      placeholder={t.project.unset}
      options={row.choices.map((c) => ({ value: c.value, label: choiceLabel(c) }))}
      onChange={(e) => e.target.value && onPick(e.target.value)}
      className="max-w-72"
    />
  );
}

/**
 * 設定（`conditions/`・`analysis/`・`budget/` の値）の表。セルで直接編集し、打った文字列をそのまま送る
 * （値の型は `fune set` と同じく service が決める）。量は入力欄が /units/parse で検査する。
 */
export function SettingsTable({
  settings,
  empty,
  grouped = false,
  edit,
}: {
  settings: SettingRow[];
  empty?: string;
  /** 1 つのまとまり（API の `group`）だけの表で、枠の見出しで分かるとき: キーはファイル内のキー（`field`）だけにし、ファイルの列を出さない */
  grouped?: boolean;
  /** 書き先（省略は設計の設定。フェーズの上書きなら `usePhaseFieldEdit`） */
  edit?: FieldEdit;
}) {
  const t = useT();
  const own = useSettingEdit();
  const set = (edit ?? own).set;
  if (settings.length === 0) return <Empty>{empty ?? t.project.noSettings}</Empty>;
  return (
    <DataTable
      head={
        <>
          <Th>{t.project.key}</Th>
          <Th>{t.project.value}</Th>
          {!grouped && <Th>{t.project.file}</Th>}
        </>
      }
    >
      {settings.map((s) => {
        const v = s.value;
        return (
          <tr key={s.key} className="hover:bg-panel-2/60">
            <Td mono className="text-ink-2">
              {grouped ? s.field : s.key}
            </Td>
            <Td>
              {v.type === "scalar" ? (
                <EditableQuantity
                  display={s.display ?? "—"}
                  initial={quantityInputText(v.nominal)}
                  dimension={v.nominal.dimension}
                  label={s.key}
                  onCommit={(t) => set(s.key, t)}
                />
              ) : s.choices.length > 0 ? (
                <ChoiceCell row={s} onPick={(value) => set(s.key, value)} />
              ) : v.type === "unset" ? (
                <EditableQuantity
                  display="—"
                  initial=""
                  dimension={null}
                  label={s.key}
                  muted
                  onCommit={(t) => set(s.key, t)}
                />
              ) : v.type === "bool" ? (
                <Switch
                  aria-label={s.key}
                  value={v.value}
                  onChange={(next) => set(s.key, String(next))}
                />
              ) : v.type === "text" || v.type === "enum" ? (
                <CommitInput
                  label={s.key}
                  value={v.value}
                  onCommit={(text) => set(s.key, text)}
                  className="max-w-72"
                />
              ) : (
                <span className="font-mono text-[12px]">{s.display ?? "—"}</span>
              )}
            </Td>
            {!grouped && (
              <Td mono className="text-ink-4">
                {s.file ?? "—"}
              </Td>
            )}
          </tr>
        );
      })}
    </DataTable>
  );
}
