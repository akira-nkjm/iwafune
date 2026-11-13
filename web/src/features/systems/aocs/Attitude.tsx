import { useEnumOptions } from "@/api/labels";
import { useSetText } from "@/api/ops";
import type { AocsAttitudeView, AocsLawField } from "@/api/types";
import { localized, useT } from "@/i18n";
import { CommitInput } from "@/kit/forms/CommitInput";
import { systemTone } from "@/lib/systems";
import { toneText } from "@/lib/tone";

/** 実効軸の小さな図と設定欄。全欄は設定ページと同じ SetText で編集する。 */
export function Attitude({ item }: { item: AocsAttitudeView }) {
  const dict = useT();
  const t = dict.systems.aocsModes;
  return (
    <section className="border-line border-t py-3">
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
      <div className="grid gap-2 sm:grid-cols-2">
        {item.fields.map((field) => (
          <LawField key={field.path} field={field} />
        ))}
      </div>
    </section>
  );
}

function LawField({ field }: { field: AocsLawField }) {
  const dict = useT();
  const t = dict.systems.aocsModes;
  const set = useSetText();
  const options = useEnumOptions("attitude_law");
  const commit = (value: string) =>
    set.mutate({ path: field.path, value, summary: dict.quantity.setSummary(field.path, value) });
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
          disabled={set.isPending}
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
          disabled={set.isPending}
          allowEmpty={false}
          onCommit={commit}
        />
      )}
      {set.error && (
        <p role="alert" className="text-xs text-neg-ink">
          {set.error.message}
        </p>
      )}
    </div>
  );
}
