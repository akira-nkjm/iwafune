import { useId } from "react";
import { useT } from "@/i18n";
import { CommitInput } from "@/kit/forms/CommitInput";
import { Segmented } from "@/kit/selection/Segmented";
import type { CellText } from "../draft/draft";

/**
 * 1 セルの編集欄（ポップオーバーの中身）。上で基本の状態を選び、下でほかの状態ごとに周回のうちその状態で
 * 過ごす割合（0〜1）を打つ（空にするとその状態の割合を消す）。いちばん下のボタンでセルを未指定に戻す。
 * 割合は打った文字列のまま下書きに入れる（数として読むのは service。`fune mode set --duty` と同じ）。
 */
export function ModeCellForm({
  states,
  value,
  onChange,
}: {
  /** 部品の状態の語彙（基本の状態の選択肢と、時間割合の欄） */
  states: string[];
  /** 今のセル（未指定は null） */
  value: CellText | null;
  /** 状態・時間割合を変えたとき、未指定に戻したとき（null） */
  onChange: (value: CellText | null) => void;
}) {
  const t = useT().modes.cell;
  const id = useId();
  // 基本の状態を変えたら、その状態の時間割合は意味が無くなるので消す
  const setState = (state: string) => {
    const duty = Object.entries(value?.duty ?? {}).filter(([s]) => s !== state);
    onChange({ state, duty: Object.fromEntries(duty) });
  };
  const setDuty = (base: CellText, state: string, raw: string) => {
    const next = { ...base.duty };
    if (raw.trim() === "") delete next[state];
    else next[state] = raw;
    onChange({ state: base.state, duty: next });
  };
  return (
    <div className="space-y-3">
      <Segmented
        label={t.state}
        size="sm"
        options={states.map((s) => ({ value: s, label: s }))}
        value={value?.state ?? null}
        onChange={setState}
      />
      {value && (
        <div className="space-y-2">
          <p className="text-ink-2 text-xs">
            {t.dutyHelpBefore} <span className="font-mono">{value.state}</span>
            {t.dutyHelpAfter}
          </p>
          {states
            .filter((s) => s !== value.state)
            .map((s) => (
              <label
                key={`${value.state}-${s}`}
                htmlFor={`${id}-${s}`}
                className="flex items-center justify-between gap-2 text-xs"
              >
                <span className="font-mono">{s}</span>
                <CommitInput
                  id={`${id}-${s}`}
                  type="number"
                  min={0}
                  max={1}
                  step={0.01}
                  label={t.dutyFor(s)}
                  value={value.duty[s] ?? ""}
                  onCommit={(text) => setDuty(value, s, text)}
                  className="num w-20 text-right"
                />
              </label>
            ))}
          <button
            type="button"
            className="text-ink-3 text-xs underline"
            onClick={() => onChange(null)}
          >
            {t.clear}
          </button>
        </div>
      )}
    </div>
  );
}
