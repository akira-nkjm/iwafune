import { useQuery } from "@tanstack/react-query";
import { q } from "@/api/queries";
import { useT } from "@/i18n";
import { Chip } from "@/kit/Chip";
import { Field } from "@/kit/Field";
import { Loading, SkeletonChips } from "@/kit/Loading";
import { NativeSelect } from "@/kit/NativeSelect";
import { Switch } from "@/kit/Switch";
import { TextInput } from "@/kit/TextInput";
import { withNote } from "@/lib/design";

export interface NewAttribute {
  id: string;
  label: string;
  dimension: string;
  units: string[];
  byState: boolean;
}

/** 新しい属性の定義（CreateAttribute）: 次元を選ぶと、その次元の単位から許可単位を選べる */
export function NewAttributeFields({
  value,
  onChange,
  idError,
}: {
  value: NewAttribute;
  onChange: (v: NewAttribute) => void;
  idError: string | null;
}) {
  const t = useT();
  const tn = t.components.newAttr;
  const all = useQuery(q.units(null));
  const units = useQuery({ ...q.units(value.dimension), enabled: value.dimension !== "" });
  const dims = all.data?.dimensions ?? [];
  // 表示単位の候補（service が次元ごとに選んだもの）を先に、その次元の残りの単位を後に並べる
  const preferred = dims.find((d) => d.name === value.dimension)?.display_units ?? [];
  const symbols = [
    ...preferred,
    ...(units.data?.units.map((u) => u.symbol) ?? []).filter((u) => !preferred.includes(u)),
  ];
  const toggle = (u: string) =>
    onChange({
      ...value,
      units: value.units.includes(u) ? value.units.filter((x) => x !== u) : [...value.units, u],
    });
  return (
    <div className="grid gap-3">
      <div className="grid grid-cols-2 gap-3">
        <Field label={tn.id} htmlFor="new-attr-id" error={idError}>
          <TextInput
            id="new-attr-id"
            mono
            value={value.id}
            invalid={!!idError}
            placeholder="max_dipole"
            onChange={(e) => onChange({ ...value, id: e.target.value })}
          />
        </Field>
        <Field label={tn.label} htmlFor="new-attr-label">
          <TextInput
            id="new-attr-label"
            value={value.label}
            placeholder={tn.labelPlaceholder}
            onChange={(e) => onChange({ ...value, label: e.target.value })}
          />
        </Field>
      </div>
      <Field
        label={tn.dimension}
        htmlFor="new-attr-dim"
        hint={all.isError ? tn.unitsError : tn.dimensionHint}
      >
        <NativeSelect
          id="new-attr-dim"
          value={value.dimension}
          onChange={(e) => onChange({ ...value, dimension: e.target.value, units: [] })}
          placeholder={tn.pickDimension}
          options={dims.map((d) => ({
            value: d.name,
            label: withNote(d.name, d.si_unit),
          }))}
        />
      </Field>
      {value.dimension && (
        <Field label={tn.units} hint={tn.unitsHint}>
          <div className="flex max-h-28 flex-wrap gap-1 overflow-y-auto">
            {symbols.map((u) => (
              <Chip key={u} selected={value.units.includes(u)} onSelect={() => toggle(u)}>
                {u}
              </Chip>
            ))}
            {units.isPending && (
              <Loading>
                <SkeletonChips count={3} />
              </Loading>
            )}
          </div>
        </Field>
      )}
      <Switch
        value={value.byState}
        onChange={(byState) => onChange({ ...value, byState })}
        className="self-start"
      >
        {tn.byState}
      </Switch>
    </div>
  );
}
