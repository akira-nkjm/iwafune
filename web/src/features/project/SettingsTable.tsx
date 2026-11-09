import { useSetText } from "@/api/ops";
import type { SettingView } from "@/api/types";
import { useT } from "@/i18n";
import { DataTable } from "@/kit/DataTable";
import { Empty } from "@/kit/Empty";
import { Switch } from "@/kit/Switch";
import { Td } from "@/kit/Td";
import { Th } from "@/kit/Th";
import { EditableQuantity } from "@/quantity/EditableQuantity";
import { quantityInputText } from "@/quantity/format";

/**
 * 設定（`mission/`・`analysis/`・`budget/` の値）の表。セルで直接編集し、打った文字列をそのまま送る
 * （値の型は `fune set` と同じく service が決める）。量は入力欄が /units/parse で検査する。
 */
export function SettingsTable({
  settings,
  empty,
  prefix,
}: {
  settings: SettingView[];
  empty?: string;
  /** キーの表示から外す接頭辞（枠の見出しで分かるとき。例: "mission.orbit."） */
  prefix?: string;
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
          {!prefix && <Th>{t.project.file}</Th>}
        </>
      }
    >
      {settings.map((s) => {
        const v = s.value;
        return (
          <tr key={s.key} className="hover:bg-panel-2/60">
            <Td mono className="text-ink-2">
              {prefix && s.key.startsWith(prefix) ? s.key.slice(prefix.length) : s.key}
            </Td>
            <Td>
              {v.type === "scalar" ? (
                <EditableQuantity
                  display={s.display}
                  initial={quantityInputText(v.nominal)}
                  dimension={v.nominal.dimension}
                  label={s.key}
                  onCommit={(_q, t) => set(s.key, t)}
                />
              ) : v.type === "unset" ? (
                <EditableQuantity
                  display="—"
                  initial=""
                  dimension={null}
                  label={s.key}
                  muted
                  onCommit={(_q, t) => set(s.key, t)}
                />
              ) : v.type === "bool" ? (
                <Switch
                  aria-label={s.key}
                  on={v.value}
                  onChange={(next) => set(s.key, String(next))}
                />
              ) : v.type === "text" || v.type === "enum" ? (
                <input
                  aria-label={s.key}
                  defaultValue={v.value}
                  key={v.value}
                  onBlur={(e) => e.target.value !== v.value && set(s.key, e.target.value)}
                  className="h-7 w-full max-w-72 rounded-[8px] border border-line-strong px-2 text-[12.5px]"
                />
              ) : (
                <span className="font-mono text-[12px]">{s.display}</span>
              )}
            </Td>
            {!prefix && (
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
