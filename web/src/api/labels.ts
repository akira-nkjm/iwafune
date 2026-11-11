import { useQuery } from "@tanstack/react-query";
import { localized } from "@/i18n";
import { withNote } from "@/lib/design";
import { q } from "./queries";
import type { VocabLabels } from "./types";

/**
 * 組み込みの列挙の表示名（`GET /defs` の `labels.enum`。正本は fune-model の labels.toml で、CLI も同じ表を引く）。
 * `ty` は `origin`・`evidence`・`maturity`・`severity` など。届くまでと表に無いときは値の綴りを返す。
 */
export function useEnumLabel(): (ty: string, value: string) => string {
  const { data } = useQuery(q.defs());
  return (ty, value) => enumLabel(data?.labels, ty, value);
}

/** 列挙の値の表示名（`useEnumLabel` の中身。表に無ければ値の綴り） */
function enumLabel(labels: VocabLabels | undefined, ty: string, value: string): string {
  const l = labels?.enum[ty]?.[value];
  return (l && localized(l.label, l.label_en)) || value;
}

/**
 * 列挙の値と表示名を `labels.enum_order` の順に（順が無い型は綴りの順）。届くまでは空。
 * 絞り込みのボタン（診断の重さなど）のように、表示名だけを出すところで使う。
 */
export function useEnumValues(ty: string): { value: string; label: string }[] {
  const { data } = useQuery(q.defs());
  const labels = data?.labels;
  const values = labels?.enum_order[ty] ?? Object.keys(labels?.enum[ty] ?? {}).sort();
  return values.map((value) => ({ value, label: enumLabel(labels, ty, value) }));
}

/**
 * 列挙の値の選択肢（並びは `useEnumValues`）。表示は「値（表示名）」（`withNote`。ID と表示名の選択肢と同じ形）。
 * 成熟度（`maturity`）などの選択肢は、値の一覧を手で書かずにこれで作る。届くまでは空。
 */
export function useEnumOptions(ty: string): { value: string; label: string }[] {
  return useEnumValues(ty).map(({ value, label }) => ({ value, label: withNote(value, label) }));
}
