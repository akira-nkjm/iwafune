import { useSetText } from "@/api/ops";
import type { SettingView } from "@/api/types";
import { useT } from "@/i18n";
import { Empty } from "@/kit/feedback/Empty";
import { CommitInput } from "@/kit/forms/CommitInput";
import { Switch } from "@/kit/selection/Switch";
import { DataTable } from "@/kit/table/DataTable";
import { Td } from "@/kit/table/Td";
import { Th } from "@/kit/table/Th";
import { EditableQuantity } from "@/quantity/EditableQuantity";
import { quantityInputText } from "@/quantity/format";

/**
 * 設定（`mission/`・`analysis/`・`budget/` の値）の表。セルで直接編集し、打った文字列をそのまま送る
 * （値の型は `fune set` と同じく service が決める）。量は入力欄が /units/parse で検査する。
 */
export function SettingsTable({
  settings,
  empty,
  grouped = false,
}: {
  settings: SettingView[];
  empty?: string;
  /** 1 つのまとまり（API の `group`）だけの表で、枠の見出しで分かるとき: キーはファイル内のキー（`field`）だけにし、ファイルの列を出さない */
  grouped?: boolean;
}) {
  const t = useT();
  const setText = useSetText();
  if (settings.length === 0) return <Empty>{empty ?? t.project.noSettings}</Empty>;
  const set = (key: string, text: string) =>
    setText.mutate({ path: key, value: text, summary: t.quantity.setSummary(key, text) });
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
