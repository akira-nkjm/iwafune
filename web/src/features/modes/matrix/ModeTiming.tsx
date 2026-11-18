import { Clock } from "lucide-react";
import { useId } from "react";
import { useEnumLabel, useEnumOptions } from "@/api/labels";
import { useCommandText } from "@/api/ops";
import type { ModeHeader, ModeStartView, ModeWhen } from "@/api/types";
import { useT } from "@/i18n";
import { CommitInput } from "@/kit/forms/CommitInput";
import { NativeSelect } from "@/kit/forms/NativeSelect";
import { Popover, PopoverContent, PopoverTrigger } from "@/ui/popover";
import { isUnspecified, type TimingFields, timingCommand, timingFields } from "./timing";

/**
 * 列見出しのモードの最長継続時間の評価のタイミング（使う時間帯・開始の基準）。ふだんは小さな文字で、押すと
 * ポップオーバーで編集する。欄を変えるたびに `fune mode timing` と同じ引数で送る（読み方は service）。
 */
export function ModeTiming({ mode }: { mode: ModeHeader }) {
  const t = useT().modes.timing;
  const enumLabel = useEnumLabel();
  const fields = timingFields(mode);
  const text = isUnspecified(fields) ? "" : summary(fields, enumLabel, t.atEvent);
  return (
    <Popover>
      <PopoverTrigger asChild>
        <button
          type="button"
          title={t.title}
          aria-label={`${t.title}: ${text || t.unspecified}`}
          className={`mx-auto mt-0.5 flex items-center gap-1 rounded px-1.5 py-0.5 text-[11px] hover:bg-panel hover:text-ink ${text ? "text-ink-3" : "text-ink-4"}`}
        >
          <Clock size={11} aria-hidden="true" />
          <span>{text || t.unspecified}</span>
        </button>
      </PopoverTrigger>
      <PopoverContent align="start" className="w-72">
        <TimingForm mode={mode} fields={fields} />
      </PopoverContent>
    </Popover>
  );
}

/** 「日照中 · 食明け +5 min」のような 1 行の要約 */
function summary(
  f: TimingFields,
  enumLabel: (ty: string, value: string) => string,
  atEvent: (event: string, offset: string) => string,
): string {
  const parts: string[] = [];
  if (f.when && f.when !== "any") parts.push(enumLabel("mode_when", f.when));
  if (f.start) parts.push(atEvent(enumLabel("start_from", f.start), f.offset));
  return parts.join(" · ");
}

function TimingForm({ mode, fields }: { mode: ModeHeader; fields: TimingFields }) {
  const t = useT().modes.timing;
  const id = useId();
  const edit = useCommandText();
  const whenOptions = useEnumOptions("mode_when");
  const startOptions = useEnumOptions("start_from");
  const send = (next: TimingFields) =>
    edit.mutate({ command: timingCommand(mode.id, next), summary: t.summary(mode.id) });
  return (
    <div className="space-y-2">
      <p className="text-ink-3 text-xs">{t.hint}</p>
      <label htmlFor={`${id}-when`} className="block text-xs">
        {t.when}
        <NativeSelect
          id={`${id}-when`}
          className="mt-1 w-full"
          size="sm"
          options={whenOptions}
          value={fields.when ?? "any"}
          onChange={(e) => send({ ...fields, when: e.target.value as ModeWhen })}
        />
      </label>
      <label htmlFor={`${id}-start`} className="block text-xs">
        {t.start}
        <NativeSelect
          id={`${id}-start`}
          className="mt-1 w-full"
          size="sm"
          placeholder={t.noStart}
          options={startOptions}
          value={fields.start ?? ""}
          onChange={(e) =>
            send({
              ...fields,
              start: (e.target.value || null) as ModeStartView["from"] | null,
            })
          }
        />
      </label>
      {fields.start && (
        <label htmlFor={`${id}-offset`} className="block text-xs">
          {t.offset}
          <CommitInput
            id={`${id}-offset`}
            label={t.offset}
            value={fields.offset}
            placeholder={t.offsetHint}
            onCommit={(offset) => send({ ...fields, offset })}
          />
        </label>
      )}
    </div>
  );
}
