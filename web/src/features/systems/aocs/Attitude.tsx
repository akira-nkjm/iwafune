import { Pencil } from "lucide-react";
import { type FieldEdit, useSettingEdit } from "@/api/field-edit";
import { useEnumLabel, useEnumOptions } from "@/api/labels";
import type { AocsAttitudeView, AocsLawField } from "@/api/types";
import { localized, useT } from "@/i18n";
import { CommitInput } from "@/kit/forms/CommitInput";
import { systemTone } from "@/lib/systems";
import { toneText } from "@/lib/tone";
import { Popover, PopoverContent, PopoverTrigger } from "@/ui/popover";

/**
 * カードに出す姿勢則の 1 行（姿勢則の表示名と主軸 → 目標）と「編集」。編集はポップオーバーで `Attitude` の欄を開く
 * （設定ページと同じ SetText の経路）。
 */
export function AttitudeLine({ item, mode }: { item: AocsAttitudeView; mode: string }) {
  const dict = useT();
  const t = dict.systems.aocsModes;
  const enumLabel = useEnumLabel();
  const primary = item.axes[0];
  return (
    <div className="flex items-center justify-between gap-2 text-xs">
      <p className="min-w-0 truncate">
        {item.law ? (
          <span className="font-medium">{enumLabel("attitude_law", item.law)}</span>
        ) : (
          <span className="text-ink-3">{t.noLaw}</span>
        )}
        {primary && (
          <span className="ml-1.5 text-ink-3">
            <span className="font-mono">{primary.body}</span> →{" "}
            {localized(primary.target, primary.target_en)}
          </span>
        )}
      </p>
      <Popover>
        <PopoverTrigger asChild>
          <button
            type="button"
            aria-label={t.editLawOf(mode)}
            className="inline-flex shrink-0 items-center gap-1 rounded-control px-1.5 py-0.5 text-ink-3 hover:bg-panel-2 hover:text-ink"
          >
            <Pencil size={12} aria-hidden />
            {t.edit}
          </button>
        </PopoverTrigger>
        <PopoverContent align="end" className="w-80 p-3" aria-label={t.editLawOf(mode)}>
          <Attitude item={item} />
        </PopoverContent>
      </Popover>
    </div>
  );
}

/**
 * 実効軸の小さな図と設定欄。全欄は設定ページと同じ SetText で編集する（`edit` を渡すと書き先を替えられる。
 * フェーズの上書きの姿勢則は `usePhaseFieldEdit`）。
 */
export function Attitude({ item, edit }: { item: AocsAttitudeView; edit?: FieldEdit }) {
  const dict = useT();
  const t = dict.systems.aocsModes;
  const own = useSettingEdit();
  const target = edit ?? own;
  return (
    <section>
      <h3 className="mb-2 font-medium text-sm">{dict.modes.attitudeLaw}</h3>
      {item.axes.length === 0 && <p className="text-xs text-ink-3">{t.noLaw}</p>}
      <div className="mb-3 space-y-2">
        {item.axes.map((axis, i) => (
          <div key={axis.body} className="flex items-center gap-2 text-xs">
            <span className="w-10 text-ink-3">{i === 0 ? t.primary : t.secondary}</span>
            <span className="font-mono">{axis.body}</span>
            <svg
              viewBox="0 0 64 24"
              width="64"
              height="24"
              role="img"
              aria-label={t.axisDiagram}
              className={toneText[systemTone("aocs")]}
            >
              <rect x="1" y="5" width="14" height="14" rx="2" fill="none" stroke="currentColor" />
              <path
                d="M 15 12 H 59 M 52 6 L 59 12 L 52 18"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.5"
              />
            </svg>
            <span>{localized(axis.target, axis.target_en)}</span>
          </div>
        ))}
      </div>
      <div className="grid gap-2">
        {item.fields.map((field) => (
          <LawField key={field.path} field={field} edit={target} />
        ))}
      </div>
    </section>
  );
}

function LawField({ field, edit }: { field: AocsLawField; edit: FieldEdit }) {
  const dict = useT();
  const t = dict.systems.aocsModes;
  const options = useEnumOptions("attitude_law");
  const commit = (value: string) => edit.set(field.path, value);
  return (
    <div className="grid gap-1 text-xs text-ink-3">
      <label htmlFor={field.path}>
        {t.fields[field.field as keyof typeof t.fields] ?? field.field}
      </label>
      {field.field === "law" ? (
        <select
          id={field.path}
          aria-label={field.path}
          value={field.text}
          disabled={edit.pending}
          onChange={(e) => commit(e.target.value)}
          className="rounded-control border border-line bg-panel px-2 py-1.5 text-ink"
        >
          <option value="" disabled>
            {t.noLaw}
          </option>
          {options.map((o) => (
            <option key={o.value} value={o.value}>
              {o.label}
            </option>
          ))}
        </select>
      ) : (
        <CommitInput
          id={field.path}
          label={field.path}
          value={field.text}
          disabled={edit.pending}
          allowEmpty={false}
          onCommit={commit}
        />
      )}
      {edit.error && (
        <p role="alert" className="text-xs text-neg-ink">
          {edit.error.message}
        </p>
      )}
    </div>
  );
}
